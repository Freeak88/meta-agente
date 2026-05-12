#![allow(dead_code)]

use async_trait::async_trait;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

use crate::mcp::{McpError, McpRequest, McpResponse, McpTransport};

pub struct StdioTransport {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl StdioTransport {
    pub async fn spawn(command: &str, args: &[String]) -> Result<Self, String> {
        let mut child = Command::new(command)
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|err| format!("failed_to_spawn_mcp_server: {}", err))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "mcp_server_missing_stdin".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "mcp_server_missing_stdout".to_string())?;

        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
        })
    }

    async fn write_request(&mut self, request: &McpRequest) -> Result<(), String> {
        let mut line = serde_json::to_string(request)
            .map_err(|err| format!("mcp_request_encode_error: {}", err))?;
        line.push('\n');

        self.stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|err| format!("mcp_stdio_write_error: {}", err))?;
        self.stdin
            .flush()
            .await
            .map_err(|err| format!("mcp_stdio_flush_error: {}", err))
    }

    async fn read_response(&mut self) -> Result<McpResponse, String> {
        let mut line = String::new();
        let bytes_read = self
            .stdout
            .read_line(&mut line)
            .await
            .map_err(|err| format!("mcp_stdio_read_error: {}", err))?;

        if bytes_read == 0 {
            return Err("mcp_stdio_eof".to_string());
        }

        serde_json::from_str(line.trim())
            .map_err(|err| format!("mcp_response_decode_error: {}", err))
    }

    pub async fn shutdown(mut self) -> Result<(), String> {
        self.child
            .kill()
            .await
            .map_err(|err| format!("mcp_stdio_kill_error: {}", err))
    }
}

#[async_trait]
impl McpTransport for StdioTransport {
    async fn send(&mut self, request: McpRequest) -> Result<McpResponse, String> {
        let expected_id = request.id;
        self.write_request(&request).await?;
        let response = self.read_response().await?;

        if response.id != expected_id {
            return Err(format!(
                "mcp_response_id_mismatch: expected {}, got {}",
                expected_id, response.id
            ));
        }

        Ok(response)
    }
}

pub async fn run_mock_stdio_server() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin);
    let mut writer = stdout;

    loop {
        let mut line = String::new();
        let Ok(bytes_read) = reader.read_line(&mut line).await else {
            break;
        };
        if bytes_read == 0 {
            break;
        }

        let response = match serde_json::from_str::<McpRequest>(line.trim()) {
            Ok(request) => mock_response(request),
            Err(err) => McpResponse {
                jsonrpc: "2.0".to_string(),
                id: 0,
                result: None,
                error: Some(McpError {
                    code: -32700,
                    message: format!("parse_error: {}", err),
                }),
            },
        };

        let Ok(mut response_line) = serde_json::to_string(&response) else {
            break;
        };
        response_line.push('\n');

        if writer.write_all(response_line.as_bytes()).await.is_err() {
            break;
        }
        if writer.flush().await.is_err() {
            break;
        }
    }
}

fn mock_response(request: McpRequest) -> McpResponse {
    match request.method.as_str() {
        "initialize" => ok(
            request.id,
            serde_json::json!({
                "protocolVersion": "2024-11-05",
                "serverInfo": {
                    "name": "mock-stdio-mcp",
                    "version": "0.1.0"
                },
                "capabilities": {
                    "tools": {}
                }
            }),
        ),
        "tools/list" => ok(
            request.id,
            serde_json::json!({
                "tools": [
                    {
                        "name": "echo",
                        "description": "Echoes text over stdio",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "text": {"type": "string"}
                            }
                        }
                    }
                ]
            }),
        ),
        "tools/call" => {
            let text = request
                .params
                .as_ref()
                .and_then(|params| params.get("arguments"))
                .and_then(|arguments| arguments.get("text"))
                .and_then(Value::as_str)
                .unwrap_or_default();

            ok(
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
            )
        }
        _ => McpResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: None,
            error: Some(McpError {
                code: -32601,
                message: "method_not_found".to_string(),
            }),
        },
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

#[cfg(test)]
mod tests {
    use crate::mcp::McpClient;

    fn mock_stdio_command() -> (String, Vec<String>) {
        if cfg!(windows) {
            let script = r#"
while (($line = [Console]::In.ReadLine()) -ne $null) {
  $req = $line | ConvertFrom-Json
  if ($req.method -eq "initialize") {
    $result = @{
      protocolVersion = "2024-11-05"
      serverInfo = @{ name = "mock-stdio-mcp"; version = "0.1.0" }
      capabilities = @{ tools = @{} }
    }
    $resp = @{ jsonrpc = "2.0"; id = $req.id; result = $result }
  } elseif ($req.method -eq "tools/list") {
    $tool = @{
      name = "echo"
      description = "Echoes text over stdio"
      inputSchema = @{ type = "object"; properties = @{ text = @{ type = "string" } } }
    }
    $resp = @{ jsonrpc = "2.0"; id = $req.id; result = @{ tools = @($tool) } }
  } elseif ($req.method -eq "tools/call") {
    $text = $req.params.arguments.text
    $resp = @{
      jsonrpc = "2.0"
      id = $req.id
      result = @{ content = @(@{ type = "text"; text = $text }); isError = $false }
    }
  } else {
    $resp = @{ jsonrpc = "2.0"; id = $req.id; error = @{ code = -32601; message = "method_not_found" } }
  }
  $resp | ConvertTo-Json -Depth 20 -Compress
  [Console]::Out.Flush()
}
"#;
            (
                "powershell".to_string(),
                vec![
                    "-NoProfile".to_string(),
                    "-Command".to_string(),
                    script.to_string(),
                ],
            )
        } else {
            let script = r#"
import json, sys
for line in sys.stdin:
    req = json.loads(line)
    method = req.get("method")
    if method == "initialize":
        resp = {"jsonrpc":"2.0","id":req["id"],"result":{"protocolVersion":"2024-11-05","serverInfo":{"name":"mock-stdio-mcp","version":"0.1.0"},"capabilities":{"tools":{}}}}
    elif method == "tools/list":
        resp = {"jsonrpc":"2.0","id":req["id"],"result":{"tools":[{"name":"echo","description":"Echoes text over stdio","inputSchema":{"type":"object","properties":{"text":{"type":"string"}}}}]}}
    elif method == "tools/call":
        text = req.get("params", {}).get("arguments", {}).get("text", "")
        resp = {"jsonrpc":"2.0","id":req["id"],"result":{"content":[{"type":"text","text":text}],"isError":False}}
    else:
        resp = {"jsonrpc":"2.0","id":req["id"],"error":{"code":-32601,"message":"method_not_found"}}
    print(json.dumps(resp), flush=True)
"#;
            (
                "python3".to_string(),
                vec!["-c".to_string(), script.to_string()],
            )
        }
    }

    #[tokio::test]
    async fn stdio_transport_lists_tools_from_process() {
        let (command, args) = mock_stdio_command();
        let mut client = McpClient::with_stdio(&command, &args).await.unwrap();

        let tools = client.list_tools().await.unwrap();

        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "echo");
    }

    #[tokio::test]
    async fn stdio_transport_executes_tool_from_process() {
        let (command, args) = mock_stdio_command();
        let mut client = McpClient::with_stdio(&command, &args).await.unwrap();

        let result = client
            .call_tool("echo", Some(serde_json::json!({"text": "stdio ok"})))
            .await
            .unwrap();

        assert_eq!(result.content.len(), 1);
        assert_eq!(result.content[0].text, "stdio ok");
    }
}
