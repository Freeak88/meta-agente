# Cloud v1.2 - Diseño Técnico

## Contexto

Hoy Meta-Agente corre en una maquina local. Queremos:

- Hostear agentes en cloud
- Escalar workers automaticamente
- Ver metricas en dashboard
- Cobrar por uso

## Arquitectura Cloud

```
┌─────────────────────────────────────────┐
│           CONTROL PLANE (API)           │
│  - REST API publica (axum)              │
│  - Auth (API keys, JWT)                 │
│  - Billing (credits, usage)             │
│  - Dashboard (metricas, logs)           │
├─────────────────────────────────────────┤
│           ORCHESTRATOR                  │
│  - Kubernetes / Docker Swarm            │
│  - Auto-scaling de workers              │
│  - Health checks, rolling updates       │
│  - Resource limits (CPU, memory)        │
├─────────────────────────────────────────┤
│           WORKER POOL                   │
│  - Workers stateless                    │
│  - Pull packages desde registry         │
│  - Reportan metricas cada 30s           │
│  - Snapshot a storage central           │
├─────────────────────────────────────────┤
│           REGISTRY + STORAGE            │
│  - AgentPackages inmutables (S3/MinIO)  │
│  - Snapshots (PostgreSQL/SQLite)        │
│  - Logs (ClickHouse/InfluxDB)           │
│  - Metricas (Prometheus + Grafana)      │
└─────────────────────────────────────────┘
```

## Componentes Nuevos

### 1. Auth

```rust
pub struct AuthMiddleware;

impl AuthMiddleware {
    fn validate_api_key(key: &str) -> Result<Account, AuthError>;
    fn validate_jwt(token: &str) -> Result<Account, AuthError>;
}

pub struct Account {
    pub id: String,
    pub plan: Plan,
    pub credits: u64,
    pub max_agents: u32,
    pub max_workers: u32,
}
```

### 2. Billing

```rust
pub struct BillingEngine;

impl BillingEngine {
    fn charge_execution(account: &Account, execution_time_ms: u64, steps: u32) -> Result<(), BillingError>;
    fn charge_storage(account: &Account, bytes: u64) -> Result<(), BillingError>;
    fn charge_simulation(account: &Account, runs: u32) -> Result<(), BillingError>;
}

pub enum Plan {
    Free { executions: u32, storage_mb: u32 },
    Pro { executions: u32, storage_gb: u32, priority: bool },
    Enterprise { unlimited: bool, sla: String },
}
```

### 3. Orchestrator

```rust
pub struct Orchestrator;

impl Orchestrator {
    async fn deploy_agent(agent_id: &str, version: &str) -> Result<WorkerId, OrchestratorError>;
    async fn scale_workers(agent_id: &str, target: u32) -> Result<(), OrchestratorError>;
    async fn health_check() -> Vec<WorkerHealth>;
    async fn rolling_update(agent_id: &str, new_version: &str) -> Result<(), OrchestratorError>;
}
```

### 4. Metrics

```rust
pub struct MetricsCollector;

impl MetricsCollector {
    fn record_execution(agent_id: &str, duration_ms: u64, status: ExecutionStatus);
    fn record_failure(agent_id: &str, step_id: &str, error_type: &str);
    fn record_cost(agent_id: &str, credits: f64);
}

pub struct DashboardQuery;

impl DashboardQuery {
    fn agent_health(agent_id: &str) -> HealthScore;
    fn account_usage(account_id: &str) -> UsageReport;
    fn system_load() -> SystemMetrics;
}
```

## API v1.2

| Metodo | Path | Auth | Descripcion |
|--------|------|------|-------------|
| POST | `/auth/register` | No | Crear cuenta |
| POST | `/auth/login` | No | Obtener JWT |
| GET | `/account` | JWT | Ver cuenta, creditos, plan |
| POST | `/account/upgrade` | JWT | Cambiar plan |
| GET | `/dashboard` | JWT | Metricas de mis agentes |
| GET | `/dashboard/:agent_id` | JWT | Metricas de un agente |
| POST | `/agents/:id/pause` | API key | Pausar agente cloud |
| POST | `/agents/:id/resume` | API key | Reanudar agente cloud |
| GET | `/agents/:id/logs` | JWT | Logs en tiempo real |
| GET | `/agents/:id/metrics` | JWT | Metricas Prometheus |
| POST | `/webhooks` | JWT | Configurar webhook de eventos |

## Storage

| Dato | Tecnologia | Razon |
|------|------------|-------|
| AgentPackages | S3 / MinIO | Inmutable, versionado, barato |
| Snapshots | PostgreSQL | Relacional, ACID, backup facil |
| Logs | ClickHouse / InfluxDB | Time-series, agregaciones rapidas |
| Metricas | Prometheus + Grafana | Estandar de industria |
| Cache | Redis | Sesiones, rate limits, locks |

## Invariantes Cloud

1. **Worker es stateless**: snapshot va a storage, no a disco local
2. **AgentPackage es inmutable**: nunca se modifica post-deploy
3. **Auth en todo endpoint**: sin excepciones, incluso health checks internos
4. **Billing determinista**: mismo execution -> mismo costo, siempre
5. **Rollback automatico**: si deploy falla, vuelve a version anterior

## No Entra En v1.2

- Multi-region (v1.3)
- Edge computing / WASM workers (v1.3)
- Custom domains / white-label (v1.4)
- Marketplace de skills (v1.4)
- SOC2 / ISO27001 (v1.5)

## Dependencias Nuevas

```toml
tokio-postgres = "0.7"
redis = "0.25"
prometheus = "0.13"
jsonwebtoken = "9"
argon2 = "0.5"
```

## Tests Planificados

| # | Test | Descripcion |
|---|------|-------------|
| 1 | `auth_register_login` | Crear cuenta, login, JWT valido |
| 2 | `auth_invalid_key` | API key invalida -> 401 |
| 3 | `billing_free_plan` | Exceder limites free -> 402 Payment Required |
| 4 | `orchestrator_deploy` | Deploy agente, worker arranca |
| 5 | `orchestrator_scale` | Scale up/down workers |
| 6 | `metrics_execution` | Ejecutar agente, metrica registrada |
| 7 | `snapshot_cloud` | Snapshot guardado en PostgreSQL |
| 8 | `rollback_deploy` | Deploy falla, rollback automatico |

## Checkpoint de Diseño

| Item | Estado |
|------|--------|
| Arquitectura 4 capas | OK |
| Auth | OK |
| Billing | OK |
| Orchestrator | OK |
| Metrics | OK |
| Storage | OK |
| API nuevos endpoints | OK |
| Invariantes | OK |
| Tests planificados | OK |
| Scope negativo | OK |
