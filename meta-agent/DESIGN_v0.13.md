# Meta-Agente Generativo - Diseno Tecnico v0.13

## Contexto

Hoy el DSL (OPL) requiere que un humano escriba codigo declarativo.
Queremos que un humano escriba en lenguaje natural y el sistema
genere el OPL, lo simule, y entregue un paquete validado.

## Arquitectura

```text
+-----------------------------------------+
|         INTENT INTERPRETER              |
|  - Recibe texto libre humano            |
|  - Extrae: objetivo, dominio,           |
|    constraints, entorno                 |
+-----------------------------------------+
|         CAPABILITY MAPPER               |
|  - Mapea objetivo a capabilities        |
|  - Busca en registry de capabilities    |
|  - Compone flujos de capabilities       |
+-----------------------------------------+
|         BLUEPRINT GENERATOR             |
|  - Genera OPL v1.0                      |
|  - Aplica templates por dominio         |
|  - Deriva risk model, policies          |
+-----------------------------------------+
|         SIMULATION VALIDATOR            |
|  - Corre simulation adversarial         |
|  - Si REJECTED -> regenera con fixes    |
|  - Si NEEDS_REVIEW -> ajusta thresholds |
|  - Si APPROVED -> entrega               |
+-----------------------------------------+
|         AGENT PACKAGE OUTPUT            |
|  - OPL + SimulationReport + Metadata    |
+-----------------------------------------+
```

## Estructuras nuevas

```rust
pub struct HumanIntent {
    pub raw_input: String,
    pub goal: String,
    pub domain: Option<String>,
    pub constraints: Vec<String>,
    pub environment: Option<String>,
    pub urgency: Option<String>,
}

pub struct MetaAgentConfig {
    pub max_iterations: u32,
    pub simulation_threshold: ValidationStatus,
    pub templates_enabled: bool,
    pub learning_enabled: bool,
}

pub struct GeneratedAgent {
    pub opl_source: String,
    pub ast: OplAst,
    pub package: AgentPackage,
    pub simulation_report: SimulationReport,
    pub generation_metadata: GenerationMetadata,
}

pub struct GenerationMetadata {
    pub iterations: u32,
    pub initial_intent: String,
    pub final_status: ValidationStatus,
    pub improvements: Vec<String>,
}
```

## Flujo de generacion

1. Recibir HumanIntent.
2. Interpretar: extraer entidades, dominio, capabilities necesarias.
3. Mapear: capability_id -> adapter disponible en registry.
4. Generar OPL draft (Blueprint Generator).
5. Parsear OPL -> AST (validar sintaxis).
6. Transpilar AST -> AgentPackage.
7. Simular: Optimistic + Adversarial.
8. Evaluar resultado:
   - APPROVED -> entregar GeneratedAgent.
   - NEEDS_REVIEW -> ajustar thresholds, re-simular.
   - REJECTED -> analizar failures, regenerar OPL con fixes.
9. Si max_iterations excedido -> entregar mejor intento + warning.

## Templates por dominio

```rust
pub struct DomainTemplate {
    pub domain: String,
    pub default_capabilities: Vec<String>,
    pub default_policies: Vec<Policy>,
    pub risk_threshold: f64,
    pub hitl_required: Vec<String>,
}
```

Ejemplos:

- `finanzas`: audit full, HITL mandatory, retention long.
- `marketing`: autonomy high, HITL on anomaly, audit minimal.
- `devops`: autonomy full, HITL none, audit minimal.

## Iteracion y mejora

```rust
fn regenerate_with_fixes(
    previous_opl: &str,
    failures: &[PredictedFailure],
    iteration: u32,
) -> String {
    let mut opl = previous_opl.to_string();

    for failure in failures {
        match failure.failure_type.as_str() {
            "timeout" => {
                opl = add_retry_and_fallback(&opl, &failure.step_id);
            }
            "validation_failed" => {
                opl = add_prevalidation_step(&opl, &failure.step_id);
            }
            _ => {
                opl = add_generic_fallback(&opl, &failure.step_id);
            }
        }
    }

    opl
}
```

## Invariantes del meta-agente

1. Nunca entrega OPL invalido: si parse falla, regenera.
2. Nunca entrega sin simulation: siempre simula antes de entregar.
3. Nunca ignora REJECTED: si simulation rechaza, itera o advierte.
4. HumanIntent nunca se persiste: solo el OPL generado y el reporte.
5. Mejora es determinista: mismo intent + mismo registry -> mismo OPL (con seed).

## Tests planificados

| # | Test | Descripcion |
|---|------|-------------|
| 1 | `meta_generate_simple` | "Enviar email" -> OPL con 1 step |
| 2 | `meta_generate_complex` | "Facturacion AFIP" -> OPL con 5+ steps |
| 3 | `meta_simulation_approved` | Genera, simula, APPROVED |
| 4 | `meta_simulation_rejected_then_fixed` | REJECTED -> regenera -> APPROVED |
| 5 | `meta_domain_template_finance` | Dominio finanzas -> HITL mandatory |
| 6 | `meta_capability_not_found` | Intent requiere cap inexistente -> error |
| 7 | `meta_deterministic` | Mismo intent -> mismo OPL |
| 8 | `meta_max_iterations` | Siempre REJECTED -> entrega warning |

## No entra en v0.13

- LLM real (GPT-4, Claude): usamos heuristicas + templates, no LLM.
- Web UI para meta-agente: CLI only.
- Learning engine persistente: solo in-memory.
- Multi-intent composition: un intent -> un agente.
- Pricing/cost optimization: no monetizacion.

## Dependencias a agregar

```toml
# Ninguna nueva. Todo con structs y logica existente.
# Post-v0.13: opcional integracion con LLM API.
```

## Checkpoint de diseno

| Item | Estado |
|------|--------|
| Arquitectura 5 capas | OK |
| Estructuras nuevas | OK |
| Flujo de generacion | OK |
| Templates por dominio | OK |
| Iteracion y mejora | OK |
| Invariantes | OK |
| Tests planificados | OK |
| Scope negativo | OK |
