# Changelog MVP

## v0.1 - Loop basico

- Loop determinista: step -> capability -> execute -> result -> next.
- Retry basico.
- Pause en fallo.

## v0.2 - State persistence

- `save`, `load`, `resume` en `state.rs`.
- Snapshot JSON en `./snapshots/`.
- Flag `--resume`.

## v0.3 - Risk gate

- `RiskGate` con `ALLOW` / `BLOCK`.
- `RiskConfig`: blacklist, max failures, max executions.
- Flags `--block-validate`, `--strict`.

## v0.4 - Capability registry

- `CapabilityRegistry`: `capability_id -> adapter`.
- `CapabilityAdapter` trait.
- Loop desacoplado de mocks.

## v0.5 - HTTP real

- `HttpGetAdapter` con `reqwest`.
- Timeout con `tokio::timeout`.
- Flag `--mock-http`.

## v0.6 - Input + config por step

- `Step.input`.
- `config_override` por step.
- Validacion minima de input requerido.

## v0.7 - Config global por agente

- `AgentGlobalConfig`: timeout, retries, backoff, base_url, headers.
- Merge: registry -> global -> step override.

## v0.8 - Output routing

- `InputSource`: `Static`, `FromStep`, `FromContext`, `Merge`.
- Routing declarativo entre steps.
- Error en resolucion pausa con snapshot.

## v0.9 - Conditional steps

- `Condition`: `StateEq`, `StateGt`, `OutputOk`, `OutputFailed`, `And`, `Or`, `Not`.
- Skip registrado en results.
- `on_skip` message.
- Branching basico sin grafos.
