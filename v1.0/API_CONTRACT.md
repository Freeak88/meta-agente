# API Contract v1.0

## Request: POST /agents

```json
{
  "intent": {
    "raw_input": "Cada hora, fijate si hay facturas nuevas en AFIP",
    "goal": "automatizar facturación AFIP",
    "domain": "finanzas",
    "constraints": ["legal", "credenciales"],
    "environment": "browser",
    "urgency": "alta"
  }
}
```

## Response: 201 Created

```json
{
  "agent_id": "automatizar_facturacion_afip_agent",
  "version": "1.0.0",
  "hash": "sha256:a3f7c2...",
  "opl": "CREATE AGENT automatizar_facturacion_afip_agent\nFOR \"automatizar facturación AFIP\"...",
  "simulation_report": {
    "agent_id": "automatizar_facturacion_afip_agent",
    "mode": "optimistic",
    "runs": [],
    "summary": {
      "pass_rate": 1.0,
      "validation_status": "APPROVED"
    }
  },
  "status": "APPROVED",
  "created_at": "2026-05-10T11:35:00Z"
}
```

## Reglas

- `hash` = SHA-256 del OPL source
- `version` = semver, empieza en `1.0.0`
- `status` = `APPROVED` | `REJECTED` | `NEEDS_REVIEW` (de simulation)
- Si `REJECTED` -> HTTP `422 Unprocessable Entity` con `errors` array
- Si `NEEDS_REVIEW` -> HTTP `202 Accepted` con `warnings` array
