# Operant MVP v0.9 - Resumen Ejecutivo

Operant MVP v0.9 es una base minima, compilable y testeada para ejecutar Agent Packages declarativos en Rust. El sistema corre steps secuenciales con retry, pausa operacional, snapshots en disco, reanudacion, risk gate, registry de capabilities, adapters mock/HTTP, routing de outputs y condiciones skip/no-skip.

## Que demuestra

- Un agente puede ejecutar un flujo determinista de steps.
- El estado se mantiene consistente y puede persistirse en JSON.
- Un fallo no destruye la corrida: pausa, guarda snapshot y permite resume.
- El loop no esta acoplado a adapters concretos.
- El mismo flujo corre contra mocks o HTTP real con `reqwest`.
- Los steps pueden recibir input estatico, output de otros steps o contexto global.
- Los steps pueden ejecutarse o skippearse por condiciones booleanas simples.

## Estado actual

| Area | Estado |
|------|--------|
| Codigo Rust | Compila |
| Tests | 31 verdes |
| HTTP real | Validado con httpbin |
| Snapshot/resume | Validado |
| Documentacion | Incluida |
| Post-MVP | Priorizado |

## Comandos principales

```bash
cargo fmt --check
cargo test
cargo run
cargo run -- --mock-http
cargo run -- --resume
```

## Decision de producto

El MVP queda congelado en v0.9. La siguiente fase recomendada no es agregar mas features al nucleo, sino estabilizar interfaces, revisar invariantes y preparar una rama separada para parallel steps.

Parallel steps debe tratarse como post-MVP porque introduce sincronizacion, dependencias concurrentes, orden de escritura al estado, cancelacion y merge de resultados.
