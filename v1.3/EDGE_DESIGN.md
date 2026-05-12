# Edge Computing v1.3 — Diseño Técnico

## Contexto

Hoy los workers corren en un servidor central. Queremos:
- Correr agents en el edge (más cerca del usuario/dato)
- WASM para sandbox portable
- Multi-region para baja latencia
- Sync con cloud central cuando hay conectividad

## Arquitectura Edge

```
┌─────────────────────────────────────────┐
│           CLOUD CENTRAL                 │
│  - Control Plane                        │
│  - Registry                             │
│  - Billing                              │
│  - Orchestrator (master)                │
├─────────────────────────────────────────┤
│           EDGE NODES                    │
│  - WASM Runtime (wasmtime/wasmer)       │
│  - Agent Packages compilados a WASM     │
│  - Local state (IndexedDB, SQLite)      │
│  - Sync cuando online                   │
├─────────────────────────────────────────┤
│           BROWSER / IoT                 │
│  - WASM en browser (WebAssembly)        │
│  - Service Worker para background       │
│  - LocalStorage / IndexedDB             │
│  - WebRTC para P2P sync                 │
└─────────────────────────────────────────┘
```

## WASM Runtime

```rust
pub struct WasmRuntime {
    engine: wasmtime::Engine,
    module: wasmtime::Module,
    store: wasmtime::Store<WasmState>,
}

pub struct WasmState {
    memory: Vec<u8>,
    capabilities: HashMap<String, Box<dyn CapabilityAdapter>>,
}

impl WasmRuntime {
    pub fn compile(opl: &str) -> Result<Vec<u8>, CompileError> {
        // Transpilar OPL → Rust → WASM
        // Por ahora: compilar a JSON, interpretar en WASM
    }

    pub fn instantiate(wasm: &[u8]) -> Result<Self, InstantiateError> {
        // Cargar módulo WASM
        // Exportar funciones: execute_step, get_state, set_state
    }

    pub fn execute(&mut self, step_id: &str, input: &str) -> Result<String, ExecutionError> {
        // Llamar función exportada del WASM
        // Retornar resultado serializado
    }
}
```

## Edge Node

```rust
pub struct EdgeNode {
    node_id: String,
    region: String,
    runtime: WasmRuntime,
    local_state: EdgeState,
    sync_queue: Vec<SyncEvent>,
}

pub struct EdgeState {
    agents: HashMap<String, AgentPackage>,
    snapshots: HashMap<String, State>,
    pending_executions: Vec<ExecutionRecord>,
}

impl EdgeNode {
    pub async fn sync_with_cloud(&mut self) -> Result<(), SyncError> {
        // Enviar pending_executions a cloud
        // Recibir updates de agent packages
        // Mergear conflictos (cloud wins por default)
    }

    pub async fn execute_offline(&mut self, agent_id: &str) -> Result<ExecutionResult, OfflineError> {
        // Ejecutar con local_state
        // Guardar en pending_executions para sync posterior
        // No bloquear si no hay conectividad
    }
}
```

## Multi-region Orchestrator

```rust
pub struct MultiRegionOrchestrator {
    regions: HashMap<String, RegionConfig>,
    latency_map: HashMap<(String, String), u64>, // (user_region, edge_region) → latency_ms
}

pub struct RegionConfig {
    region_id: String,
    edge_nodes: Vec<String>,
    capabilities: Vec<String>, // qué capabilities disponibles localmente
    cost_multiplier: f64,     // más caro edge que cloud
}

impl MultiRegionOrchestrator {
    pub fn select_region(&self, user_region: &str, required_caps: &[String]) -> Option<String> {
        // Elegir edge node más cercano que tenga todas las capabilities
        // Fallback a cloud central si no hay match
    }

    pub fn deploy_to_region(&self, agent_id: &str, region: &str) -> Result<Deployment, OrchestratorError> {
        // Enviar WASM a edge node
        // Registrar en orchestrator central
    }
}
```

## Browser Runtime

```rust
pub struct BrowserAgent {
    service_worker: ServiceWorkerRegistration,
    wasm_module: WebAssembly::Module,
    local_db: IndexedDB,
}

impl BrowserAgent {
    pub async fn install(opl: &str) -> Result<Self, InstallError> {
        // Registrar service worker
        // Cargar WASM
        // Abrir IndexedDB
    }

    pub async fn run_in_background(&self) -> Result<(), BackgroundError> {
        // Service Worker ejecuta agente
        // Wake up periódicamente (periodic background sync)
        // Notificar al usuario si HITL requerido
    }
}
```

## Sync Strategy

| Escenario | Comportamiento |
|-----------|---------------|
| Online | Sync inmediato, cloud es source of truth |
| Offline | Ejecutar local, queue para sync |
| Reconexión | Flush queue, resolver conflictos |
| Conflict | Cloud wins, edge notifica al usuario |
| Capacidad no disponible edge | Fallback a cloud |

## Invariantes Edge

1. **WASM es sandbox**: no accede a filesystem, red solo por capabilities
2. **State local es ephemeral**: cloud tiene la verdad
3. **Offline execution no bloquea**: siempre hay fallback
4. **Sync es eventual**: no garantía de tiempo real
5. **Edge node no tiene billing**: cobro en cloud post-sync

## No entra en v1.3

- P2P entre edge nodes (v1.4)
- CRDT para state sin conflictos (v1.4)
- GPU en edge (v1.5)
- Blockchain/verifiable compute (v1.5)

## Dependencias nuevas

```toml
wasmtime = "19"           # WASM runtime
wasmer = "4"              # Alternativa WASM
web-sys = "0.3"           # Browser APIs (opcional, solo si compilamos a WASM)
```

## Tests planificados

| # | Test | Descripción |
|---|------|-------------|
| 1 | `wasm_compile` | OPL → WASM compila |
| 2 | `wasm_execute` | WASM ejecuta step simple |
| 3 | `edge_node_sync` | Offline → online, sync correcto |
| 4 | `edge_offline` | Ejecuta sin red, guarda en queue |
| 5 | `multi_region_select` | Elige región por latencia |
| 6 | `browser_install` | Instala agente en browser (mock) |
| 7 | `wasm_sandbox` | WASM no puede acceder filesystem |

## Checkpoint de diseño

| Item | Estado |
|------|--------|
| WASM runtime | ✅ |
| Edge node | ✅ |
| Multi-region orchestrator | ✅ |
| Browser runtime | ✅ |
| Sync strategy | ✅ |
| Invariantes | ✅ |
| Tests planificados | ✅ |
| Scope negativo | ✅ |
