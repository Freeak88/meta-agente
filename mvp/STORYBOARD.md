# Storyboard: Meta-Agente MVP v0.9

Duracion: 90 segundos

Resolucion: 1920x1080, terminal a pantalla completa

Fuente sugerida: Fira Code 16pt, fondo `#1a1a1a`, texto `#e0e0e0`

## Pre-roll

### Setup terminal

```powershell
# Windows Terminal, PowerShell, fondo oscuro
# Zoom: Ctrl++ hasta que el texto ocupe buena parte de la pantalla
function prompt { "$ " }
```

### Pre-compilar

```powershell
cd D:\Repos\agentify\mvp
cargo build --release
cargo test
```

### Limpieza

```powershell
if (Test-Path snapshots\hello_test_0.1.json) { Remove-Item snapshots\hello_test_0.1.json }
```

## Escena 1: El problema (0:00 - 0:10)

| Timestamp | 0:00 - 0:05 |
|-----------|-------------|
| Pantalla | Negro, texto blanco centrado |
| Texto | `Todos los dias hay tareas que una computadora deberia hacer sola.` |
| Voz | "Todos los dias hay tareas que una computadora deberia hacer sola." |
| Musica | Fade in ambiental, volumen bajo |

| Timestamp | 0:05 - 0:10 |
|-----------|-------------|
| Pantalla | Planilla, facturas o formularios manuales |
| Voz | "Pero alguien tiene que estar ahi. Si falla, se pierde contexto." |
| Corte | Duro a negro |

## Escena 2: La promesa (0:10 - 0:20)

| Timestamp | 0:10 - 0:15 |
|-----------|-------------|
| Pantalla | Negro, texto `Meta-Agente` centrado |
| Voz | "Meta-Agente arma operadores digitales que trabajan solos." |

| Timestamp | 0:15 - 0:20 |
|-----------|-------------|
| Pantalla | Cuatro bloques: `Mision -> Pasos -> Ejecuta -> Recupera` |
| Voz | "Le decis que queres. El sistema arma los pasos. El agente corre. Si falla, se recupera." |
| Transicion | Fade a terminal |

## Escena 3: Demo real, parte 1 (0:20 - 0:45)

| Timestamp | 0:20 - 0:25 |
|-----------|-------------|
| Pantalla | Terminal limpia, solo prompt |
| Comando | `cargo run` |
| Voz | "Vamos a ver un agente que consulta un servicio web real." |
| Accion | Escribir comando y Enter |

| Timestamp | 0:25 - 0:35 |
|-----------|-------------|
| Pantalla | Output apareciendo |
| Enfocar | `[STEP] fetch`, `[RISK] ALLOW`, `[OK]` |
| Voz | "Primero llama una API real. El sistema evalua riesgo, permite ejecutar y confirma que todo esta bien." |

| Timestamp | 0:35 - 0:45 |
|-----------|-------------|
| Pantalla | Output de `extract` y `cleanup` |
| Enfocar | `[INPUT] resolved`, `[COND] SKIPPED` |
| Voz | "Despues usa el resultado de un paso como input del siguiente. Tambien salta pasos que no hacen falta." |
| Corte | Pausa breve, limpiar terminal |

## Escena 4: El fallo (0:45 - 1:05)

| Timestamp | 0:45 - 0:50 |
|-----------|-------------|
| Pantalla | Terminal limpia |
| Comando | `cargo run -- --mock-http` |
| Voz | "Ahora simulemos que algo falla." |

| Timestamp | 0:50 - 1:00 |
|-----------|-------------|
| Pantalla | Output con `validate` fallando |
| Enfocar | `[FAIL] err=validation_failed` |
| Voz | "El paso de validacion fallo. El agente reintenta automaticamente." |

| Timestamp | 1:00 - 1:05 |
|-----------|-------------|
| Pantalla | `[STRATEGY] max retries exceeded -> save snapshot & pause` |
| Voz | "Como siguio fallando, se detuvo y guardo exactamente donde quedo." |
| Pausa | 1 segundo sobre `[LOOP] PAUSED` |

## Escena 5: Recuperacion (1:05 - 1:20)

| Timestamp | 1:05 - 1:10 |
|-----------|-------------|
| Pantalla | Terminal limpia |
| Comando | `cargo run -- --resume` |
| Voz | "Ahora le decimos que siga." |

| Timestamp | 1:10 - 1:15 |
|-----------|-------------|
| Pantalla | Snapshot cargado |
| Enfocar | `[STATE] snapshot loaded` |
| Voz | "Carga el estado guardado." |

| Timestamp | 1:15 - 1:20 |
|-----------|-------------|
| Pantalla | `validate` OK y `[LOOP] COMPLETED` |
| Voz | "Sigue desde donde quedo. Sin empezar de cero. Sin perder datos." |

## Escena 6: Cierre (1:20 - 1:30)

| Timestamp | 1:20 - 1:25 |
|-----------|-------------|
| Pantalla | Negro, texto blanco |
| Texto | `Esto es un MVP. Ya funciona.` |
| Voz | "Esto es un MVP. Ya funciona." |

| Timestamp | 1:25 - 1:30 |
|-----------|-------------|
| Pantalla | Repo + claim |
| Texto | `github.com/Freeak88/meta-agente` |
| Voz | "Software que trabaja solo." |
| Musica | Fade out |

## Fallback si falla la red

Si `cargo run` falla por red:

```powershell
cargo run -- --mock-http
```

Voz:

```text
El servicio externo fallo. Para eso existe el modo mock: podemos validar el comportamiento sin depender de internet.
```

Si queda un snapshot viejo:

```powershell
Remove-Item snapshots\hello_test_0.1.json
```

## Cortes y zoom

| Momento | Accion | Razon |
|---------|--------|-------|
| Inicio de comando | Zoom 1.2x al prompt | Enfocar atencion |
| Lineas `[OK]`, `[FAIL]`, `[PAUSED]` | Zoom 1.3x | Momento emocional |
| JSON largo | Zoom out | Contexto |
| Entre escenas | Corte duro o fade corto | Ritmo |
| Post-ejecucion | Pausa 1-2s | Lectura |

## Checklist pre-grabacion

- [ ] Terminal configurada.
- [ ] `cargo build --release` ejecutado.
- [ ] `cargo test` verde.
- [ ] Snapshot limpio.
- [ ] Microfono probado.
- [ ] Musica ambiental lista.
- [ ] Script abierto en segunda pantalla.
- [ ] Timer visible.

## Post-produccion minima

- Cortar silencios mayores a 2 segundos.
- Voz clara, musica baja.
- Subtitulos opcionales.
- Thumbnail sugerido: terminal con `[LOOP] COMPLETED`.
