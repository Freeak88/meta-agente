#[cfg(test)]
mod contract_tests {
    use crate::dsl::{parse_opl, OplTranspiler, OplValue};
    use crate::meta_agent::{GeneratedOpl, HumanIntent, InterpretedIntent, MetaAgentResult};
    use crate::simulation::{SimulationConfig, SimulationEngine, SimulationMode, ValidationStatus};

    fn get_test_intent() -> HumanIntent {
        HumanIntent {
            raw_input: "Cada hora, fijate si hay facturas nuevas en AFIP, descargalas, guardalas en Drive, y avisame por WhatsApp si hay algo raro".to_string(),
            goal: "automatizar facturación AFIP".to_string(),
            domain: Some("finanzas".to_string()),
            constraints: vec!["legal".to_string(), "credenciales".to_string()],
            environment: Some("browser".to_string()),
            urgency: Some("alta".to_string()),
        }
    }

    fn get_expected_opl() -> &'static str {
        r#"CREATE AGENT afip_invoice_processor
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
  BACKOFF FIXED"#
    }

    #[test]
    fn test_human_intent_contract() {
        let intent = get_test_intent();

        assert_eq!(intent.goal, "automatizar facturación AFIP");
        assert_eq!(intent.domain, Some("finanzas".to_string()));
        assert_eq!(intent.constraints, vec!["legal", "credenciales"]);
        assert_eq!(intent.environment, Some("browser".to_string()));
        assert_eq!(intent.urgency, Some("alta".to_string()));
    }

    #[test]
    fn test_interpreted_intent_contract_shape() {
        let interpreted = InterpretedIntent {
            goal: "automatizar facturación AFIP".to_string(),
            domain: "finanzas".to_string(),
            required_capabilities: vec![
                "afip_login".to_string(),
                "http_get".to_string(),
                "file_download".to_string(),
                "google_drive_upload".to_string(),
                "whatsapp_send".to_string(),
            ],
            constraints: vec!["legal".to_string(), "credenciales".to_string()],
            risk_level: "HIGH".to_string(),
        };

        assert_eq!(interpreted.required_capabilities.len(), 5);
        assert!(interpreted
            .required_capabilities
            .contains(&"afip_login".to_string()));
        assert_eq!(interpreted.risk_level, "HIGH");
    }

    #[test]
    fn test_expected_opl_parses() {
        let ast = parse_opl(get_expected_opl()).expect("OPL esperado debe parsear");

        assert_eq!(ast.agent.id, "afip_invoice_processor");
        assert_eq!(ast.agent.mission, "automatizar facturación AFIP");
        assert_eq!(
            ast.agent.properties.get("domain"),
            Some(&OplValue::String("finanzas".to_string()))
        );
        assert_eq!(ast.agent.steps.len(), 5);
        assert_eq!(ast.agent.fallbacks.len(), 2);

        let check_invoices = ast
            .agent
            .steps
            .iter()
            .find(|step| step.id == "check_invoices")
            .expect("check_invoices debe existir");

        match check_invoices.input.as_ref() {
            Some(OplValue::Object(input)) => {
                assert_eq!(
                    input.get("path"),
                    Some(&OplValue::String("/api/invoices".to_string()))
                );
                assert!(matches!(input.get("query"), Some(OplValue::Object(_))));
            }
            other => panic!("INPUT debe parsear como objeto, fue {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_expected_opl_simulates_optimistic() {
        let ast = parse_opl(get_expected_opl()).expect("debe parsear");
        let package = OplTranspiler::transpile(&ast).expect("debe transpilar");

        let report = SimulationEngine::run(
            &package,
            &SimulationConfig {
                agent_id: "afip_invoice_processor".to_string(),
                mode: SimulationMode::Optimistic,
                seed: None,
                max_runs: Some(1),
                failure_rate: None,
                execution_log_id: None,
            },
        )
        .await;

        assert_eq!(report.summary.validation_status, ValidationStatus::Approved);
        assert_eq!(report.summary.pass_rate, 1.0);
        assert!(report.summary.predicted_failures.is_empty());
    }

    #[tokio::test]
    async fn test_expected_opl_simulates_adversarial() {
        let ast = parse_opl(get_expected_opl()).expect("debe parsear");
        let package = OplTranspiler::transpile(&ast).expect("debe transpilar");

        let report = SimulationEngine::run(
            &package,
            &SimulationConfig {
                agent_id: "afip_invoice_processor".to_string(),
                mode: SimulationMode::Adversarial,
                seed: None,
                max_runs: Some(1),
                failure_rate: None,
                execution_log_id: None,
            },
        )
        .await;

        assert_eq!(report.summary.validation_status, ValidationStatus::Rejected);
        assert_eq!(report.summary.pass_rate, 0.0);
        assert!(report
            .summary
            .predicted_failures
            .iter()
            .any(|failure| failure.step_id == "login" && failure.probability == 1.0));
    }

    #[tokio::test]
    async fn test_expected_opl_simulates_stochastic_needs_review() {
        let ast = parse_opl(get_expected_opl()).expect("debe parsear");
        let package = OplTranspiler::transpile(&ast).expect("debe transpilar");

        let report = SimulationEngine::run(
            &package,
            &SimulationConfig {
                agent_id: "afip_invoice_processor".to_string(),
                mode: SimulationMode::Stochastic,
                seed: Some(42),
                max_runs: Some(100),
                failure_rate: Some(0.2),
                execution_log_id: None,
            },
        )
        .await;

        assert_eq!(
            report.summary.validation_status,
            ValidationStatus::NeedsReview
        );
        assert!(report.summary.pass_rate > 0.0 && report.summary.pass_rate < 1.0);
        assert_eq!(report.runs.len(), 100);
    }

    #[tokio::test]
    async fn test_meta_agent_result_struct() {
        let ast = parse_opl(get_expected_opl()).expect("debe parsear");
        let package = OplTranspiler::transpile(&ast).expect("debe transpilar");
        let simulation = SimulationEngine::run(
            &package,
            &SimulationConfig {
                agent_id: "afip_invoice_processor".to_string(),
                mode: SimulationMode::Optimistic,
                seed: None,
                max_runs: Some(1),
                failure_rate: None,
                execution_log_id: None,
            },
        )
        .await;

        let result = MetaAgentResult {
            opl: GeneratedOpl {
                source: get_expected_opl().to_string(),
                intent_hash: "abc123".to_string(),
            },
            package,
            simulation,
            status: "APPROVED".to_string(),
        };

        assert_eq!(result.status, "APPROVED");
        assert!(result.opl.source.contains("afip_invoice_processor"));
        assert_eq!(result.package.id, "afip_invoice_processor");
        assert_eq!(
            result.simulation.summary.validation_status,
            ValidationStatus::Approved
        );
    }
}
