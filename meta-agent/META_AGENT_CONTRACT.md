# Contrato Meta-Agente v0.13

## Entrada: HumanIntent mínimo

```rust
HumanIntent {
    raw_input: "Cada hora, fijate si hay facturas nuevas en AFIP, descargalas, guardalas en Drive, y avisame por WhatsApp si hay algo raro".to_string(),
    goal: "automatizar facturación AFIP".to_string(),
    domain: Some("finanzas".to_string()),
    constraints: vec!["legal".to_string(), "credenciales".to_string()],
    environment: Some("browser".to_string()),
    urgency: Some("alta".to_string()),
}
```

## Salida esperada: OPL generado

```opl
CREATE AGENT afip_invoice_processor
FOR "automatizar facturación AFIP"
WITH
  domain = "finanzas",
  autonomy = "supervised",
  audit = "full",
  hitl = "mandatory"

STEP login
  USES afip_login
  RISK HIGH
  HITL true

STEP check_invoices
  USES http_get
  INPUT { path = "/api/invoices", query = { status = "pending" } }
  RISK MEDIUM

STEP download
  USES file_download
  RISK MEDIUM

STEP upload_drive
  USES google_drive_upload
  RISK MEDIUM

STEP notify_whatsapp
  USES whatsapp_send
  RISK LOW

ON timeout
  FALLBACK TO login
  RETRY 3
  BACKOFF EXPONENTIAL

ON invalid_data
  FALLBACK TO check_invoices
  RETRY 2
  BACKOFF FIXED
```

## Reglas de generación

| Input | Output |
|-------|--------|
| `domain = "finanzas"` | `autonomy = "supervised"`, `audit = "full"`, `hitl = "mandatory"` |
| `constraints = ["legal"]` | `RISK HIGH` en steps críticos, `HITL true` |
| `environment = "browser"` | Capabilities de browser/login cuando aplique |
| `goal` con "cada hora" | No genera scheduling: eso es infraestructura, no agente |
| `goal` con "avisame" | Step final de notificación |

## Simulation esperada

| Modo | Resultado |
|------|-----------|
| Optimistic | `APPROVED` |
| Adversarial | `REJECTED` |
| Stochastic (seed 42, 100 runs, 20% failure) | `NEEDS_REVIEW` |

## Tests de contrato

1. `HumanIntent` mantiene la entrada humana mínima.
2. `InterpretedIntent` fija las entidades/capabilities esperadas.
3. El OPL esperado parsea a AST válido.
4. AST transpila a `AgentPackage`.
5. `AgentPackage` simula optimistic/adversarial/stochastic con status esperado.
