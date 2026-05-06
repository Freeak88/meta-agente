# Meta-Agente

> Un sistema que crea agentes de software que trabajan solos.

## Que es esto

Meta-Agente es una fabrica de agentes digitales. Le decis que queres que haga, y el sistema arma un agente autonomo que lo ejecuta paso a paso, se recupera de errores y te avisa si necesita ayuda.

No es un chatbot. No es una app. Es un **operador de software** que corre tareas de principio a fin sin que alguien este mirando.

## Ejemplo real

```text
Vos: "Cada hora, fijate si hay facturas nuevas en AFIP, descargalas,
      guardalas en Drive, y avisame por WhatsApp si hay algo raro."

Meta-Agente: arma un agente con estos pasos
  1. Entrar a AFIP con login automatico
  2. Buscar facturas nuevas
  3. Descargar cada una
  4. Subir a Google Drive
  5. Si algo falla mas de 2 veces, pausa y avisa
  6. Si todo ok, manda confirmacion

El agente corre solo. Si AFIP esta caido, reintenta. Si sigue caido,
te manda un mensaje y espera. Vos le decis "segui" y sigue desde
donde quedo.
```

## Como funciona

### 1. Definis la mision

Escribis que queres que haga, en lenguaje normal. El sistema lo traduce a pasos concretos.

### 2. El sistema arma el agente

- Que pasos necesita.
- Que herramientas usa: navegador, APIs, archivos.
- Que puede salir mal.
- Cuando pedir ayuda humana.

### 3. Simulacion antes de ejecutar

Antes de tocar sistemas reales, el agente puede correr en modo simulado. Si algo falla en la simulacion, se arregla antes de salir a produccion.

### 4. Ejecucion autonoma

El agente corre solo. Si todo va bien, termina y reporta. Si algo falla, decide:

- Reintentar.
- Saltar el paso si no es critico.
- Pausar y pedir ayuda si es grave.

### 5. Recuperacion

Si se pausa, guarda exactamente donde quedo. Vos le decis "segui" y continua desde el mismo paso, sin perder nada.

## Que puede hacer hoy

| Capacidad | Descripcion |
|-----------|-------------|
| Pasos secuenciales | Ejecuta pasos uno tras otro, en orden |
| Reintentos automaticos | Si falla, reintenta |
| Pausa inteligente | Si sigue fallando, se detiene y guarda estado |
| Reanudacion | Sigue desde donde quedo |
| Gate de riesgo | Bloquea pasos peligrosos antes de ejecutar |
| Routing de datos | El output de un paso alimenta el siguiente |
| Condicionales | Salta pasos si no se cumplen condiciones |
| HTTP real | Puede llamar APIs reales |
| Mocks | Puede simular APIs para testing sin internet |

## Que viene despues

1. **Pasos paralelos**: hacer varias cosas al mismo tiempo.
2. **Simulacion avanzada**: probar escenarios de fallo antes de ejecutar.
3. **Lenguaje declarativo**: escribir misiones en texto plano, no codigo.
4. **Meta-agente generativo**: el sistema diseña agentes solo, sin programador.
5. **Control/Data plane**: separar quien diseña de quien ejecuta.
6. **Versionado**: volver a versiones anteriores de un agente.
7. **Observabilidad**: dashboard con salud de cada agente.
8. **Intervencion humana real**: aprobar o rechazar pasos desde web/mobile.

## Para desarrolladores

Si queres correrlo local:

```bash
git clone https://github.com/Freeak88/meta-agente.git
cd meta-agente/mvp
cargo test
cargo run
cargo run -- --mock-http
```

Tecnologia: Rust + Tokio. Codigo testeado, documentado y congelado como MVP v0.9.

## Estado

- Version: **v0.9 MVP**
- Repo: [github.com/Freeak88/meta-agente](https://github.com/Freeak88/meta-agente)
- Licencia: MIT
- Estado: privado por ahora

Hecho con la intencion de que el software trabaje solo.
