# Operant MVP v0.9

Framework minimo de agentes autonomos en Rust. Un operante ejecuta un Agent Package: steps declarativos con retry, pause, resume, risk gate, routing de datos y condicionales.

## Arquitectura

```text
AgentPackage (steps)
    -> ExecutionLoop
    -> RiskGate
    -> CapabilityRegistry
    -> CapabilityAdapter (mock/HTTP)
    -> State snapshot
```

## Loop de ejecucion

1. Evaluar condicion: skip si no se cumple.
2. Resolver input: `Static`, `FromStep`, `FromContext`, `Merge`.
3. Risk gate: `ALLOW` o `BLOCK` antes de ejecutar.
4. Resolver capability: registry a adapter.
5. Ejecutar adapter con input y config mergeada.
6. Actualizar estado.
7. Guardar snapshot al pausar o completar.

## Flags

| Flag | Efecto |
|------|--------|
| `--resume` | Carga snapshot, retrocede al step fallido y reintenta |
| `--mock-http` | Usa `MockHttpGet` en vez de `reqwest` real |
| `--block-validate` | Blacklistea `http_validate` para probar `BLOCK` |
| `--strict` | Usa `RiskConfig` estricto |

## Invariantes clave

- `total_executions` solo incrementa en llamadas reales a `execute`.
- `consecutive_failures` resetea en exito y acumula en fallo.
- `paused` implica snapshot guardado en disco.
- `completed` implica `paused == false` y `step_index >= steps.len()`.
- `results` contiene cada step ejecutado o skippeado.
- Un step skippeado aparece con `data: "SKIPPED"` y `success: true`.

## Config

Precedencia fija:

```text
Registry default -> Agent global -> Step override
```

## Tests y ejecucion

```bash
cargo fmt --check
cargo test
cargo run
cargo run -- --mock-http
cargo run -- --resume
```

## Post-MVP

1. Parallel steps
2. Simulation layer
3. DSL (OPL)
4. Meta-agente generativo
5. Control Plane / Data Plane
6. Versionado inmutable
7. Observabilidad real
8. Strategy engine avanzado
9. Capability composition
10. HITL real

## Licencia

MIT. MVP experimental.
