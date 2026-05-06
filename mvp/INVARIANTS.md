# Invariantes del Estado v0.9

## Siempre

1. `paused == true` implica snapshot guardado en disco.
2. `completed == true` implica `paused == false` y `step_index >= steps.len()`.
3. `total_executions` representa llamadas reales a `execute()`.
4. `consecutive_failures == 0` si el ultimo step ejecutado fue exitoso.
5. `results` contiene steps ejecutados y skippeados.

## Nunca

1. Un step ejecutado queda fuera de `results`.
2. `step_index` decrementa excepto durante resume de step fallido.
3. `total_executions` incrementa en skip, input error o risk `BLOCK`.
4. Un adapter accede directamente a `State`.
5. El loop depende de un adapter concreto.

## Si / Entonces

- Si `RiskGate` retorna `Block`, entonces no se ejecuta adapter.
- Si `Condition` evalua `false`, entonces el step se registra como `SKIPPED`.
- Si `max_retries` se excede, entonces `paused = true` y hay snapshot.
- Si `--resume` no encuentra snapshot, entonces inicia fresh con warning.
