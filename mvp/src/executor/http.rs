use crate::capability::{CapabilityAdapter, CapabilityConfig, ExecutionResult};
use async_trait::async_trait;
use reqwest::Url;
use serde_json::{Map, Value};
use std::time::Duration;
use tokio::time::timeout;

pub struct HttpGetAdapter {
    registry_base_url: Option<String>,
}

impl HttpGetAdapter {
    pub fn new(base_url: Option<String>) -> Self {
        Self {
            registry_base_url: base_url.map(|url| url.trim_end_matches('/').to_string()),
        }
    }

    fn resolve_url(
        &self,
        input: Option<Value>,
        config: &CapabilityConfig,
    ) -> Result<String, String> {
        let (url, query) = match input {
            Some(Value::String(url)) => (url, None),
            Some(Value::Object(mut map)) => {
                let path = map
                    .remove("path")
                    .and_then(|value| value.as_str().map(ToString::to_string))
                    .unwrap_or_else(|| "/get".to_string());
                let query = map.remove("query").and_then(|value| match value {
                    Value::Object(query) => Some(query),
                    _ => None,
                });
                (path, query)
            }
            Some(other) => {
                return Err(format!(
                    "invalid_input: expected string URL or object, got {}",
                    other
                ))
            }
            None => return Err("missing_input: URL required".to_string()),
        };

        let base_url = config
            .base_url
            .as_deref()
            .or(self.registry_base_url.as_deref())
            .unwrap_or("https://httpbin.org")
            .trim_end_matches('/');

        let full_url = if url.starts_with("http://") || url.starts_with("https://") {
            url
        } else if url.starts_with('/') {
            format!("{}{}", base_url, url)
        } else {
            format!("{}/{}", base_url, url)
        };

        Self::append_query(full_url, query)
    }

    fn append_query(url: String, query: Option<Map<String, Value>>) -> Result<String, String> {
        let Some(query) = query else {
            return Ok(url);
        };
        let mut url = Url::parse(&url).map_err(|err| format!("invalid_url: {}", err))?;

        for (key, value) in query {
            let value = match value {
                Value::String(value) => value,
                Value::Number(value) => value.to_string(),
                Value::Bool(value) => value.to_string(),
                other => other.to_string(),
            };
            url.query_pairs_mut().append_pair(&key, &value);
        }

        Ok(url.to_string())
    }
}

#[async_trait]
impl CapabilityAdapter for HttpGetAdapter {
    fn capability_id(&self) -> &str {
        "http_get"
    }

    async fn execute(&self, input: Option<Value>, config: &CapabilityConfig) -> ExecutionResult {
        let full_url = match self.resolve_url(input, config) {
            Ok(url) => url,
            Err(err) => return ExecutionResult::Failure(err),
        };

        let client = match reqwest::Client::builder()
            .timeout(Duration::from_millis(config.timeout_ms))
            .build()
        {
            Ok(client) => client,
            Err(err) => return ExecutionResult::Failure(format!("client_error: {}", err)),
        };
        let duration = Duration::from_millis(config.timeout_ms);

        let mut request = client.get(&full_url);
        if let Some(headers) = &config.headers {
            if let Some(headers) = headers.as_object() {
                for (key, value) in headers {
                    if let Some(value) = value.as_str() {
                        request = request.header(key, value);
                    }
                }
            }
        }

        match timeout(duration, request.send()).await {
            Ok(Ok(response)) => {
                let status = response.status();
                let body = match response.text().await {
                    Ok(body) => body,
                    Err(err) => {
                        return ExecutionResult::Failure(format!("body_read_error: {}", err))
                    }
                };

                if status.is_success() {
                    match serde_json::from_str::<Value>(&body) {
                        Ok(json) => ExecutionResult::Success(json),
                        Err(_) => ExecutionResult::Success(Value::String(body)),
                    }
                } else {
                    ExecutionResult::Failure(format!("http_error: {} - {}", status.as_u16(), body))
                }
            }
            Ok(Err(err)) => ExecutionResult::Failure(format!("request_error: {}", err)),
            Err(_) => ExecutionResult::Timeout,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_relative_url_against_base() {
        let adapter = HttpGetAdapter::new(Some("https://example.test/api".to_string()));

        let url = adapter
            .resolve_url(
                Some(Value::String("/status".to_string())),
                &CapabilityConfig::default(),
            )
            .unwrap();

        assert_eq!(url, "https://example.test/api/status");
    }

    #[test]
    fn keeps_absolute_url() {
        let adapter = HttpGetAdapter::new(Some("https://example.test".to_string()));

        let url = adapter
            .resolve_url(
                Some(Value::String("https://other.test/get".to_string())),
                &CapabilityConfig::default(),
            )
            .unwrap();

        assert_eq!(url, "https://other.test/get");
    }

    #[test]
    fn resolves_object_input_with_query() {
        let adapter = HttpGetAdapter::new(Some("https://example.test".to_string()));

        let url = adapter
            .resolve_url(
                Some(serde_json::json!({
                    "path": "/json",
                    "query": {"foo": "bar", "n": 7}
                })),
                &CapabilityConfig::default(),
            )
            .unwrap();

        assert_eq!(url, "https://example.test/json?foo=bar&n=7");
    }

    #[tokio::test]
    async fn rejects_missing_input() {
        let adapter = HttpGetAdapter::new(Some("https://example.test".to_string()));

        match adapter.execute(None, &CapabilityConfig::default()).await {
            ExecutionResult::Failure(err) => assert!(err.contains("missing_input")),
            _ => panic!("expected missing input failure"),
        }
    }

    #[test]
    fn config_base_url_overrides_registry_base_url() {
        let adapter = HttpGetAdapter::new(Some("https://registry.test".to_string()));
        let config = CapabilityConfig {
            base_url: Some("https://agent.test".to_string()),
            ..CapabilityConfig::default()
        };

        let url = adapter
            .resolve_url(Some(Value::String("/get".to_string())), &config)
            .unwrap();

        assert_eq!(url, "https://agent.test/get");
    }
}
