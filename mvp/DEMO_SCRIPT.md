# Demo Script: Meta-Agente MVP v0.9

Duracion objetivo: 90 segundos

Audiencia: cualquiera con un navegador

## Escena 1: El problema (10s)

Pantalla negra, texto blanco:

```text
Todos los dias hay tareas que una computadora deberia hacer sola.
```

Corte a una pantalla con facturas, formularios o planillas.

Narrador:

```text
Pero alguien tiene que estar ahi. Si falla, se pierde contexto. Si cambia algo, se rompe.
```

## Escena 2: La promesa (10s)

Pantalla negra, logo o texto Meta-Agente.

Narrador:

```text
Meta-Agente arma operadores digitales que trabajan solos.

Le decis que queres. El sistema arma los pasos. El agente corre.
Si falla, reintenta. Si no puede, guarda donde quedo y te pide ayuda.
```

## Escena 3: Demo real (50s)

Terminal limpia, fondo oscuro, fuente grande.

Narrador:

```text
Vamos a ver un agente que consulta un servicio web real, usa el resultado en pasos siguientes,
salta pasos que no hacen falta y se detiene si algo falla.
```

Comando:

```bash
cargo run
```

Mostrar salida representativa:

```text
=== OPERANT MVP v0.9 ===

[STEP] fetch (capability=http_get)
  [COND] condition met
  [INPUT] resolved: "/get"
  [RISK] ALLOW
  [OK] data={"url":"https://httpbin.org/get", ...}

[STEP] extract (capability=http_get)
  [COND] condition met
  [INPUT] resolved: "https://httpbin.org/get"
  [OK] ...

[STEP] cleanup (capability=http_get)
  [COND] SKIPPED: condition not met
```

Narrador:

```text
El agente llamo una API real. Despues uso el output de ese paso como input del siguiente.
Tambien salto un paso de cleanup porque no hacia falta.
```

Continuar salida:

```text
[STEP] validate (capability=http_validate)
  [FAIL] err=validation_failed
  [FAIL] err=validation_failed
  [STRATEGY] max retries exceeded -> save snapshot & pause

[LOOP] PAUSED (snapshot saved, can resume)
```

Narrador:

```text
Ahora algo fallo. El agente reintento. Como siguio fallando, se detuvo y guardo exactamente donde quedo.
```

Pausa breve.

Narrador:

```text
Ahora le decimos que siga.
```

Comando:

```bash
cargo run -- --resume
```

Mostrar salida representativa:

```text
[STATE] snapshot loaded from ./snapshots/hello_test_0.1.json
[STATE] resumed from step 7 (validate)
[LOOP] rewound to retry failed step

[STEP] validate (capability=http_validate)
  [OK] data={"checks_passed":3,"valid":true}

[LOOP] COMPLETED
```

Narrador:

```text
Siguio desde donde quedo. Sin empezar de cero. Sin perder datos.
```

## Escena 4: Cierre (20s)

Pantalla negra, texto:

```text
Esto es un MVP. Ya funciona.
```

Narrador:

```text
Lo que viene: pasos paralelos, simulacion avanzada y un lenguaje para describir misiones en texto plano.
```

Pantalla final:

```text
github.com/Freeak88/meta-agente

Software que trabaja solo.
```

## Notas tecnicas para grabar

- Usar terminal con fondo oscuro y fuente grande.
- Ocultar path completo si distrae.
- Compilar antes de grabar para evitar ruido de build.
- Usar `cargo run -- --mock-http` si internet esta inestable.
- Voz neutra. No vender de mas: dejar que el comportamiento hable.

## Variante 60s

- Quitar escena 1.
- Empezar directo con: "Vamos a ver un agente que trabaja solo".
- Mantener tres momentos: ejecuta, pausa, resume.

## Variante 3min

- Mostrar routing: `extract` usa `fetch.url`.
- Mostrar skip condicional: `cleanup` y `never_runs`.
- Mostrar `cargo test`: 31 tests verifican el comportamiento.
