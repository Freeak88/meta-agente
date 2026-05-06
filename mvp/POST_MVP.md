# Post-MVP Priorizado

| # | Feature | Bloquea | Complejidad |
|---|---------|---------|-------------|
| 1 | Parallel steps | Sincronizacion, state thread-safe | Alta |
| 2 | Simulation layer | Mocks deterministas, scenario engine | Media |
| 3 | DSL (OPL) | Parser, gramatica, transpilador | Alta |
| 4 | Meta-agente generativo | DSL, simulation, templates | Muy alta |
| 5 | Control Plane / Data Plane | API REST, registry distribuido | Media |
| 6 | Versionado inmutable | Hashes, changelog, rollback | Baja |
| 7 | Observabilidad real | Metricas, health score, alertas | Media |
| 8 | Strategy engine avanzado | Prioridades, policy override | Media |
| 9 | Capability composition | Adapters compuestos | Media |
| 10 | HITL real | API/WebSocket, UI de aprobacion | Alta |

## Branching strategy

- `main`: MVP v0.9 congelado. Solo bugfixes.
- `parallel-steps`: feature branch. Merge cuando pase tests y review.
- `dsl`: feature branch. Depende de `parallel-steps` porque requiere estado thread-safe.
- `meta-agent`: feature branch. Depende de `dsl` y `simulation`.
