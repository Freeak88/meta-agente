#[cfg(test)]
mod generator_tests {
    use crate::meta_agent::{
        BlueprintGenerator, CapabilityMapper, HumanIntent, IntentInterpreter, MetaAgent,
    };

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

    #[test]
    fn test_intent_interpreter() {
        let intent = get_test_intent();
        let interpreted = IntentInterpreter::interpret(&intent);

        assert_eq!(interpreted.domain, "finanzas");
        assert_eq!(interpreted.risk_level, "HIGH");
        assert!(interpreted
            .required_capabilities
            .contains(&"afip_login".to_string()));
        assert!(interpreted
            .required_capabilities
            .contains(&"http_get".to_string()));
        assert!(interpreted
            .required_capabilities
            .contains(&"google_drive_upload".to_string()));
        assert!(interpreted
            .required_capabilities
            .contains(&"whatsapp_send".to_string()));
    }

    #[test]
    fn test_capability_mapper() {
        let intent = get_test_intent();
        let interpreted = IntentInterpreter::interpret(&intent);
        let mapped = CapabilityMapper::map(&interpreted);

        assert!(!mapped.is_empty());
        assert!(mapped
            .iter()
            .any(|(id, capability, _)| id == "login" && capability == "afip_login"));
        assert!(mapped
            .iter()
            .any(|(id, capability, input)| id == "check_invoices"
                && capability == "http_get"
                && input.is_some()));
    }

    #[test]
    fn test_blueprint_generator() {
        let intent = get_test_intent();
        let interpreted = IntentInterpreter::interpret(&intent);
        let mapped = CapabilityMapper::map(&interpreted);
        let opl = BlueprintGenerator::generate(&interpreted, &mapped);

        assert!(opl.contains("CREATE AGENT automatizar_facturacion_afip_agent"));
        assert!(opl.contains("STEP login"));
        assert!(opl.contains("USES afip_login"));
        assert!(opl.contains("STEP check_invoices"));
        assert!(
            opl.contains("INPUT { path = \"/api/invoices\", query = { status = \"pending\" } }")
        );
        assert!(opl.contains("RISK HIGH"));
        assert!(opl.contains("HITL true"));
        assert!(opl.contains("ON timeout"));
        assert!(opl.contains("BACKOFF EXPONENTIAL"));
    }

    #[test]
    fn test_blueprint_parses_and_transpiles() {
        let intent = get_test_intent();
        let interpreted = IntentInterpreter::interpret(&intent);
        let mapped = CapabilityMapper::map(&interpreted);
        let opl = BlueprintGenerator::generate(&interpreted, &mapped);
        let ast = crate::dsl::parse_opl(&opl).expect("OPL generado debe parsear");
        let package =
            crate::dsl::OplTranspiler::transpile(&ast).expect("OPL generado debe transpilar");

        assert_eq!(package.id, "automatizar_facturacion_afip_agent");
        assert_eq!(package.steps.len(), 5);
    }

    #[tokio::test]
    async fn test_meta_agent_end_to_end() {
        let intent = get_test_intent();
        let result = MetaAgent::generate(intent).await.expect("debe generar");

        assert_eq!(result.status, "APPROVED");
        assert_eq!(result.package.id, "automatizar_facturacion_afip_agent");
        assert!(result.opl.source.contains("CREATE AGENT"));
        assert!(result.simulation.summary.pass_rate > 0.0);
    }

    #[tokio::test]
    async fn test_meta_agent_deterministic() {
        let first = MetaAgent::generate(get_test_intent())
            .await
            .expect("debe generar");
        let second = MetaAgent::generate(get_test_intent())
            .await
            .expect("debe generar");

        assert_eq!(first.opl.source, second.opl.source);
        assert_eq!(first.opl.intent_hash, second.opl.intent_hash);
    }
}
