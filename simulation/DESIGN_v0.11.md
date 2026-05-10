# Simulation Layer - Diseño Técnico v0.11

## Contexto

MVP v0.10 ejecuta en producción real (HTTP, mocks). Antes de deployar
un agente nuevo, queremos saber qué haría sin tocar sistemas reales.

## Problema específico

- ¿Se rompe si el servicio tarda 10s?
- ¿Qué pasa si 2 de 5 steps fallan?
- ¿Cuánto costaría ejecutar esto 100 veces?
- ¿Dónde están los riesgos ocultos?

## Solución: Simulation Engine

Capa que corre el mismo AgentPackage contra mocks controlados,
generando un reporte de predicción.

## 4 Modos de simulación

### 1. Optimistic

- Todos los mocks retornan success
- Usado para: validar flujo base, métricas ideales
- Output: tiempo mínimo, costo mínimo, 0 fallos

### 2. Adversarial

- Todos los mocks retornan failure donde sea posible
- Usado para: validar resiliencia, coverage de fallbacks
- Output: qué falla primero, si el agente se recupera

### 3. Stochastic

- Probabilístico: success 70%, timeout 15%, failure 10%, anomaly 5%
- Usado para: validar comportamiento estadístico
- Output: distribución de resultados, confianza

### 4. Replay

- Reproduce un execution_log exacto
- Usado para: reproducir bugs, validar fixes
- Output: comparación expected vs actual

## Arquitectura

```text
┌─────────────────────────────────────────┐
│         SIMULATION ENGINE               │
│  - Recibe AgentPackage + ScenarioConfig │
│  - Construye MockRegistry determinista  │
│  - Corre loop completo (sin HTTP real)  │
├─────────────────────────────────────────┤
│         MOCK REGISTRY                   │
│  - Cada capability tiene respuesta fija │
│  - Configurable por modo + scenario     │
│  - No side effects, no red, no disco    │
├─────────────────────────────────────────┤
│         REPORT GENERATOR                │
│  - Recoge resultados de simulación      │
│  - Compara con expected                 │
│  - Genera recomendaciones               │
└─────────────────────────────────────────┘
```

## Estructuras nuevas

```rust
struct SimulationConfig {
    pub mode: SimulationMode,
    pub seed: Option<u64>,
    pub scenario: ScenarioConfig,
    pub max_runs: u32,
}

enum SimulationMode {
    Optimistic,
    Adversarial,
    Stochastic,
    Replay(ExecutionLog),
}

struct ScenarioConfig {
    pub responses: HashMap<String, MockResponse>,
    pub latency_ms: HashMap<String, u64>,
    pub failure_rate: f64,
}

struct MockResponse {
    pub success: bool,
    pub data: Option<Value>,
    pub error: Option<String>,
}

struct SimulationReport {
    pub agent_id: String,
    pub mode: SimulationMode,
    pub runs: Vec<SimulationRun>,
    pub summary: SimulationSummary,
}

struct SimulationRun {
    pub run_id: u32,
    pub results: HashMap<String, StepResult>,
    pub execution_time_ms: u64,
    pub status: SimulationStatus,
}

struct SimulationSummary {
    pub pass_rate: f64,
    pub avg_execution_time_ms: u64,
    pub predicted_failures: Vec<PredictedFailure>,
    pub risk_spikes: Vec<RiskSpike>,
    pub recommendations: Vec<String>,
    pub validation_status: ValidationStatus,
}

struct PredictedFailure {
    pub step_id: String,
    pub failure_type: String,
    pub probability: f64,
    pub mitigation: String,
}

struct RiskSpike {
    pub step_id: String,
    pub risk_score: f64,
    pub reason: String,
}
```

## Flujo de simulación

```text
1. Recibir AgentPackage + SimulationConfig
2. Construir MockRegistry desde ScenarioConfig
3. Para cada run (1 en optimistic/adversarial/replay, N en stochastic):
   a. Crear State fresh
   b. Ejecutar loop con MockRegistry
   c. Capturar resultados
4. Generar SimulationReport
5. Comparar con umbrales de aceptación
6. Retornar APPROVED / REJECTED / NEEDS_REVIEW
```

## MockRegistry

```rust
struct MockRegistry {
    responses: HashMap<String, MockResponse>,
    rng: Option<StdRng>,
}

impl MockRegistry {
    fn resolve(&self, capability_id: &str) -> ExecutionResult {
        match self.responses.get(capability_id) {
            Some(resp) => {
                if resp.success {
                    ExecutionResult::Success(resp.data.clone().unwrap_or(Value::Null))
                } else {
                    ExecutionResult::Failure(resp.error.clone().unwrap_or("mock_failure".to_string()))
                }
            }
            None => ExecutionResult::Failure(format!("mock_not_found: {}", capability_id)),
        }
    }

    fn resolve_stochastic(&mut self, capability_id: &str, failure_rate: f64) -> ExecutionResult {
        let roll = self.rng.as_mut().unwrap().gen::<f64>();
        if roll < failure_rate {
            ExecutionResult::Failure("stochastic_failure".to_string())
        } else {
            self.resolve(capability_id)
        }
    }
}
```

## Invariantes de simulación

1. **Simulation no toca State real**: siempre State fresh
2. **Simulation no toca disco**: snapshots en memoria, no persistidos
3. **Simulation no toca red**: MockRegistry, no HTTP real
4. **Simulation es determinista con seed**: mismo seed → mismo resultado
5. **Report es inmutable**: generado una vez, no modificado post-ejecución

## Tests planificados

| # | Test | Descripción |
|---|------|-------------|
| 1 | `sim_optimistic_pass` | Todos ok, report APPROVED |
| 2 | `sim_adversarial_fail` | Todos fallan, report REJECTED |
| 3 | `sim_stochastic_distribution` | 100 runs, pass_rate ~70% |
| 4 | `sim_replay_exact` | Replay de log, mismos resultados |
| 5 | `sim_predicted_failure` | Adversarial detecta step débil |
| 6 | `sim_risk_spike` | Step con alta latencia flagged |
| 7 | `sim_deterministic_seed` | Mismo seed, mismo resultado |
| 8 | `sim_no_side_effects` | Post-sim, State real no cambia |

## No entra en v0.11

- Visualización de reporte (gráficos, UI)
- Comparación A/B entre versiones
- Simulation en paralelo (usa loop secuencial, no ParallelGroup)
- Integración con meta-agente generativo (eso es v0.13)

## Dependencias a agregar

```toml
rand = { version = "0.8", features = ["std_rng"] }
```

## Migración desde v0.10

```rust
// Antes: solo runtime real
let loop = ExecutionLoop::new(package, &registry);
loop.run(&mut state).await;

// Después: runtime o simulación
if config.simulation_mode.is_some() {
    let report = SimulationEngine::run(package, config).await;
    println!("Simulation: {:?}", report.validation_status);
} else {
    let loop = ExecutionLoop::new(package, &registry);
    loop.run(&mut state).await;
}
```

## Checkpoint de diseño

| Item | Estado |
|------|--------|
| 4 modos definidos | ✅ |
| Arquitectura | ✅ |
| Estructuras nuevas | ✅ |
| MockRegistry | ✅ |
| Invariantes | ✅ |
| Tests planificados | ✅ |
| Scope negativo | ✅ |
| Dependencias | ✅ |
