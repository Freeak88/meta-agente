# Parallel Steps - Diseno Tecnico v0.10

## Contexto

MVP v0.9 ejecuta steps secuenciales. Para operaciones independientes, como ping a multiples servicios, scraping paralelo o validaciones batch, el loop secuencial se vuelve cuello de botella.

## Problema especifico

Queremos ejecutar N steps simultaneamente, pero sin romper las garantias que hicieron estable al MVP:

- No romper el estado.
- Mantener audit trail determinista.
- Permitir cancelacion parcial.
- Mergear resultados sin race conditions.
- Guardar snapshots solo desde el thread principal.

## Opciones evaluadas

### Opcion A: `tokio::task::JoinSet`

| Aspecto | Valor |
|---------|-------|
| API | Alto nivel, maneja spawning y awaiting |
| Cancelacion | `abort_all()` built-in |
| Resultados | Join de tasks con resultado tipado |
| Orden | Orden de finalizacion, no de declaracion |
| Overhead | Bajo |

Pros:

- API limpia y dificil de usar mal.
- Cancelacion simple.
- Integrado con Tokio, que ya usa el MVP.
- Buen default para I/O async.

Contras:

- Menos control fino sobre scheduling.
- `Any` y `N(usize)` requieren logica adicional encima.

### Opcion B: `FuturesUnordered`

| Aspecto | Valor |
|---------|-------|
| API | Bajo nivel, stream de futures |
| Cancelacion | Manual con `AbortHandle` o flags |
| Resultados | Orden de completitud |
| Orden | No determinista |
| Overhead | Minimo |

Pros:

- Maximo control.
- Permite `select!` complejo.
- Eficiente para muchos futures.

Contras:

- Mas boilerplate.
- Mas facil de usar mal.
- Cancelacion manual es verbosa.

### Opcion C: `rayon`

| Aspecto | Valor |
|---------|-------|
| API | Paralelismo CPU-bound |
| Cancelacion | No nativa |
| Use case | Computacion paralela, no I/O async |

Descartado: nuestro caso principal es I/O async, no procesamiento CPU-bound.

## Decision

Usar `tokio::task::JoinSet` como default para v0.10.

Razon:

- Cubre la mayoria de los casos con API segura.
- Mantiene el sistema dentro del runtime Tokio existente.
- Permite cancelacion global simple con `abort_all()`.
- Reduce superficie de bugs de concurrencia en la primera iteracion.

`FuturesUnordered` queda como escape hatch futuro si aparecen casos avanzados que realmente lo justifiquen.

## Arquitectura propuesta

```text
MAIN TASK
  - Mantiene State original
  - Evalua condiciones del grupo
  - Spawnea workers
  - Recoge resultados
  - Mergea en State
  - Decide continue / pause / snapshot

WORKERS PARALELOS
  - Reciben Step + snapshot inmutable de State
  - Ejecutan capability
  - No mutan State global
  - Devuelven ParallelResult

MERGE PHASE
  - Orden determinista
  - Escritura unica en State
  - Snapshot solo desde main task
```

## Invariantes nuevas

1. State global es single-writer: solo el main task escribe `State`.
2. Workers son read-only respecto a `State`: reciben snapshot inmutable.
3. Resultados se mergean en orden determinista, no por orden de completitud.
4. Snapshot solo ocurre desde main task.
5. Cancelacion aborta tasks, no muta `State`.
6. `total_executions` incrementa por ejecucion real, igual que en modo secuencial.
7. Un step paralelo nunca modifica `step_index` directamente.

## Estructuras nuevas

```rust
#[derive(Debug, Clone)]
struct ParallelGroup {
    id: String,
    steps: Vec<Step>,
    join_condition: JoinCondition,
    timeout_ms: u64,
    max_failures: u32,
}

#[derive(Debug, Clone)]
enum JoinCondition {
    All,
    Any,
    N(usize),
}

#[derive(Debug, Clone)]
struct ParallelResult {
    step_id: String,
    result: Result<StepResult, String>,
    execution_time_ms: u64,
}
```

## Flujo de ejecucion

