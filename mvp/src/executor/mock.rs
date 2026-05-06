use crate::capability::{CapabilityAdapter, CapabilityConfig, CapabilityRegistry, ExecutionResult};
use async_trait::async_trait;
use serde_json::Value;
use tokio::time::{sleep, Duration};

pub struct MockHttpPing;

#[async_trait]
impl CapabilityAdapter for MockHttpPing {
    fn capability_id(&self) -> &str {
        "http_ping"
    }

    async fn execute(&self, _input: Option<Value>, config: &CapabilityConfig) -> ExecutionResult {
        sleep(Duration::from_millis(config.timeout_ms.min(50))).await;
        if config.timeout_ms == 0 {
            return ExecutionResult::Timeout;
        }

        ExecutionResult::Success(serde_json::json!({
            "status": "ok",
            "latency_ms": 45
        }))
    }
}

pub struct MockHttpValidate {
    succeeds: bool,
}

impl MockHttpValidate {
    pub fn new(succeeds: bool) -> Self {
        Self { succeeds }
    }
}

#[async_trait]
impl CapabilityAdapter for MockHttpValidate {
    fn capability_id(&self) -> &str {
        "http_validate"
    }

    async fn execute(&self, _input: Option<Value>, config: &CapabilityConfig) -> ExecutionResult {
        sleep(Duration::from_millis(config.timeout_ms.min(50))).await;
        if config.timeout_ms == 0 {
            return ExecutionResult::Timeout;
        }

        if self.succeeds {
            ExecutionResult::Success(serde_json::json!({
                "valid": true,
                "checks_passed": 3
            }))
        } else {
            ExecutionResult::Failure("validation_failed".to_string())
        }
    }
}

pub struct MockHttpGet;

#[async_trait]
impl CapabilityAdapter for MockHttpGet {
    fn capability_id(&self) -> &str {
        "http_get"
    }

    async fn execute(&self, input: Option<Value>, config: &CapabilityConfig) -> ExecutionResult {
        sleep(Duration::from_millis(config.timeout_ms.min(100))).await;
        if config.timeout_ms == 0 {
            return ExecutionResult::Timeout;
        }

        let url = match input.as_ref() {
            Some(Value::String(url)) => url.clone(),
            Some(Value::Object(map)) => map
                .get("path")
                .and_then(|value| value.as_str())
                .unwrap_or("unknown")
                .to_string(),
            _ => "unknown".to_string(),
        };
        ExecutionResult::Success(serde_json::json!({
            "mock": true,
            "url": url,
            "status": 200,
            "headers": {
                "Host": "mock.local",
                "User-Agent": "operant-mvp/mock"
            },
            "body": "{\"mocked\": true}"
        }))
    }
}

pub fn register_mocks(registry: &mut CapabilityRegistry, resume_mode: bool) {
    registry.register(Box::new(MockHttpPing), None);
    registry.register(Box::new(MockHttpValidate::new(resume_mode)), None);
}

pub fn register_mock_http_get(registry: &mut CapabilityRegistry) {
    registry.register(Box::new(MockHttpGet), None);
}
