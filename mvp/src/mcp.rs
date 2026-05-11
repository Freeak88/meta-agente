#![allow(dead_code)]

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl McpRequest {
    fn new(id: u64, method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.into(),
            params,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpResponse {
    pub jsonrpc: String,
    pub id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<McpError>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpError {
    pub code: i64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct McpTool {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub input_schema: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpTextContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct McpToolCallResult {
    pub content: Vec<McpTextContent>,
    #[serde(default)]
    pub is_error: bool,
}

#[async_trait]
pub trait McpTransport: Send {
    async fn send(&mut self, request: McpRequest) -> Result<McpResponse, String>;
}

pub struct McpClient<T>
where
    T: McpTransport,
{
    transport: T,
    next_id: u64,
}

impl<T> McpClient<T>
where
    T: McpTransport,
{
    pub async fn connect(mut transport: T) -> Result<Self, String> {
        let request = McpRequest::new(
            1,
            "initialize",
            Some(serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "meta-agente",
                    "version": "1.1.0"
                }
            })),
        );

        let response = transport.send(request).await?;
        Self::ensure_ok(response)?;

        Ok(Self {
            transport,
            next_id: 2,
        })
    }

    pub async fn list_tools(&mut self) -> Result<Vec<McpTool>, String> {
        let request = self.request("tools/list", None);
        let response = self.transport.send(request).await?;
        let result = Self::ensure_ok(response)?;
        let tools = result
            .get("tools")
            .cloned()
            .ok_or_else(|| "mcp_response_missing_tools".to_string())?;

        serde_json::from_value(tools).map_err(|err| format!("mcp_tools_decode_error: {}", err))
    }

    pub async fn call_tool(
        &mut self,
        name: &str,
        arguments: Option<Value>,
    ) -> Result<McpToolCallResult, String> {
        let request = self.request(
            "tools/call",
            Some(serde_json::json!({
                "name": name,
                "arguments": arguments.unwrap_or(Value::Object(Default::default()))
            })),
        );
        let response = self.transport.send(request).await?;
        let result = Self::ensure_ok(response)?;

        serde_json::from_value(result)
            .map_err(|err| format!("mcp_tool_result_decode_error: {}", err))
    }

    fn request(&mut self, method: &str, params: Option<Value>) -> McpRequest {
        let id = self.next_id;
        self.next_id += 1;
        McpRequest::new(id, method, params)
    }

    fn ensure_ok(response: McpResponse) -> Result<Value, String> {
        if let Some(error) = response.error {
            return Err(format!("mcp_error {}: {}", error.code, error.message));
        }

        response
            .result
            .ok_or_else(|| "mcp_response_missing_result".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    struct MockMcpServer {
        tools: Vec<McpTool>,
        seen_methods: VecDeque<String>,
    }

    impl MockMcpServer {
        fn new() -> Self {
            Self {
                tools: vec![McpTool {
                    name: "echo".to_string(),
                    description: Some("Echoes the provided text".to_string()),
                    input_schema: Some(serde_json::json!({
                        "type": "object",
                        "properties": {
                            "text": {"type": "string"}
                        }
                    })),
                }],
                seen_methods: VecDeque::new(),
            }
        }

        fn ok(id: u64, result: Value) -> McpResponse {
            McpResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(result),
                error: None,
            }
        }

        fn err(id: u64, code: i64, message: &str) -> McpResponse {
            McpResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(McpError {
                    code,
                    message: message.to_string(),
                }),
            }
        }
    }

    #[async_trait]
    impl McpTransport for MockMcpServer {
        async fn send(&mut self, request: McpRequest) -> Result<McpResponse, String> {
            self.seen_methods.push_back(request.method.clone());

            match request.method.as_str() {
                "initialize" => Ok(Self::ok(
                    request.id,
                    serde_json::json!({
                        "protocolVersion": "2024-11-05",
                        "serverInfo": {
                            "name": "mock-mcp",
                            "version": "0.1.0"
                        },
                        "capabilities": {
                            "tools": {}
                        }
                    }),
                )),
                "tools/list" => Ok(Self::ok(
                    request.id,
                    serde_json::json!({
                        "tools": self.tools
                    }),
                )),
                "tools/call" => {
                    let params = request.params.unwrap_or(Value::Null);
                    let name = params
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or_default();

                    if name != "echo" {
                        return Ok(Self::err(request.id, -32602, "tool_not_found"));
                    }

                    let text = params
                        .get("arguments")
                        .and_then(|arguments| arguments.get("text"))
                        .and_then(Value::as_str)
                        .unwrap_or_default();

                    Ok(Self::ok(
                        request.id,
                        serde_json::json!({
                            "content": [
                                {
                                    "type": "text",
                                    "text": text
                                }
                            ],
                            "isError": false
                        }),
                    ))
                }
                _ => Ok(Self::err(request.id, -32601, "method_not_found")),
            }
        }
    }

    #[tokio::test]
    async fn mcp_client_connects_to_mock_server() {
        let server = MockMcpServer::new();
        let client = McpClient::connect(server).await;

        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn mcp_client_lists_tools() {
        let server = MockMcpServer::new();
        let mut client = McpClient::connect(server).await.unwrap();

        let tools = client.list_tools().await.unwrap();

        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "echo");
        assert_eq!(
            tools[0].description,
            Some("Echoes the provided text".to_string())
        );
    }

    #[tokio::test]
    async fn mcp_client_executes_tool() {
        let server = MockMcpServer::new();
        let mut client = McpClient::connect(server).await.unwrap();

        let result = client
            .call_tool("echo", Some(serde_json::json!({"text": "hola mcp"})))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert_eq!(result.content.len(), 1);
        assert_eq!(result.content[0].content_type, "text");
        assert_eq!(result.content[0].text, "hola mcp");
    }

    #[tokio::test]
    async fn mcp_client_returns_server_errors() {
        let server = MockMcpServer::new();
        let mut client = McpClient::connect(server).await.unwrap();

        let err = client.call_tool("missing", None).await.unwrap_err();

        assert!(err.contains("tool_not_found"));
    }
}