1. Main task evalua la condicion del `ParallelGroup`.
2. Main task crea un snapshot inmutable del estado requerido por los workers.
3. Main task spawnea tasks con `JoinSet`.
4. Cada worker ejecuta un step sin tocar `State`.
5. Main task espera resultados con timeout global.
6. Main task evalua `JoinCondition`.
7. Main task ordena resultados de forma determinista.
8. Main task mergea en `State.results`.
9. Main task actualiza `step_index`.
10. Main task decide `continue`, `pause` o `snapshot`.

## Join conditions

### `All`

Todos los steps deben terminar OK. Si uno falla, el grupo falla.

### `Any`

Al menos un step debe terminar OK. Si uno termina OK, el grupo puede continuar y cancelar pendientes si corresponde.

### `N(usize)`

Al menos N steps deben terminar OK. Si se vuelve imposible alcanzar N, cancelar pendientes y fallar.

## Cancelacion

```rust
if failures > group.max_failures {
    join_set.abort_all();
}
```

Reglas:

- Timeout global obligatorio.
- `abort_all()` cancela pendientes.
- Resultados parciales solo se mergean si la politica del grupo lo permite.
- Para v0.10, default seguro: si el grupo falla, mergear resultados completados y pausar con snapshot.

## Snapshot en paralelo

Snapshot ocurre solamente despues de la fase de merge:

```rust
if should_pause {
    state.save()?;
}
```

Ningun worker puede llamar `state.save()`.

## Tests planificados

| # | Test | Descripcion |
|---|------|-------------|
| 1 | `parallel_all_ok` | 2 steps, ambos OK, `All` pasa |
| 2 | `parallel_all_one_fail` | 2 steps, uno falla, `All` falla y pausa |
| 3 | `parallel_any_one_ok` | 2 steps, uno OK, `Any` pasa |
| 4 | `parallel_n_of_m` | 3 steps, 2 OK, `N(2)` pasa |
| 5 | `parallel_timeout` | 1 step lento, timeout global cancela |
| 6 | `parallel_max_failures` | 3 steps, 2 fallan rapido, aborta resto |
| 7 | `parallel_state_consistent` | Post-merge sin races ni writes parciales |
| 8 | `parallel_snapshot_safe` | Pause durante parallel genera snapshot valido |

## Riesgos identificados

| Riesgo | Mitigacion |
|--------|------------|
| Deadlock o espera infinita | Timeout global obligatorio |
| Explosion de memoria | `max_parallel_steps`, default 10 |
| Estado inconsistente | Single-writer y merge atomico |
| Snapshot corrupto | Solo main task escribe snapshot |
| Audit no determinista | Ordenar resultados antes de mergear |
| Cancelacion incompleta | `JoinSet::abort_all()` y tests de timeout |

## No entra en v0.10

- Subgrupos anidados.
- Dynamic parallelism.
- Shared mutable state entre workers.
- Backpressure avanzado.
- Prioridades entre tasks paralelos.
- Scheduler propio.

## Dependencias

No se agregan dependencias para la primera version.

`JoinSet` ya esta disponible en Tokio.

Si el diseno cambia y necesitamos mapa concurrente:

```toml
dashmap = "5.5"
```

Pero la preferencia v0.10 es no compartir `State` ni `results` mutablemente con workers.

## Migracion desde v0.9

```rust
while step_index < steps.len() {
    if is_parallel_group(&steps[step_index]) {
        execute_parallel_group(group, state).await;
    } else {
        execute_sequential_step(step, state).await;
    }
}
```

La migracion real debe extraer primero una funcion secuencial reutilizable. No implementar parallel encima de un bloque gigante del loop.

## Checkpoint de diseno

| Item | Estado |
|------|--------|
| Opciones evaluadas | OK |
| Decision `JoinSet` | OK |
| Invariantes nuevas | OK |
| Estructuras nuevas | OK |
| Flujo de ejecucion | OK |
| Cancelacion | OK |
| Tests planificados | OK |
| Riesgos identificados | OK |
| Scope negativo | OK |

## Proximo paso

Resolver issue #2 antes de implementar `ParallelGroup`: definir la frontera thread-safe del estado y extraer una funcion de ejecucion secuencial reusable por workers.
