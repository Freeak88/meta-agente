#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::dsl::{parse_opl, OplTranspiler, OplValue};
use crate::r#loop::AgentPackage;
use crate::simulation::{
    SimulationConfig, SimulationEngine, SimulationMode, SimulationReport, ValidationStatus,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HumanIntent {
    pub raw_input: String,
    pub goal: String,
    pub domain: Option<String>,
    pub constraints: Vec<String>,
    pub environment: Option<String>,
    pub urgency: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InterpretedIntent {
    pub goal: String,
    pub domain: String,
    pub required_capabilities: Vec<String>,
    pub constraints: Vec<String>,
    pub risk_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GeneratedOpl {
    pub source: String,
    pub intent_hash: String,
}

#[derive(Debug, Clone)]
pub struct MetaAgentResult {
    pub opl: GeneratedOpl,
    pub package: AgentPackage,
    pub simulation: SimulationReport,
    pub status: String,
}

pub struct IntentInterpreter;

impl IntentInterpreter {
    pub fn interpret(intent: &HumanIntent) -> InterpretedIntent {
        let domain = intent
            .domain
            .clone()
            .unwrap_or_else(|| "general".to_string());
        let required_capabilities =
            Self::extract_capabilities(&intent.raw_input, &intent.goal, &domain);
        let risk_level = Self::derive_risk_level(&domain, &intent.constraints);

        InterpretedIntent {
            goal: intent.goal.clone(),
            domain,
            required_capabilities,
            constraints: intent.constraints.clone(),
            risk_level,
        }
    }

    fn extract_capabilities(raw_input: &str, goal: &str, domain: &str) -> Vec<String> {
        let mut capabilities = Vec::new();
        let text = format!("{} {}", raw_input, goal).to_lowercase();

        if text.contains("login") || text.contains("afip") || domain == "finanzas" {
            capabilities.push("afip_login".to_string());
        }
        if text.contains("factura") || text.contains("invoice") {
            capabilities.push("http_get".to_string());
            capabilities.push("file_download".to_string());
        }
        if text.contains("drive") || text.contains("guardar") {
            capabilities.push("google_drive_upload".to_string());
        }
        if text.contains("whatsapp")
            || text.contains("avis")
            || text.contains("notificar")
            || text.contains("mensaje")
        {
            capabilities.push("whatsapp_send".to_string());
        }
        if text.contains("validar") || text.contains("check") {
            capabilities.push("form_validate".to_string());
        }

        capabilities.dedup();
        if capabilities.is_empty() {
            capabilities.push("http_ping".to_string());
        }

        capabilities
    }

    fn derive_risk_level(domain: &str, constraints: &[String]) -> String {
        if constraints.iter().any(|constraint| constraint == "legal") || domain == "finanzas" {
            "HIGH".to_string()
        } else if constraints
            .iter()
            .any(|constraint| constraint == "credenciales")
        {
            "MEDIUM".to_string()
        } else {
            "LOW".to_string()
        }
    }
}

pub struct CapabilityMapper;

impl CapabilityMapper {
    pub fn map(intent: &InterpretedIntent) -> Vec<(String, String, Option<OplValue>)> {
        intent
            .required_capabilities
            .iter()
            .enumerate()
            .map(|(index, capability)| {
                (
                    Self::capability_to_step_id(capability, index),
                    capability.clone(),
                    Self::derive_input(capability, &intent.goal),
                )
            })
            .collect()
    }

    fn capability_to_step_id(capability: &str, index: usize) -> String {
        match capability {
            "afip_login" => "login".to_string(),
            "http_get" => "check_invoices".to_string(),
            "file_download" => "download".to_string(),
            "google_drive_upload" => "upload_drive".to_string(),
            "whatsapp_send" => "notify_whatsapp".to_string(),
            "form_validate" => "validate".to_string(),
            _ => format!("step_{index}"),
        }
    }

    fn derive_input(capability: &str, goal: &str) -> Option<OplValue> {
        let goal_lower = goal.to_lowercase();

        match capability {
            "http_get" if goal_lower.contains("factur") => {
                let mut query = HashMap::new();
                query.insert(
                    "status".to_string(),
                    OplValue::String("pending".to_string()),
                );

                let mut input = HashMap::new();
                input.insert(
                    "path".to_string(),
                    OplValue::String("/api/invoices".to_string()),
                );
                input.insert("query".to_string(), OplValue::Object(query));

                Some(OplValue::Object(input))
            }
            "http_get" => Some(OplValue::String("/api/data".to_string())),
            "whatsapp_send" => Some(OplValue::String("Factura procesada".to_string())),
            _ => None,
        }
    }
}

pub struct BlueprintGenerator;

impl BlueprintGenerator {
    pub fn generate(
        intent: &InterpretedIntent,
        mapped: &[(String, String, Option<OplValue>)],
    ) -> String {
        let mut opl = String::new();

        opl.push_str(&format!("CREATE AGENT {}\n", Self::agent_id(&intent.goal)));
        opl.push_str(&format!("FOR \"{}\"\n", intent.goal));
        opl.push_str("WITH\n");
        opl.push_str(&format!("  domain = \"{}\",\n", intent.domain));
        opl.push_str(&format!(
            "  autonomy = \"{}\",\n",
            Self::autonomy(&intent.domain)
        ));
        opl.push_str(&format!("  audit = \"{}\",\n", Self::audit(&intent.domain)));
        opl.push_str(&format!("  hitl = \"{}\"\n\n", Self::hitl(&intent.domain)));

        for (step_id, capability, input) in mapped {
            opl.push_str(&format!("STEP {step_id}\n"));
            opl.push_str(&format!("  USES {capability}\n"));

            if let Some(input) = input {
                opl.push_str(&format!("  INPUT {}\n", Self::format_input(input)));
            }

            opl.push_str(&format!(
                "  RISK {}\n",
                Self::step_risk(step_id, &intent.risk_level)
            ));

            if Self::needs_hitl(step_id, &intent.domain) {
                opl.push_str("  HITL true\n");
            }

            opl.push('\n');
        }

        opl.push_str("ON timeout\n");
        opl.push_str("  FALLBACK TO login\n");
        opl.push_str("  RETRY 3\n");
        opl.push_str("  BACKOFF EXPONENTIAL\n\n");

        opl.push_str("ON invalid_data\n");
        opl.push_str("  FALLBACK TO check_invoices\n");
        opl.push_str("  RETRY 2\n");
        opl.push_str("  BACKOFF FIXED");

        opl
    }

    fn agent_id(goal: &str) -> String {
        let clean = goal
            .to_lowercase()
            .replace(' ', "_")
            .replace('á', "a")
            .replace('é', "e")
            .replace('í', "i")
            .replace('ó', "o")
            .replace('ú', "u");
        format!("{clean}_agent")
    }

    fn autonomy(domain: &str) -> &'static str {
        match domain {
            "finanzas" | "legal" | "salud" => "supervised",
            "marketing" | "scraping" => "semi",
            _ => "full",
        }
    }

    fn audit(domain: &str) -> &'static str {
        match domain {
            "finanzas" | "legal" | "salud" => "full",
            _ => "minimal",
        }
    }

    fn hitl(domain: &str) -> &'static str {
        match domain {
            "finanzas" | "legal" | "salud" => "mandatory",
            _ => "on_anomaly",
        }
    }

    fn step_risk(step_id: &str, base_risk: &str) -> String {
        if step_id == "login" || step_id == "validate" {
            "HIGH".to_string()
        } else if base_risk == "HIGH" {
            "MEDIUM".to_string()
        } else {
            "LOW".to_string()
        }
    }

    fn needs_hitl(step_id: &str, domain: &str) -> bool {
        (step_id == "login" || step_id == "validate") && (domain == "finanzas" || domain == "legal")
    }

    fn format_input(input: &OplValue) -> String {
        match input {
            OplValue::String(value) | OplValue::Identifier(value) => format!("\"{value}\""),
            OplValue::Number(value) => value.to_string(),
            OplValue::Bool(value) => value.to_string(),
            OplValue::List(values) => {
                let values = values
                    .iter()
                    .map(Self::format_input)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{values}]")
            }
            OplValue::Object(values) => {
                let mut entries = values.iter().collect::<Vec<_>>();
                entries.sort_by(|(left, _), (right, _)| left.cmp(right));
                let values = entries
                    .into_iter()
                    .map(|(key, value)| format!("{key} = {}", Self::format_input(value)))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{ {values} }}")
            }
        }
    }
}

