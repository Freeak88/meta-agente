# Simulation Report Examples v0.11

## Contrato de salida

Cada simulación retorna `SimulationReport` serializable a JSON.
Estos ejemplos son la verdad. El engine debe generar exactamente
esta estructura, estos campos, estos tipos.

---

## 1. Optimistic

### Input

```json
{
  "agent_id": "invoice_processor",
  "mode": "optimistic",
  "steps": ["login", "fetch", "validate", "submit"]
}
```

### Output

```json
{
  "agent_id": "invoice_processor",
  "mode": "optimistic",
  "runs": [
    {
      "run_id": 1,
      "results": {
        "login": {
          "success": true,
          "data": "{\"status\":\"ok\"}",
          "error": null,
          "timestamp": "2026-05-09T22:43:00Z"
        },
        "fetch": {
          "success": true,
          "data": "{\"count\":3}",
          "error": null,
          "timestamp": "2026-05-09T22:43:01Z"
        },
        "validate": {
          "success": true,
          "data": "{\"valid\":true}",
          "error": null,
          "timestamp": "2026-05-09T22:43:02Z"
        },
        "submit": {
          "success": true,
          "data": "{\"id\":\"inv_123\"}",
          "error": null,
          "timestamp": "2026-05-09T22:43:03Z"
        }
      },
      "execution_time_ms": 3500,
      "status": "PASS"
    }
  ],
  "summary": {
    "pass_rate": 1.0,
    "avg_execution_time_ms": 3500,
    "predicted_failures": [],
    "risk_spikes": [],
    "recommendations": [
      "All steps pass in ideal conditions. Agent ready for deployment.",
      "Consider adding timeout limits to external dependencies."
    ],
    "validation_status": "APPROVED"
  }
}
```

---

## 2. Adversarial

### Input

```json
{
  "agent_id": "invoice_processor",
  "mode": "adversarial",
  "steps": ["login", "fetch", "validate", "submit"]
}
```

### Output

```json
{
  "agent_id": "invoice_processor",
  "mode": "adversarial",
  "runs": [
    {
      "run_id": 1,
      "results": {
        "login": {
          "success": false,
          "data": null,
          "error": "timeout",
          "timestamp": "2026-05-09T22:43:00Z"
        },
        "fetch": {
          "success": false,
          "data": null,
          "error": "timeout",
          "timestamp": "2026-05-09T22:43:00Z"
        },
        "validate": {
          "success": false,
          "data": null,
          "error": "validation_failed",
          "timestamp": "2026-05-09T22:43:00Z"
        },
        "submit": {
          "success": false,
          "data": null,
          "error": "timeout",
          "timestamp": "2026-05-09T22:43:00Z"
        }
      },
      "execution_time_ms": 12000,
      "status": "FAIL"
    }
  ],
  "summary": {
    "pass_rate": 0.0,
    "avg_execution_time_ms": 12000,
    "predicted_failures": [
      {
        "step_id": "login",
        "failure_type": "timeout",
        "probability": 1.0,
        "mitigation": "Add retry with exponential backoff and fallback auth method"
      },
      {
        "step_id": "validate",
        "failure_type": "validation_failed",
        "probability": 1.0,
        "mitigation": "Add input validation before submission and HITL on failure"
      }
    ],
    "risk_spikes": [
      {
        "step_id": "login",
        "risk_score": 1.0,
        "reason": "Critical path failure blocks entire workflow"
      },
      {
        "step_id": "submit",
        "risk_score": 0.95,
        "reason": "Irreversible action with no recovery path"
      }
    ],
    "recommendations": [
      "Agent fails completely under adversarial conditions. Not ready for production.",
      "Add fallback authentication for login step.",
      "Add HITL (human-in-the-loop) for submit step.",
      "Consider circuit breaker pattern for external dependencies."
    ],
    "validation_status": "REJECTED"
  }
}
```

---

## 3. Stochastic (100 runs, seed 12345)

### Input

```json
{
  "agent_id": "invoice_processor",
  "mode": "stochastic",
  "seed": 12345,
  "max_runs": 100,
  "failure_rate": 0.2
}
```

### Output

