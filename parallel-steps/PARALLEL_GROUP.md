# ParallelGroup - Issue #3

## Scope

`ParallelGroup` agrega ejecucion concurrente minima sin reescribir el
runtime secuencial. La ruta v0.9 sigue intacta; el grupo paralelo se
ejecuta como una unidad explicita y luego mergea sus resultados en
`State`.

## Estructuras

```rust
pub struct ParallelGroup {
    pub id: String,
    pub steps: Vec<Step>,
    pub join_strategy: JoinStrategy,
    pub max_concurrency: Option<usize>,
}

pub enum JoinStrategy {
    AllSuccess,
    AnySuccess,
    Quorum(usize),
    FailFast,
}
```

`max_concurrency` queda definido en el contrato. En esta iteracion solo
se acepta `None` o un valor igual/mayor a la cantidad de steps. El limite
real con backpressure queda para una tarea posterior.

## Worker Contract

- Cada worker recibe `WorkerContext`.
- `WorkerContext` contiene `AgentStateSnapshot`, no `&mut State`.
- El registry se clona como handle liviano con adapters en `Arc`.
- El worker retorna `WorkerResult`.
- El worker no guarda snapshot, no cambia `step_index`, no toca
  `State.results`.

## Merge Contract

- El main loop recoge todos los `WorkerResult`.
- El orden de merge es el orden declarativo del grupo.
- `State::merge_results()` es el unico punto de escritura masiva.
- Si un child step ya existe en `State.results`, el grupo se rechaza.
- Si dos children comparten `id`, el grupo se rechaza.

## Join Semantics

| Strategy | Sucede cuando |
|----------|---------------|
| `AllSuccess` | Todos los children devuelven `success=true` |
| `AnySuccess` | Al menos un child devuelve `success=true` |
| `Quorum(n)` | `n` o mas children devuelven `success=true` |
| `FailFast` | Falla al observar el primer fallo y aborta lo pendiente cuando Tokio lo permite |

Si el join falla, el main thread marca `paused=true` y guarda snapshot.

## Invariantes nuevas

1. Workers nunca reciben `&mut State`.
2. Workers solo pueden devolver `WorkerResult`.
3. El merge es determinista por orden declarativo, no por completion time.
4. Writes conflictivos se rechazan antes de spawnear workers.
5. `FailFast` puede abortar tareas pendientes, pero no deshace resultados
   ya observados.
6. La ruta secuencial existente no cambia su comportamiento.

## Tests cubiertos

- `AllSuccess` pasa cuando todos pasan.
- `AllSuccess` falla con un child fallido.
- `AnySuccess` pasa con un exito.
- `Quorum(2)` pasa con 2/3 exitos.
- `Quorum(2)` falla con 1/3 exitos.
- Merge deterministico por orden declarativo.
- Worker no muta `State` vivo directamente.
- `FailFast` falla ante el primer fallo observado.
