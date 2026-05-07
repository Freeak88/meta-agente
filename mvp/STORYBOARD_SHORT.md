# Storyboard 60s

Recorte quirurgico para redes o pitch rapido.

| Tiempo | Pantalla | Voz |
|--------|----------|-----|
| 0:00 - 0:05 | Logo `Meta-Agente` | "Meta-Agente arma operadores digitales que trabajan solos." |
| 0:05 - 0:15 | `cargo run` con HTTP real | "El agente llama una API real y usa el resultado para el siguiente paso." |
| 0:15 - 0:30 | `cleanup` SKIPPED | "Si un paso no hace falta, lo salta y lo deja registrado." |
| 0:30 - 0:45 | `validate` falla y pausa | "Si algo falla, reintenta. Si sigue fallando, guarda estado y se detiene." |
| 0:45 - 0:55 | `cargo run -- --resume` | "Despues sigue desde donde quedo." |
| 0:55 - 1:00 | `[LOOP] COMPLETED` + repo | "Software que trabaja solo." |

## Mantener

- Fallo + pause.
- Resume + completed.
- Una frase clara de valor.

## Quitar

- Introduccion larga del problema.
- JSON completo.
- Explicacion tecnica.
