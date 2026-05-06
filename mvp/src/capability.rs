use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CapabilityConfig {
    pub timeout_ms: u64,
    pub retry_policy: RetryPolicy,
    pub headers: Option<Value>,
    pub base_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CapabilityContract {
    pub input_schema: Option<Value>,
    pub required_input: bool,
}

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub backoff_ms: u64,
}

impl Default for CapabilityConfig {
    fn default() -> Self {
        Self {
            timeout_ms: 5_000,
            retry_policy: RetryPolicy {
                max_attempts: 3,
                backoff_ms: 1_000,
            },
            headers: None,
            base_url: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ExecutionResult {
    Success(Value),
    Failure(String),
    Timeout,
}

#[async_trait]
pub trait CapabilityAdapter: Send + Sync {
    fn capability_id(&self) -> &str;

    async fn execute(&self, input: Option<Value>, config: &CapabilityConfig) -> ExecutionResult;
}

pub struct CapabilityRegistry {
    adapters: HashMap<String, Box<dyn CapabilityAdapter>>,
    configs: HashMap<String, CapabilityConfig>,
    contracts: HashMap<String, CapabilityContract>,
}

impl CapabilityRegistry {
    pub fn new() -> Self {
        Self {
            adapters: HashMap::new(),
            configs: HashMap::new(),
            contracts: HashMap::new(),
        }
    }

    pub fn register(
        &mut self,
        adapter: Box<dyn CapabilityAdapter>,
        config: Option<CapabilityConfig>,
    ) {
        let id = adapter.capability_id().to_string();
        self.adapters.insert(id.clone(), adapter);
        self.configs.insert(id.clone(), config.unwrap_or_default());
        self.contracts
            .insert(id.clone(), Self::default_contract_for(&id));
    }

    pub fn resolve(
        &self,
        capability_id: &str,
    ) -> Option<(&dyn CapabilityAdapter, &CapabilityConfig)> {
        let adapter = self.adapters.get(capability_id)?;
        let config = self.configs.get(capability_id)?;
        Some((adapter.as_ref(), config))
    }

    pub fn list(&self) -> Vec<&str> {
        let mut ids: Vec<&str> = self.adapters.keys().map(|s| s.as_str()).collect();
        ids.sort_unstable();
        ids
    }

    pub fn validate_input(&self, capability_id: &str, input: &Option<Value>) -> Result<(), String> {
        let Some(contract) = self.contracts.get(capability_id) else {
            return Err(format!("capability_not_registered: {}", capability_id));
        };

        if contract.required_input && input.is_none() {
            return Err(format!("capability {} requires input", capability_id));
        }

        if let Some(schema) = &contract.input_schema {
            Self::validate_simplified_schema(capability_id, input, schema)?;
        }

        Ok(())
    }

    fn default_contract_for(capability_id: &str) -> CapabilityContract {
        let required_input = matches!(capability_id, "http_get" | "http_post" | "db_query");
        let input_schema = match capability_id {
            "http_get" => Some(serde_json::json!({
                "types": ["string", "object"],
                "object_fields": {
                    "path": "string",
                    "query": "object"
                }
            })),
            _ => None,
        };

        CapabilityContract {
            input_schema,
            required_input,
        }
    }

    fn validate_simplified_schema(
        capability_id: &str,
        input: &Option<Value>,
        schema: &Value,
    ) -> Result<(), String> {
        let Some(input) = input else {
            return Ok(());
        };

        let allowed_types = schema
            .get("types")
            .and_then(|types| types.as_array())
            .map(|types| {
                types
                    .iter()
                    .filter_map(|value| value.as_str())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let input_type = match input {
            Value::String(_) => "string",
            Value::Object(_) => "object",
            Value::Array(_) => "array",
            Value::Bool(_) => "boolean",
            Value::Number(_) => "number",
            Value::Null => "null",
        };

        if !allowed_types.is_empty() && !allowed_types.contains(&input_type) {
            return Err(format!(
                "invalid_input: capability {} expected {:?}, got {}",
                capability_id, allowed_types, input_type
            ));
        }

        if capability_id == "http_get" {
            Self::validate_http_get_input(input)?;
        }

        Ok(())
    }

    fn validate_http_get_input(input: &Value) -> Result<(), String> {
        let Value::Object(map) = input else {
            return Ok(());
        };

        if let Some(path) = map.get("path") {
            if !path.is_string() {
                return Err("invalid_input: http_get path must be string".to_string());
            }
        }

        if let Some(query) = map.get("query") {
            if !query.is_object() {
                return Err("invalid_input: http_get query must be object".to_string());
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestAdapter;

    #[async_trait]
    impl CapabilityAdapter for TestAdapter {
        fn capability_id(&self) -> &str {
            "test_capability"
        }

        async fn execute(
            &self,
            _input: Option<Value>,
            _config: &CapabilityConfig,
        ) -> ExecutionResult {
            ExecutionResult::Success(serde_json::json!({"ok": true}))
        }
    }

    #[test]
    fn registers_and_resolves_adapter() {
        let mut registry = CapabilityRegistry::new();
        registry.register(Box::new(TestAdapter), None);

        let resolved = registry.resolve("test_capability");

        assert!(resolved.is_some());
        assert_eq!(registry.list(), vec!["test_capability"]);
    }

    #[test]
    fn returns_none_for_unknown_capability() {
        let registry = CapabilityRegistry::new();

        assert!(registry.resolve("missing").is_none());
    }

    #[test]
    fn validates_required_input() {
        struct HttpAdapter;

        #[async_trait]
        impl CapabilityAdapter for HttpAdapter {
            fn capability_id(&self) -> &str {
                "http_get"
            }

            async fn execute(
                &self,
                _input: Option<Value>,
                _config: &CapabilityConfig,
            ) -> ExecutionResult {
                ExecutionResult::Success(serde_json::json!({}))
            }
        }

        let mut registry = CapabilityRegistry::new();
        registry.register(Box::new(HttpAdapter), None);

        let err = registry.validate_input("http_get", &None).unwrap_err();

        assert!(err.contains("requires input"));
    }

    #[test]
    fn validates_http_get_object_shape() {
        struct HttpAdapter;

        #[async_trait]
        impl CapabilityAdapter for HttpAdapter {
            fn capability_id(&self) -> &str {
                "http_get"
            }

            async fn execute(
                &self,
                _input: Option<Value>,
                _config: &CapabilityConfig,
            ) -> ExecutionResult {
                ExecutionResult::Success(serde_json::json!({}))
            }
        }

        let mut registry = CapabilityRegistry::new();
        registry.register(Box::new(HttpAdapter), None);
        let input = Some(serde_json::json!({"path": 42}));

        let err = registry.validate_input("http_get", &input).unwrap_err();

        assert!(err.contains("path must be string"));
    }
}
