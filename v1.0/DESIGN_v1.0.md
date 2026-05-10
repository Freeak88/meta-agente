# Control/Data Plane v1.0

## Contexto

Hoy todo corre en un solo proceso: generación, simulación, ejecución.
Queremos separar para escalar, hostear, y cobrar.

## Arquitectura

```text
┌─────────────────────────────────────────┐
│         CONTROL PLANE (API)             │
│  - Recibe requests REST/GRPC            │
│  - Genera AgentPackages (MetaAgent)     │
│  - Simula antes de aprobar              │
│  - Versiona y almacena                  │
│  - Orquesta deployment                  │
├─────────────────────────────────────────┤
│         DATA PLANE (Workers)            │
│  - Pull AgentPackages desde registry    │
│  - Ejecuta loops (secuencial/paralelo)  │
│  - Reporta métricas                     │
│  - Mantiene estado local                │
├─────────────────────────────────────────┤
│         REGISTRY (Storage)              │
│  - AgentPackages inmutables (hash)      │
│  - Snapshots de estado                  │
│  - Logs de ejecución                    │
│  - Métricas agregadas                   │
└─────────────────────────────────────────┘
```

## API REST v1.0

### Endpoints

| Método | Path | Descripción |
|--------|------|-------------|
| POST | `/agents` | Crear agente desde intent |
| GET | `/agents/:id` | Obtener agente por ID |
| GET | `/agents/:id/versions` | Listar versiones |
| POST | `/agents/:id/deploy` | Deploy versión específica |
| POST | `/agents/:id/execute` | Ejecutar ahora |
| GET | `/agents/:id/status` | Estado actual |
| POST | `/agents/:id/pause` | Pausar ejecución |
| POST | `/agents/:id/resume` | Reanudar |
| GET | `/agents/:id/logs` | Logs de ejecución |
| GET | `/agents/:id/metrics` | Métricas de salud |
| POST | `/simulate` | Simular sin deploy |

### Request/Response

```json
// POST /agents
{
  "intent": {
    "raw_input": "Cada hora, fijate si hay facturas nuevas...",
    "goal": "automatizar facturación AFIP",
    "domain": "finanzas"
  }
}

// Response 201
{
  "agent_id": "afip_invoice_processor",
  "version": "1.0.0",
  "hash": "sha256:abc123...",
  "opl": "...",
  "simulation_report": { },
  "status": "APPROVED"
}
```

## Workers

```rust
struct Worker {
    id: String,
    agent_package: AgentPackage,
    state: State,
    registry: CapabilityRegistry,
}

impl Worker {
    async fn run(&mut self) {
        // Pull package desde Control Plane
        // Ejecutar loop
        // Reportar métricas cada 30s
        // Guardar snapshot en pausa/fallo
    }
}
```

## Registry

```rust
struct AgentRegistry {
    packages: HashMap<String, Vec<VersionedPackage>>,
}

struct VersionedPackage {
    version: String,
    hash: String,
    opl_source: String,
    created_at: Timestamp,
    simulation_report: SimulationReport,
}
```

## Invariantes v1.0

1. **Control Plane nunca ejecuta**: solo genera y orquesta
2. **Data Plane nunca genera**: solo ejecuta lo que recibe
3. **Registry es source of truth**: packages inmutables, versionados
4. **Workers son stateless**: estado en snapshot, no en memoria local
5. **API es async**: requests no bloquean, retornan job_id

## No entra en v1.0

- Auth/OAuth (v1.1)
- Multi-tenant isolation (v1.1)
- Billing/metering (v1.2)
- Web UI dashboard (v1.2)
- WebSocket streaming (v1.1)
- Kubernetes operator (v1.3)

## Dependencias a agregar

```toml
axum = "0.7"             # HTTP framework
tokio-postgres = "0.7"  # Database (opcional, puede ser SQLite)
redis = "0.25"          # Cache/pubsub (opcional)
```

## Checkpoint de diseño

| Item | Estado |
|------|--------|
| Arquitectura 3 capas | ✅ |
| API REST definida | ✅ |
| Worker estructura | ✅ |
| Registry estructura | ✅ |
| Invariantes | ✅ |
| Scope negativo | ✅ |