```json
{
  "agent_id": "invoice_processor",
  "mode": "stochastic",
  "runs": [
    {
      "run_id": 1,
      "results": { "...": "..." },
      "execution_time_ms": 4200,
      "status": "PASS"
    },
    {
      "run_id": 2,
      "results": { "...": "..." },
      "execution_time_ms": 3800,
      "status": "PASS"
    },
    {
      "run_id": 3,
      "results": { "...": "..." },
      "execution_time_ms": 15000,
      "status": "TIMEOUT"
    }
  ],
  "summary": {
    "pass_rate": 0.73,
    "avg_execution_time_ms": 4850,
    "predicted_failures": [
      {
        "step_id": "login",
        "failure_type": "timeout",
        "probability": 0.15,
        "mitigation": "Increase timeout or add retry"
      },
      {
        "step_id": "validate",
        "failure_type": "validation_failed",
        "probability": 0.08,
        "mitigation": "Pre-validate data format"
      }
    ],
    "risk_spikes": [
      {
        "step_id": "submit",
        "risk_score": 0.82,
        "reason": "High variance in execution time indicates instability"
      }
    ],
    "recommendations": [
      "Pass rate 73% is below recommended 80% threshold.",
      "Submit step shows high variance. Consider adding pre-checks.",
      "Add monitoring for timeout patterns in login step."
    ],
    "validation_status": "NEEDS_REVIEW"
  }
}
```

---

## 4. Replay

### Input

```json
{
  "agent_id": "invoice_processor",
  "mode": "replay",
  "execution_log_id": "exec_2026_05_09_001"
}
```

### Output

```json
{
  "agent_id": "invoice_processor",
  "mode": "replay",
  "runs": [
    {
      "run_id": 1,
      "results": {
        "login": {
          "success": true,
          "data": "{\"status\":\"ok\"}",
          "error": null,
          "timestamp": "2026-05-09T22:43:00Z"
        },
        "fetch": {
          "success": true,
          "data": "{\"count\":3}",
          "error": null,
          "timestamp": "2026-05-09T22:43:01Z"
        },
        "validate": {
          "success": false,
          "data": null,
          "error": "validation_failed",
          "timestamp": "2026-05-09T22:43:02Z"
        }
      },
      "execution_time_ms": 8500,
      "status": "FAIL"
    }
  ],
  "summary": {
    "pass_rate": 0.0,
    "avg_execution_time_ms": 8500,
    "predicted_failures": [
      {
        "step_id": "validate",
        "failure_type": "validation_failed",
        "probability": 1.0,
        "mitigation": "Fix identified in commit abc123"
      }
    ],
    "risk_spikes": [],
    "recommendations": [
      "Replay matches original execution exactly. Bug reproduction confirmed.",
      "Apply fix from commit abc123 and re-run simulation."
    ],
    "validation_status": "REJECTED"
  }
}
```

---

## Reglas de generación

### `validation_status`

| Condición | Status |
|-----------|--------|
| `pass_rate == 1.0` y 0 risk_spikes | `APPROVED` |
| `pass_rate >= 0.8` y risk_score < 0.9 | `NEEDS_REVIEW` |
| `pass_rate < 0.8` o risk_score >= 0.9 | `REJECTED` |

### `predicted_failures`

- Solo incluir si `probability > 0.05`
- Ordenar por `probability` DESC
- Máximo 5 items

### `risk_spikes`

- Solo incluir si `risk_score > 0.7`
- Ordenar por `risk_score` DESC
- Máximo 3 items

### `recommendations`

- Mínimo 1, máximo 5
- Primera: siempre resumen del status
- Resto: accionables concretos
- Nunca genéricos ("mejorar código")

---

## Tests de contrato

| # | Test | Valida |
|---|------|--------|
| 1 | `report_schema_valid` | JSON serializa/deserializa sin pérdida |
| 2 | `report_optimistic_approved` | Status correcto para modo optimistic |
| 3 | `report_adversarial_rejected` | Status correcto para modo adversarial |
| 4 | `report_stochastic_needs_review` | Status correcto para 73% pass rate |
| 5 | `report_replay_matches` | Resultados idénticos al log original |
| 6 | `report_limits_items` | Max 5 failures, 3 spikes, 5 recommendations |
