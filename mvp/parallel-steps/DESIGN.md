# Parallel Steps - Diseno preliminar

## Problema

MVP v0.9 ejecuta secuencialmente. Para operaciones independientes, como ping a multiples servicios o scraping paralelo, el loop secuencial se vuelve cuello de botella.

## Solucion propuesta

Introducir `ParallelGroup` como unidad declarativa post-MVP. No se implementa en v0.9.

```rust
struct ParallelGroup {
    id: String,
    steps: Vec<Step>,
    join_condition: JoinCondition,
    timeout_ms: u64,
    max_failures: u32,
}
```

## Join conditions

- `All`: esperar todos, fallar si alguno falla.
- `Any`: esperar cualquiera, exito si uno termina OK.
- `N(usize)`: esperar N de M, exito si N terminan OK.

## Estado thread-safe

```rust
// Opcion A: Arc<Mutex<State>> con writes controlados.
// Opcion B: channels; workers emiten StepResult y el thread principal mergea.
// Regla preferida: workers no mutan State directamente.
```

Cada step paralelo escribe en un slot aislado. El join mergea results en `State.results` despues de sincronizar.

## Cancelacion

- `tokio::select!` con timeout global.
- `AbortHandle` por step.
- Si `max_failures` se excede, cancelar pendientes.

## Invariantes nuevas

- Un step paralelo nunca modifica `step_index` global.
- `total_executions` incrementa por ejecucion real, igual que en el modo secuencial.
- `results` se mergea despues del join, no durante ejecucion worker.
- Snapshot solo lo escribe el thread principal.

## Bloqueos conocidos

1. `State` necesita una frontera explicita para concurrencia.
2. `results: HashMap` no debe compartirse mutablemente sin lock o merge central.
3. Snapshot no puede dispararse desde workers.

## Dependencias a evaluar

- `tokio::task::JoinSet`
- `dashmap`
- `tokio::sync::mpsc`

## No entra en este diseno

- Sub-groups anidados.
- Dynamic parallelism.
- Shared mutable state entre steps paralelos.
- Cambios al MVP v0.9 congelado.
