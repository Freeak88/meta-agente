# Cheat Sheet: Demo en vivo

## Preparacion

```powershell
cd D:\Repos\agentify\mvp
cargo test
```

Esperado:

```text
31 passed
```

## Demo principal: HTTP real + pausa

```powershell
if (Test-Path snapshots\hello_test_0.1.json) { Remove-Item snapshots\hello_test_0.1.json }
cargo run
```

Esperado:

- `fetch` OK con HTTP real.
- `extract` OK usando output de `fetch`.
- `cleanup` SKIPPED.
- `merge_test` OK.
- `context_test` OK.
- `never_runs` SKIPPED.
- `validate` falla, reintenta y pausa.

## Resume

```powershell
cargo run -- --resume
```

Esperado:

- Carga snapshot.
- Retrocede a `validate`.
- `validate` OK.
- `[LOOP] COMPLETED`.

## Demo sin internet

```powershell
if (Test-Path snapshots\hello_test_0.1.json) { Remove-Item snapshots\hello_test_0.1.json }
cargo run -- --mock-http
```

Esperado:

- Misma historia, pero con `http_get (MOCK)`.
- No depende de httpbin ni de red.

## Risk gate bonus

```powershell
if (Test-Path snapshots\hello_test_0.1.json) { Remove-Item snapshots\hello_test_0.1.json }
cargo run -- --block-validate
```

Esperado:

- Corre steps previos.
- Bloquea `validate` antes de ejecutarlo.
- Guarda snapshot y pausa.

## Recuperacion si algo sale mal

Si hay error de red:

```powershell
cargo run -- --mock-http
```

Frase util:

```text
La red fallo, pero para eso existe el modo mock: podemos validar el comportamiento sin depender de servicios externos.
```

Si la terminal se ensucia:

```powershell
cls
```

Si queda snapshot viejo:

```powershell
Remove-Item snapshots\hello_test_0.1.json
```

## No mostrar

- Codigo fuente.
- Builds largos.
- Logs completos si no entran en pantalla.
- Tokens, secretos o rutas personales.
