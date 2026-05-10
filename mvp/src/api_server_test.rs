#[cfg(test)]
mod server_tests {
    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    use crate::api::{CreateAgentRequest, CreateAgentResponse};
    use crate::api_server::router;
    use crate::meta_agent::HumanIntent;

    #[tokio::test]
    async fn test_health_check() {
        let app = router();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .expect("request debe construirse"),
            )
            .await
            .expect("server debe responder");

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_create_agent_endpoint() {
        let request = CreateAgentRequest {
            intent: HumanIntent {
                raw_input: "Cada hora, fijate si hay facturas nuevas en AFIP".to_string(),
                goal: "automatizar facturación AFIP".to_string(),
                domain: Some("finanzas".to_string()),
                constraints: vec!["legal".to_string(), "credenciales".to_string()],
                environment: Some("browser".to_string()),
                urgency: Some("alta".to_string()),
            },
        };
        let json = serde_json::to_string(&request).expect("request debe serializar");
        let app = router();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/agents")
                    .header("content-type", "application/json")
                    .body(Body::from(json))
                    .expect("request debe construirse"),
            )
            .await
            .expect("server debe responder");

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body debe leerse");
        let created: CreateAgentResponse =
            serde_json::from_slice(&body).expect("response debe deserializar");

        assert_eq!(created.agent_id, "automatizar_facturacion_afip_agent");
        assert_eq!(created.version, "1.0.0");
        assert!(created.hash.starts_with("sha256:"));
        assert_eq!(created.status, "APPROVED");
        assert!(created.opl.contains("CREATE AGENT"));
    }
}
