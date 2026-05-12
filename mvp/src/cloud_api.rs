#![allow(dead_code)]

use async_trait::async_trait;
use axum::{
    body::Body,
    extract::{FromRequestParts, Path, State},
    http::{request::Parts, Request, StatusCode},
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::auth::{AuthResponse, AuthService, Claims, LoginRequest, RegisterRequest};
use crate::billing::{Account as BillingAccount, BillingEngine, BillingError, Plan as BillingPlan};
use crate::metrics::{ExecutionStatus, MetricsCollector};
use crate::orchestrator::Orchestrator;
use crate::simulation::{SimulationConfig, SimulationEngine, SimulationMode, ValidationStatus};

pub struct CloudState {
    pub auth: Mutex<AuthService>,
    pub orchestrator: Mutex<Orchestrator>,
    pub metrics: Mutex<MetricsCollector>,
}

impl CloudState {
    pub fn new(jwt_secret: impl Into<String>) -> Self {
        Self {
            auth: Mutex::new(AuthService::new(jwt_secret)),
            orchestrator: Mutex::new(Orchestrator::new()),
            metrics: Mutex::new(MetricsCollector::new()),
        }
    }
}

pub fn cloud_router(state: Arc<CloudState>) -> Router {
    Router::new()
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/account", get(get_account))
        .route("/agents", post(create_cloud_agent))
        .route("/agents/:id/deploy", post(deploy_agent))
        .route("/agents/:id/scale", post(scale_agent))
        .route("/agents/:id/health", get(agent_health))
        .route("/dashboard", get(dashboard))
        .with_state(state)
}

#[async_trait]
impl FromRequestParts<Arc<CloudState>> for Claims {
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<CloudState>,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or(StatusCode::UNAUTHORIZED)?;
        let token = header
            .strip_prefix("Bearer ")
            .ok_or(StatusCode::UNAUTHORIZED)?;
        let auth = state.auth.lock().await;

        auth.validate_claims(token)
            .map_err(|_| StatusCode::UNAUTHORIZED)
    }
}

async fn register(
    State(state): State<Arc<CloudState>>,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, String)> {
    let mut auth = state.auth.lock().await;
    auth.register(payload)
        .map(Json)
        .map_err(|error| (StatusCode::CONFLICT, format!("{:?}", error)))
}

async fn login(
    State(state): State<Arc<CloudState>>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, String)> {
    let auth = state.auth.lock().await;
    auth.login(payload)
        .map(Json)
        .map_err(|error| (StatusCode::UNAUTHORIZED, format!("{:?}", error)))
}

async fn get_account(
    State(state): State<Arc<CloudState>>,
    claims: Claims,
) -> Result<Json<crate::auth::Account>, StatusCode> {
    let auth = state.auth.lock().await;
    auth.get_account(&claims.sub)
        .map(Json)
        .map_err(|_| StatusCode::NOT_FOUND)
}

async fn create_cloud_agent(
    State(state): State<Arc<CloudState>>,
    claims: Claims,
    Json(intent): Json<crate::meta_agent::HumanIntent>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mut account = {
        let auth = state.auth.lock().await;
        auth.get_account(&claims.sub)
            .map_err(|error| (StatusCode::UNAUTHORIZED, format!("{:?}", error)))?
    };
    let billing_account = billing_account_from_auth(&account);

    BillingEngine::can_create_agent(&billing_account, 0).map_err(payment_error)?;
    BillingEngine::can_execute(&billing_account, None).map_err(payment_error)?;

    let result = crate::meta_agent::MetaAgent::generate(intent)
        .await
        .map_err(|error| (StatusCode::UNPROCESSABLE_ENTITY, error))?;
    let sim_config = SimulationConfig {
        agent_id: result.package.id.clone(),
        mode: SimulationMode::Optimistic,
        seed: None,
        max_runs: Some(1),
        failure_rate: None,
        execution_log_id: None,
    };
    let sim_report = SimulationEngine::run(&result.package, &sim_config).await;

    if sim_report.summary.validation_status == ValidationStatus::Rejected {
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            "Agent simulation failed. Check capabilities and retry.".to_string(),
        ));
    }

    let sim_cost_units = 1;
    if !matches!(account.plan, crate::auth::Plan::Free) {
        account.credits = account.credits.saturating_sub(sim_cost_units);
    }

    {
        let mut metrics = state.metrics.lock().await;
        let execution_time = sim_report
            .runs
            .first()
            .map(|run| run.execution_time_ms)
            .unwrap_or(0);
        metrics.record_execution(
            &result.package.id,
            "simulation",
            execution_time,
            result.package.steps.len() as u32,
            result.package.steps.len() as u32,
            0,
            ExecutionStatus::Completed,
            sim_cost_units as f64 / 100.0,
        );
    }

    {
        let mut auth = state.auth.lock().await;
        auth.update_account(account)
            .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, format!("{:?}", error)))?;
    }

    Ok(Json(serde_json::json!({
        "agent_id": result.package.id,
        "version": "1.0.0",
        "hash": format!("sha256:{:x}", md5::compute(&result.opl.source)),
        "opl": result.opl.source,
        "simulation_report": sim_report,
        "status": "APPROVED",
        "credits_used": sim_cost_units,
        "created_at": chrono::Utc::now().to_rfc3339()
    })))
}

