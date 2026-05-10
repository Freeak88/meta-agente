# Meta-Agente v1.0

> Un sistema que crea agentes de software que trabajan solo.

## Qué es

Meta-Agente es una fábrica de agentes digitales autónomos. Le decís qué
querés que haga en lenguaje natural, y el sistema:

1. **Interpreta** tu intención
2. **Genera** un agente en lenguaje declarativo (OPL)
3. **Simula** el agente antes de ejecutar
4. **Entrega** un paquete validado vía API REST

## Quickstart

```bash
# Clonar
git clone https://github.com/Freeak88/meta-agente.git
cd meta-agente/mvp

# Correr tests
cargo test

# Levantar servidor API
cargo run -- --server
```

En otra terminal, crear un agente:

```bash
curl -X POST http://localhost:3000/agents \
  -H "Content-Type: application/json" \
  -d '{
    "intent": {
      "raw_input": "Cada hora, fijate si hay facturas nuevas en AFIP",
      "goal": "automatizar facturación AFIP",
      "domain": "finanzas"
    }
  }'
```

## Arquitectura

```text
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Human     │────▶│  MetaAgent  │────▶│    OPL      │
│   Intent    │     │  Generator  │     │   Source    │
└─────────────┘     └─────────────┘     └──────┬──────┘
                                               │
┌─────────────┐     ┌─────────────┐     ┌──────▼──────┐
│   Deploy    │◀────│  Validate   │◀────│  Simulate   │
│   Package   │     │  & Approve  │     │  4 Modes    │
└─────────────┘     └─────────────┘     └─────────────┘
```

## Historia de versiones

| Versión | Qué agrega |
|---------|------------|
| v0.9 | Loop, retry, pause, resume, snapshot, risk gate |
| v0.10 | Pasos paralelos |
| v0.11 | Simulación: optimistic, adversarial, stochastic, replay |
| v0.12 | DSL (OPL) parser y transpiler |
| v0.13 | Meta-agente generativo: intento humano a OPL |
| **v1.0** | **API REST, servidor HTTP, generación end-to-end** |

## Roadmap

- **v1.1** MCP/Skills/Tools: integración con herramientas externas
- **v1.2** Cloud: hosting managed, escalado automático
- **v1.3** Enterprise: on-premise, compliance, SSO

## Licencia

MIT. Libre para usar, modificar y distribuir.

Hecho con intención de que el software trabaje solo.
