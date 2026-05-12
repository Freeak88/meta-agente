#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillRegistry {
    skills: HashMap<String, Skill>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub domain: String,
    pub capabilities: Vec<String>,
    pub opl_template: String,
    pub required_config: Vec<String>,
    pub rating: f64,
    pub usage_count: u64,
    pub author: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillImport {
    pub skill_id: String,
    pub version: String,
    pub overrides: HashMap<String, String>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
        }
    }

    pub fn register(&mut self, skill: Skill) {
        self.skills.insert(skill.id.clone(), skill);
    }

    pub fn get(&self, id: &str) -> Option<&Skill> {
        self.skills.get(id)
    }

    pub fn list_by_domain(&self, domain: &str) -> Vec<&Skill> {
        let mut skills: Vec<&Skill> = self
            .skills
            .values()
            .filter(|skill| skill.domain == domain)
            .collect();
        skills.sort_by(|left, right| left.id.cmp(&right.id));
        skills
    }

    pub fn search(&self, query: &str) -> Vec<&Skill> {
        let query = query.to_lowercase();
        let mut skills: Vec<&Skill> = self
            .skills
            .values()
            .filter(|skill| {
                skill.name.to_lowercase().contains(&query)
                    || skill.description.to_lowercase().contains(&query)
                    || skill.domain.to_lowercase().contains(&query)
            })
            .collect();
        skills.sort_by(|left, right| {
            right
                .rating
                .partial_cmp(&left.rating)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.id.cmp(&right.id))
        });
        skills
    }

    pub fn import_skill(&self, import: &SkillImport) -> Result<String, String> {
        let skill = self
            .get(&import.skill_id)
            .ok_or_else(|| format!("skill not found: {}", import.skill_id))?;

        if skill.version != import.version {
            return Err(format!(
                "version mismatch: expected {}, got {}",
                skill.version, import.version
            ));
        }

        let mut opl = skill.opl_template.clone();
        for (placeholder, value) in &import.overrides {
            opl = opl.replace(&format!("{{{}}}", placeholder), value);
        }

        Ok(opl)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::{parse_opl, OplAst, OplTranspiler, OplValue};
    use crate::simulation::{SimulationConfig, SimulationEngine, SimulationMode, ValidationStatus};

    fn test_registry() -> SkillRegistry {
        let mut registry = SkillRegistry::new();

        registry.register(Skill {
            id: "invoice_processing_v1".to_string(),
            name: "Invoice Processing".to_string(),
            description: "Process invoices from AFIP, validate, store in Drive".to_string(),
            version: "1.0.0".to_string(),
            domain: "finanzas".to_string(),
            capabilities: vec![
                "afip_login".to_string(),
                "http_get".to_string(),
                "file_download".to_string(),
                "google_drive_upload".to_string(),
            ],
            opl_template: r#"
STEP login
  USES afip_login
  RISK HIGH

STEP fetch_invoices
  USES http_get
  INPUT { path = "/api/invoices", query = { period = "{period}" } }

STEP download
  USES file_download

STEP upload
  USES google_drive_upload
  INPUT { folder = "{folder}" }
"#
            .to_string(),
            required_config: vec!["base_url".to_string()],
            rating: 4.5,
            usage_count: 42,
            author: "meta-agente-team".to_string(),
            created_at: "2026-05-11T00:00:00Z".to_string(),
        });

        registry.register(Skill {
            id: "email_notification_v1".to_string(),
            name: "Email Notification".to_string(),
            description: "Send email notifications via SMTP".to_string(),
            version: "1.0.0".to_string(),
            domain: "general".to_string(),
            capabilities: vec!["smtp_send".to_string()],
            opl_template: r#"
STEP notify
  USES smtp_send
  INPUT { to = "{recipient}", subject = "{subject}" }
"#
            .to_string(),
            required_config: vec!["smtp_host".to_string(), "smtp_port".to_string()],
            rating: 3.8,
            usage_count: 15,
            author: "community".to_string(),
            created_at: "2026-05-10T00:00:00Z".to_string(),
        });

        registry
    }

    fn override_to_string(value: &OplValue) -> String {
        match value {
            OplValue::String(value) | OplValue::Identifier(value) => value.clone(),
            OplValue::Number(value) => value.to_string(),
            OplValue::Bool(value) => value.to_string(),
            OplValue::List(_) | OplValue::Object(_) => {
                serde_json::to_string(value).unwrap_or_default()
            }
        }
    }

    #[test]
    fn registers_and_gets_skill() {
        let registry = test_registry();
        let skill = registry.get("invoice_processing_v1").unwrap();

        assert_eq!(skill.name, "Invoice Processing");
        assert_eq!(skill.domain, "finanzas");
        assert_eq!(skill.capabilities.len(), 4);
    }

    #[test]
    fn lists_by_domain() {
        let registry = test_registry();
        let skills = registry.list_by_domain("finanzas");

        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].id, "invoice_processing_v1");
    }

    #[test]
    fn searches_name_description_and_domain() {
        let registry = test_registry();

        assert_eq!(registry.search("invoice")[0].id, "invoice_processing_v1");
        assert_eq!(registry.search("SMTP")[0].id, "email_notification_v1");
        assert_eq!(registry.search("finanzas")[0].id, "invoice_processing_v1");
    }

    #[test]
    fn imports_skill_with_overrides() {
        let registry = test_registry();
        let mut overrides = HashMap::new();
        overrides.insert("period".to_string(), "2026-05".to_string());
        overrides.insert("folder".to_string(), "/invoices/2026".to_string());

        let opl = registry
            .import_skill(&SkillImport {
                skill_id: "invoice_processing_v1".to_string(),
                version: "1.0.0".to_string(),
                overrides,
            })
            .unwrap();

        assert!(opl.contains("2026-05"));
        assert!(opl.contains("/invoices/2026"));
        assert!(!opl.contains("{period}"));
        assert!(!opl.contains("{folder}"));
    }

    #[test]
    fn rejects_version_mismatch() {
        let registry = test_registry();
        let err = registry
            .import_skill(&SkillImport {
                skill_id: "invoice_processing_v1".to_string(),
                version: "2.0.0".to_string(),
                overrides: HashMap::new(),
            })
            .unwrap_err();

        assert!(err.contains("version mismatch"));
    }

    #[test]
    fn rejects_missing_skill() {
        let registry = test_registry();
        let err = registry
            .import_skill(&SkillImport {
                skill_id: "missing".to_string(),
                version: "1.0.0".to_string(),
                overrides: HashMap::new(),
            })
            .unwrap_err();

        assert!(err.contains("skill not found"));
    }

    #[test]
    fn parses_skill_import_in_opl() {
        let opl = r#"
IMPORT skill "invoice_processing_v1" VERSION "1.0.0"
WITH
  period = "2026-05",
  folder = "/invoices"

CREATE AGENT import_test
FOR "Import skill test"
"#;

        let ast = parse_opl(opl).unwrap();

        assert_eq!(ast.imports.len(), 1);
        assert_eq!(ast.imports[0].skill_id, "invoice_processing_v1");
        assert_eq!(ast.imports[0].version, "1.0.0");
        assert_eq!(
            ast.imports[0].overrides.get("period"),
            Some(&OplValue::String("2026-05".to_string()))
        );
    }

    #[tokio::test]
    async fn e2e_import_skill_transpile_and_simulate() {
        let opl = r#"
IMPORT skill "invoice_processing_v1" VERSION "1.0.0"
WITH
  period = "2026-05",
  folder = "/invoices"

CREATE AGENT my_agent
FOR "Procesar facturas"
"#;

        let registry = test_registry();
        let ast = parse_opl(opl).unwrap();
        let mut resolved_steps = Vec::new();

        for import in &ast.imports {
            let overrides = import
                .overrides
                .iter()
                .map(|(key, value)| (key.clone(), override_to_string(value)))
                .collect();
            let skill_opl = registry
                .import_skill(&SkillImport {
                    skill_id: import.skill_id.clone(),
                    version: import.version.clone(),
                    overrides,
                })
                .unwrap();
            let wrapped_skill_opl = format!(
                "CREATE AGENT imported_skill\nFOR \"Imported skill\"\n{}",
                skill_opl
            );
            let skill_ast = parse_opl(&wrapped_skill_opl).unwrap();
            resolved_steps.extend(skill_ast.agent.steps);
        }

        let mut agent = ast.agent.clone();
        resolved_steps.extend(agent.steps);
        agent.steps = resolved_steps;

        let package = OplTranspiler::transpile(&OplAst {
            agent,
            imports: vec![],
        })
        .unwrap();

        let report = SimulationEngine::run(
            &package,
            &SimulationConfig {
                agent_id: "my_agent".to_string(),
                mode: SimulationMode::Optimistic,
                seed: None,
                max_runs: Some(1),
                failure_rate: None,
                execution_log_id: None,
            },
        )
        .await;

        assert_eq!(package.id, "my_agent");
        assert_eq!(package.steps.len(), 4);
        assert_eq!(report.summary.validation_status, ValidationStatus::Approved);
        assert_eq!(report.summary.pass_rate, 1.0);
    }
}
