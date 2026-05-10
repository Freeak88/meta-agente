# Contrato OPL -> JSON v0.12

## OPL minimo hardcodeado para test

```opl
CREATE AGENT hello_test
FOR "Validar MVP end-to-end"
WITH
  domain = "testing",
  autonomy = "full"

STEP ping
  USES http_ping
  RISK LOW

STEP fetch
  USES http_get
  INPUT "/get"

STEP validate
  USES http_validate
  RISK HIGH
  HITL true

ON timeout
  FALLBACK TO ping
  RETRY 2
  BACKOFF EXPONENTIAL
```

## JSON esperado equivalente al MVP v0.11

```json
{
  "id": "hello_test",
  "mission": "Validar MVP end-to-end",
  "steps": [
    {
      "id": "ping",
      "capability": "http_ping",
      "max_retries": 2,
      "input": null,
      "config_override": null,
      "condition": null,
      "on_skip": null,
      "risk": "LOW"
    },
    {
      "id": "fetch",
      "capability": "http_get",
      "max_retries": 2,
      "input": { "Static": "/get" },
      "config_override": null,
      "condition": null,
      "on_skip": null
    },
    {
      "id": "validate",
      "capability": "http_validate",
      "max_retries": 2,
      "input": null,
      "config_override": null,
      "condition": null,
      "on_skip": null,
      "risk": "HIGH",
      "hitl": true
    }
  ],
  "max_steps": 10,
  "risk_config": {
    "blacklisted_capabilities": [],
    "max_consecutive_failures": 5,
    "max_total_executions": 100
  },
  "global_config": {
    "timeout_ms": 5000,
    "max_retries": 2,
    "backoff_ms": 1000,
    "base_url": null,
    "headers": null,
    "environment": null
  },
  "fallbacks": [
    {
      "condition": "timeout",
      "target": "ping",
      "retry": 2,
      "backoff": "EXPONENTIAL"
    }
  ]
}
```

## Reglas de equivalencia

| OPL | JSON |
|-----|------|
| `STEP id USES cap` | step con defaults |
| `RISK HIGH` | campo `risk` en step |
| `HITL true` | campo `hitl` en step |
| `INPUT "/get"` | `InputSource::Static` |
| `ON timeout FALLBACK TO ping` | fallback en array |
| `RETRY 2` | campo `retry` |
| `BACKOFF EXPONENTIAL` | campo `backoff` |
| `WITH domain = "testing"` | properties del AST |
| `max_retries` no declarado | default 2 |

## Tests de contrato

1. AST minimo hardcodeado serializa a JSON.
2. AST hace roundtrip serde.
3. AST transpila a AgentPackage equivalente al MVP.
4. JSON de contrato matchea exactamente el esperado.
5. JSON generado contiene los campos OPL clave.
6. Transpilacion rechaza agente vacio.

## No verifica

- Syntax highlighting.
- Error messages amigables.
- Performance del parser.
- Templates/herencia.
