# Milestones

## v0.9 MVP - CERRADO

- Fecha: 2026-05-06
- Estado: congelado
- Scope: loop, persistence, risk, registry, HTTP, routing, conditions
- Tests: 31

## v0.10 Parallel Steps

- Fecha target: 2026-05-20
- Scope: ejecucion concurrente de steps independientes
- Bloquea: v0.11 Simulation
- Issues: design, state thread-safe, join strategies

## v0.11 Simulation

- Fecha target: 2026-06-03
- Scope: dry runs con escenarios controlados
- Bloquea: v0.12 DSL
- Issues: scenario engine, mock determinista, reporte

## v0.12 DSL (OPL)

- Fecha target: 2026-06-17
- Scope: parser de lenguaje declarativo
- Bloquea: v0.13 Meta-Agente
- Issues: gramatica, parser, transpilador

## v0.13 Meta-Agente Generativo

- Fecha target: 2026-07-01
- Scope: fabrica automatica de Agent Packages
- Issues: intent interpreter, blueprint generator, simulation integration

## v1.0 Control/Data Plane

- Fecha target: 2026-08-01
- Scope: separacion de generacion y ejecucion, API REST, registry distribuido
- Issues: control plane API, data plane worker, versionado
