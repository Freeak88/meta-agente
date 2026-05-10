#[cfg(test)]
mod contract_tests {
    use crate::api::{ApiError, CreateAgentRequest, CreateAgentResponse};
    use crate::meta_agent::HumanIntent;
    use crate::simulation::{
        SimulationMode, SimulationReport, SimulationSummary, ValidationStatus,
    };

    fn get_test_request() -> CreateAgentRequest {
        CreateAgentRequest {
            intent: HumanIntent {
                raw_input: "Cada hora, fijate si hay facturas nuevas en AFIP".to_string(),
                goal: "automatizar facturación AFIP".to_string(),
                domain: Some("finanzas".to_string()),
                constraints: vec!["legal".to_string(), "credenciales".to_string()],
                environment: Some("browser".to_string()),
                urgency: Some("alta".to_string()),
            },
        }
    }

    fn get_test_response() -> CreateAgentResponse {
        CreateAgentResponse {
            agent_id: "automatizar_facturacion_afip_agent".to_string(),
            version: "1.0.0".to_string(),
            hash: "sha256:a3f7c2...".to_string(),
            opl: "CREATE AGENT automatizar_facturacion_afip_agent\nFOR \"automatizar facturación AFIP\"".to_string(),
            simulation_report: SimulationReport {
                agent_id: "automatizar_facturacion_afip_agent".to_string(),
                mode: SimulationMode::Optimistic,
                runs: vec![],
                summary: SimulationSummary {
                    pass_rate: 1.0,
                    avg_execution_time_ms: 1_000,
                    predicted_failures: vec![],
                    risk_spikes: vec![],
                    recommendations: vec!["ok".to_string()],
                    validation_status: ValidationStatus::Approved,
                },
            },
            status: "APPROVED".to_string(),
            created_at: "2026-05-10T11:35:00Z".to_string(),
        }
    }

    #[test]
    fn test_request_serializes() {
        let request = get_test_request();
        let json = serde_json::to_string_pretty(&request).expect("debe serializar");

        assert!(json.contains("automatizar facturación AFIP"));
        assert!(json.contains("finanzas"));
        assert!(json.contains("credenciales"));
    }

    #[test]
    fn test_response_serializes() {
        let response = get_test_response();
        let json = serde_json::to_string_pretty(&response).expect("debe serializar");

        assert!(json.contains("APPROVED"));
        assert!(json.contains("1.0.0"));
        assert!(json.contains("sha256:a3f7c2..."));
        assert!(json.contains("automatizar_facturacion_afip_agent"));
    }

    #[test]
    fn test_error_serializes() {
        let error = ApiError {
            code: 422,
            message: "Agent simulation failed".to_string(),
            errors: vec!["login timeout".to_string(), "validation failed".to_string()],
        };

        let json = serde_json::to_string_pretty(&error).expect("debe serializar");
        assert!(json.contains("422"));
        assert!(json.contains("login timeout"));
        assert!(json.contains("validation failed"));
    }

    #[test]
    fn test_request_response_roundtrip() {
        let request = get_test_request();
        let request_json = serde_json::to_string(&request).expect("request debe serializar");
        let request_roundtrip: CreateAgentRequest =
            serde_json::from_str(&request_json).expect("request debe deserializar");
        assert_eq!(request, request_roundtrip);

        let response = get_test_response();
        let response_json = serde_json::to_string(&response).expect("response debe serializar");
        let response_roundtrip: CreateAgentResponse =
            serde_json::from_str(&response_json).expect("response debe deserializar");
        assert_eq!(response, response_roundtrip);
    }
}