pub struct MetaAgent;

impl MetaAgent {
    pub async fn generate(intent: HumanIntent) -> Result<MetaAgentResult, String> {
        let interpreted = IntentInterpreter::interpret(&intent);
        let mapped = CapabilityMapper::map(&interpreted);
        let opl_source = BlueprintGenerator::generate(&interpreted, &mapped);
        let ast = parse_opl(&opl_source)?;
        let package = OplTranspiler::transpile(&ast).map_err(|error| format!("{error:?}"))?;

        let simulation = SimulationEngine::run(
            &package,
            &SimulationConfig {
                agent_id: package.id.clone(),
                mode: SimulationMode::Optimistic,
                seed: None,
                max_runs: Some(1),
                failure_rate: None,
                execution_log_id: None,
            },
        )
        .await;

        let status = Self::validation_status_label(&simulation.summary.validation_status);

        Ok(MetaAgentResult {
            opl: GeneratedOpl {
                source: opl_source,
                intent_hash: format!("{:x}", md5::compute(intent.raw_input.as_bytes())),
            },
            package,
            simulation,
            status: status.to_string(),
        })
    }

    fn validation_status_label(status: &ValidationStatus) -> &'static str {
        match status {
            ValidationStatus::Approved => "APPROVED",
            ValidationStatus::Rejected => "REJECTED",
            ValidationStatus::NeedsReview => "NEEDS_REVIEW",
        }
    }
}
