# Issues iniciales

## #1 [DESIGN] Parallel Steps - Arquitectura de ejecucion concurrente

- Labels: `design`, `parallel`, `p0-critical`
- Milestone: `v0.10 Parallel Steps`
- Body: Ver `mvp/parallel-steps/DESIGN.md`. Necesita decision sobre `JoinSet` vs `FuturesUnordered`, `DashMap` vs `RwLock<HashMap>`, y cancelacion por timeout global vs task.

## #2 [FEATURE] State thread-safe para ejecucion paralela

- Labels: `feature`, `core`, `p0-critical`
- Milestone: `v0.10 Parallel Steps`
- Body: `State` actual necesita frontera explicita para concurrencia. Evaluar `Arc<Mutex<State>>` vs channels, slots aislados y merge post-join.

## #3 [FEATURE] Join strategies para ParallelGroup

- Labels: `feature`, `parallel`, `p1-high`
- Milestone: `v0.10 Parallel Steps`
- Body: Implementar `All`, `Any`, `N(usize)` con tests de exito/fallo.

## #4 [DESIGN] Simulation Layer - Escenarios controlados

- Labels: `design`, `simulation`, `p1-high`
- Milestone: `v0.11 Simulation`
- Body: Definir modos optimistic, adversarial, stochastic y replay.

## #5 [FEATURE] Mock determinista para simulation

- Labels: `feature`, `adapter`, `p1-high`
- Milestone: `v0.11 Simulation`
- Body: Adapter que responde segun scenario, no aleatorio.

## #6 [FEATURE] Reporte de simulacion

- Labels: `feature`, `simulation`, `p2-medium`
- Milestone: `v0.11 Simulation`
- Body: Output YAML/JSON con failures, risk spikes, recommendations y status.

## #7 [DESIGN] OPL v1.0 - Gramatica del DSL

- Labels: `design`, `dsl`, `p1-high`
- Milestone: `v0.12 DSL (OPL)`
- Body: Definir EBNF completo para Agent Packages declarativos.

## #8 [FEATURE] Parser OPL -> AST

- Labels: `feature`, `dsl`, `p1-high`
- Milestone: `v0.12 DSL (OPL)`
- Body: Implementar tokenizer y parser. Output: AST validado.

## #9 [FEATURE] Transpilador AST -> AgentPackage

- Labels: `feature`, `dsl`, `p1-high`
- Milestone: `v0.12 DSL (OPL)`
- Body: Convertir AST a estructura ejecutable con validacion semantica.

## #10 [DESIGN] Meta-Agente - Intent Interpreter

- Labels: `design`, `meta`, `p2-medium`
- Milestone: `v0.13 Meta-Agente`
- Body: Convertir lenguaje natural a estructura operativa.

## #11 [FEATURE] Blueprint Generator

- Labels: `feature`, `meta`, `p2-medium`
- Milestone: `v0.13 Meta-Agente`
- Body: Generar AgentPackage desde intent. Incluye capability mapper, policy generator y risk model.

## #12 [FEATURE] Meta-Agente + Simulation integration

- Labels: `feature`, `meta`, `simulation`, `p2-medium`
- Milestone: `v0.13 Meta-Agente`
- Body: Loop generar -> simular -> validar -> entregar.

## #13 [DESIGN] Control Plane API

- Labels: `design`, `infra`, `p2-medium`
- Milestone: `v1.0 Control/Data Plane`
- Body: REST/GRPC para crear agente, deploy, listar versiones y rollback.

## #14 [FEATURE] Data Plane Worker

- Labels: `feature`, `infra`, `p2-medium`
- Milestone: `v1.0 Control/Data Plane`
- Body: Worker pool que ejecuta AgentPackages y reporta metricas.

## #15 [FEATURE] Versionado inmutable

- Labels: `feature`, `infra`, `p2-medium`
- Milestone: `v1.0 Control/Data Plane`
- Body: SHA-256, changelog y rollback automatico.

## #16 [BUG] Risk Gate no bloquea en estado corrupto

- Labels: `bug`, `risk`, `p3-low`, `mvp-v0.9`
- Milestone: `v0.9 MVP`
- Body: Si `state.paused == true` pero no existe snapshot, el loop deberia bloquear o pedir intervencion.

## #17 [REFACTOR] Eliminar `r#loop` workaround

- Labels: `refactor`, `core`, `p3-low`, `post-mvp`
- Milestone: `v0.10 Parallel Steps`
- Body: Renombrar `loop.rs` a `engine.rs` o `runtime.rs` y actualizar imports.