async fn deploy_agent(
    State(state): State<Arc<CloudState>>,
    _claims: Claims,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mut orchestrator = state.orchestrator.lock().await;
    let deployment = orchestrator
        .deploy(&id, "1.0.0")
        .await
        .map_err(|error| (StatusCode::CONFLICT, format!("{:?}", error)))?;

    Ok(Json(serde_json::json!({
        "agent_id": id,
        "deployment": deployment,
        "status": "deployed"
    })))
}

async fn scale_agent(
    State(state): State<Arc<CloudState>>,
    _claims: Claims,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let target = body
        .get("workers")
        .and_then(|value| value.as_u64())
        .ok_or((StatusCode::BAD_REQUEST, "workers required".to_string()))? as u32;
    let mut orchestrator = state.orchestrator.lock().await;
    let deployment = orchestrator
        .scale(&id, target)
        .await
        .map_err(|error| (StatusCode::CONFLICT, format!("{:?}", error)))?;

    Ok(Json(serde_json::json!({
        "agent_id": id,
        "deployment": deployment,
        "status": "scaled"
    })))
}

async fn agent_health(
    State(state): State<Arc<CloudState>>,
    _claims: Claims,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let metrics = state.metrics.lock().await;
    let health = metrics.agent_health(&id).ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(serde_json::json!({
        "agent_id": id,
        "health": health,
        "status": format!("{:?}", health.status)
    })))
}

async fn dashboard(
    State(state): State<Arc<CloudState>>,
    claims: Claims,
) -> Json<serde_json::Value> {
    let metrics = state.metrics.lock().await;

    Json(serde_json::json!({
        "account_id": claims.sub,
        "system": metrics.system_metrics()
    }))
}

fn billing_account_from_auth(account: &crate::auth::Account) -> BillingAccount {
    let plan = match account.plan {
        crate::auth::Plan::Free => BillingPlan::Free {
            max_executions_per_month: 100,
            max_storage_mb: 100,
            max_agents: account.max_agents,
        },
        crate::auth::Plan::Pro => BillingPlan::Pro {
            max_executions_per_month: 10_000,
            max_storage_gb: 10,
            max_agents: account.max_agents,
            priority: true,
        },
        crate::auth::Plan::Enterprise => BillingPlan::Enterprise {
            custom: true,
            sla: "99.99%".to_string(),
        },
    };

    BillingAccount {
        id: account.id.clone(),
        plan,
        credits: account.credits as f64,
        used_executions: 0,
        used_storage_mb: 0,
    }
}

fn payment_error(error: BillingError) -> (StatusCode, String) {
    (
        StatusCode::PAYMENT_REQUIRED,
        format!("billing_blocked: {:?}", error),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use tower::ServiceExt;

    fn test_intent() -> crate::meta_agent::HumanIntent {
        crate::meta_agent::HumanIntent {
            raw_input: "Cada hora, fijate si hay facturas nuevas en AFIP".to_string(),
            goal: "automatizar facturación AFIP".to_string(),
            domain: Some("finanzas".to_string()),
            constraints: vec!["legal".to_string()],
            environment: Some("browser".to_string()),
            urgency: Some("alta".to_string()),
        }
    }

    async fn register_and_token(app: Router) -> (Router, String) {
        let request = RegisterRequest {
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
        };
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/register")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let auth: AuthResponse = serde_json::from_slice(&body).unwrap();
        (app, auth.token)
    }

    #[tokio::test]
    async fn account_requires_authorization() {
        let app = cloud_router(Arc::new(CloudState::new("test-secret")));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/account")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn register_login_and_get_account() {
        let app = cloud_router(Arc::new(CloudState::new("test-secret")));
        let (app, token) = register_and_token(app).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/account")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn login_returns_bearer_token() {
        let app = cloud_router(Arc::new(CloudState::new("test-secret")));
        let (app, _) = register_and_token(app).await;
        let request = LoginRequest {
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let auth: AuthResponse = serde_json::from_slice(&body).unwrap();
        assert!(!auth.token.is_empty());
        assert_eq!(auth.account.email, "test@example.com");
    }

    #[tokio::test]
    async fn create_agent_records_metrics() {
        let state = Arc::new(CloudState::new("test-secret"));
        let app = cloud_router(state.clone());
        let (app, token) = register_and_token(app).await;

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/agents")
                    .header("authorization", format!("Bearer {}", token))
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&test_intent()).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let metrics = state.metrics.lock().await;
        let health = metrics
            .agent_health("automatizar_facturacion_afip_agent")
            .unwrap();
        assert_eq!(health.total_executions, 1);
    }

    #[tokio::test]
    async fn deploy_and_scale_agent() {
        let state = Arc::new(CloudState::new("test-secret"));
        let app = cloud_router(state);
        let (app, token) = register_and_token(app).await;

        let deploy = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/agents/agent_1/deploy")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(deploy.status(), StatusCode::OK);

        let scale = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/agents/agent_1/scale")
                    .header("authorization", format!("Bearer {}", token))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"workers":2}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(scale.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn dashboard_returns_system_metrics() {
        let app = cloud_router(Arc::new(CloudState::new("test-secret")));
        let (app, token) = register_and_token(app).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/dashboard")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
