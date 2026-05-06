# Decisiones Oficiales MVP v0.9

## DO-01: Loop determinista

El loop ejecuta steps en orden secuencial. No hay grafos ni `goto`. El unico desvio es skip por condicion.

## DO-02: Snapshot JSON

El snapshot es JSON serializado del `State` completo. Se carga con `State::load` y se guarda con `State::save`.

## DO-03: Risk antes de ejecucion

Risk Gate evalua antes de tocar cualquier adapter. `BLOCK` implica snapshot y pause, sin ejecucion parcial.

## DO-04: Registry desacopla adapter

El loop no sabe si corre mock, HTTP, browser o CLI. Solo conoce `capability_id`; el registry resuelve el adapter.

## DO-05: Config mergeada

Precedencia fija: registry default -> agent global -> step override. No hay herencia dinamica fuera de esa regla.

## DO-06: Skip no es fallo

Un step skippeado registra `success: true` con `data: "SKIPPED"`. No incrementa `total_executions` y no dispara retry.

## DO-07: Resume retrocede

Al reanudar, si el ultimo step procesado fallo, el loop retrocede `step_index` para reintentar. Si fue skip o exito, avanza normalmente.

## DO-08: Input routing declarativo

`InputSource` se evalua antes de ejecutar. Si falla la resolucion, el loop pausa con snapshot.

## DO-09: Condiciones booleanas simples

`Condition` soporta `And`, `Or`, `Not`, comparaciones simples de estado y estado de outputs. No hay regex, aritmetica compleja ni agregaciones.

## DO-10: Adapter trait minimo

`execute(input, config) -> ExecutionResult`. Sin callbacks, streaming ni websockets. Request/response async.
