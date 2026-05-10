#![allow(dead_code)]

use axum::{
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use sha2::{Digest, Sha256};
use std::net::SocketAddr;

use crate::api::{ApiError, CreateAgentRequest, CreateAgentResponse};
use crate::meta_agent::MetaAgent;

pub fn router() -> Router {
    Router::new()
        .route("/agents", post(create_agent))
        .route("/health", get(health_check))
}

pub async fn start_server() {
    let app = router();
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Server running on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("server should bind to 127.0.0.1:3000");
    axum::serve(listener, app).await.expect("server should run");
}

pub async fn create_agent(
    Json(payload): Json<CreateAgentRequest>,
) -> Result<(StatusCode, Json<CreateAgentResponse>), (StatusCode, Json<ApiError>)> {
    let result = MetaAgent::generate(payload.intent).await.map_err(|error| {
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ApiError {
                code: 422,
                message: error.clone(),
                errors: vec![error],
            }),
        )
    })?;

    let response = CreateAgentResponse {
        agent_id: result.package.id.clone(),
        version: "1.0.0".to_string(),
        hash: opl_hash(&result.opl.source),
        opl: result.opl.source,
        simulation_report: result.simulation,
        status: result.status,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn health_check() -> &'static str {
    "ok"
}

fn opl_hash(opl_source: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(opl_source.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}
