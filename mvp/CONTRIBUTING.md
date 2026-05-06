# Como contribuir

## Filosofia

- MVP congelado en `main`. Solo bugfixes.
- Features nuevas en branches: `parallel-steps`, `dsl`, etc.
- Cada feature necesita: diseño, tests, implementacion y review.

## Flujo

1. Leer `ARCHITECTURE.md` e `INVARIANTS.md`.
2. Elegir un item de `POST_MVP.md`.
3. Crear branch desde `v0.9.0-mvp-base`.
4. Escribir `DESIGN.md` en `feature-name/DESIGN.md`.
5. Implementar con tests.
6. Abrir PR a `main` cuando pase CI.

## Codigo

- Rust idiomatico.
- `cargo fmt --check`.
- `cargo test`.
- `cargo clippy` cuando aplique.
- Tests obligatorios para toda logica nueva.
- Sin `unwrap()` en produccion.
- Documentacion para funciones publicas.

## Dudas

Abrir issue con label `question`.
