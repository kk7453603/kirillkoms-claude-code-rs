use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;

use crate::types::*;

#[async_trait]
pub trait McpClient: Send + Sync {
    async fn list_tools(&self) -> Result<Vec<McpToolDefinition>, McpError>;
    async fn call_tool(
        &self,
        name: &str,
        input: serde_json::Value,
    ) -> Result<McpToolResult, McpError>;
    async fn list_resources(&self) -> Result<Vec<McpResource>, McpError>;
    async fn read_resource(&self, uri: &str) -> Result<serde_json::Value, McpError>;
}

#[derive(Debug, thiserror::Error)]
pub enum McpError {
    #[error("Connection error: {0}")]
    Connection(String),
    #[error("Protocol error: {0}")]
    Protocol(String),
    #[error("Tool not found: {0}")]
    ToolNotFound(String),
    #[error("Timeout")]
    Timeout,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct JsonRpcResponse {
    #[allow(dead_code)]
    jsonrpc: String,
    #[allow(dead_code)]
    id: u64,
    result: Option<serde_json::Value>,
    error: Option<JsonRpcError>,
}

#[derive(Deserialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

struct McpClientInner {
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    stdout: Option<BufReader<ChildStdout>>,
    next_id: u64,
}

/// Stdio-based MCP client that communicates with an MCP server via stdin/stdout.
pub struct StdioMcpClient {
    config: McpServerConfig,
    inner: Mutex<McpClientInner>,
}

impl StdioMcpClient {
    pub fn new(config: McpServerConfig) -> Self {
        Self {
            config,
            inner: Mutex::new(McpClientInner {
                child: None,
                stdin: None,
                stdout: None,
                next_id: 1,
            }),
        }
    }

    pub fn config(&self) -> &McpServerConfig {
        &self.config
    }

    /// Connect to the MCP server by spawning the configured command and performing
    /// the JSON-RPC initialize handshake.
    pub async fn connect(&self) -> Result<(), McpError> {
        let mut inner = self.inner.lock().await;

        let mut child = Command::new(&self.config.command)
            .args(&self.config.args)
            .envs(&self.config.env)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| McpError::Connection(e.to_string()))?;

        inner.stdin = child.stdin.take();
        inner.stdout = child.stdout.take().map(BufReader::new);
        inner.child = Some(child);

        // Send initialize request
        Self::send_request_inner(
            &mut inner,
            "initialize",
            Some(serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "claude-code-rs", "version": "0.1.0" }
            })),
        )
        .await?;

        Ok(())
    }

    async fn send_request_inner(
        inner: &mut McpClientInner,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, McpError> {
        let id = inner.next_id;
        inner.next_id += 1;

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.to_string(),
            params,
        };

        let mut json =
            serde_json::to_string(&request).map_err(|e| McpError::Protocol(e.to_string()))?;
        json.push('\n');

        let stdin = inner
            .stdin
            .as_mut()
            .ok_or_else(|| McpError::Connection("Not connected".into()))?;
        stdin
            .write_all(json.as_bytes())
            .await
            .map_err(McpError::Io)?;
        stdin.flush().await.map_err(McpError::Io)?;

        // Read response line(s), skipping any JSON-RPC notifications (no "id" field)
        let stdout = inner
            .stdout
            .as_mut()
            .ok_or_else(|| McpError::Connection("Not connected".into()))?;

        loop {
            let mut line = String::new();
            let bytes_read = stdout.read_line(&mut line).await.map_err(McpError::Io)?;
            if bytes_read == 0 {
                return Err(McpError::Connection("Server closed connection".into()));
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Try to parse as a response (has "id" field)
            if let Ok(response) = serde_json::from_str::<JsonRpcResponse>(trimmed) {
                if let Some(error) = response.error {
                    return Err(McpError::Protocol(format!(
                        "{}: {}",
                        error.code, error.message
                    )));
                }
                return response
                    .result
                    .ok_or_else(|| McpError::Protocol("No result in response".into()));
            }
            // If it's not a valid response (e.g. a notification), skip and read next line
        }
    }

    async fn send_request(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, McpError> {
        let mut inner = self.inner.lock().await;
        Self::send_request_inner(&mut inner, method, params).await
    }
}

#[async_trait]
impl McpClient for StdioMcpClient {
    async fn list_tools(&self) -> Result<Vec<McpToolDefinition>, McpError> {
        let result = self.send_request("tools/list", None).await?;

        let tools_value = result
            .get("tools")
            .ok_or_else(|| McpError::Protocol("Response missing 'tools' field".into()))?;

        let tools: Vec<McpToolDefinition> = serde_json::from_value(tools_value.clone())
            .map_err(|e| McpError::Protocol(format!("Failed to parse tools: {}", e)))?;

        Ok(tools)
    }

    async fn call_tool(
        &self,
        name: &str,
        input: serde_json::Value,
    ) -> Result<McpToolResult, McpError> {
        let result = self
            .send_request(
                "tools/call",
                Some(serde_json::json!({
                    "name": name,
                    "arguments": input
                })),
            )
            .await?;

        let is_error = result
            .get("isError")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let content = result
            .get("content")
            .cloned()
            .unwrap_or(serde_json::Value::Null);

        Ok(McpToolResult { content, is_error })
    }

    async fn list_resources(&self) -> Result<Vec<McpResource>, McpError> {
        let result = self.send_request("resources/list", None).await?;

        let resources_value = result
            .get("resources")
            .ok_or_else(|| McpError::Protocol("Response missing 'resources' field".into()))?;

        let resources: Vec<McpResource> = serde_json::from_value(resources_value.clone())
            .map_err(|e| McpError::Protocol(format!("Failed to parse resources: {}", e)))?;

        Ok(resources)
    }

    async fn read_resource(&self, uri: &str) -> Result<serde_json::Value, McpError> {
        let result = self
            .send_request(
                "resources/read",
                Some(serde_json::json!({
                    "uri": uri
                })),
            )
            .await?;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config() -> McpServerConfig {
        McpServerConfig {
            name: "test".to_string(),
            command: "echo".to_string(),
            args: vec![],
            env: Default::default(),
            enabled: true,
        }
    }

    #[test]
    fn test_client_config_accessor() {
        let config = make_config();
        let client = StdioMcpClient::new(config);
        assert_eq!(client.config().name, "test");
    }

    #[test]
    fn test_error_display() {
        let e = McpError::Connection("refused".to_string());
        assert_eq!(e.to_string(), "Connection error: refused");

        let e = McpError::Timeout;
        assert_eq!(e.to_string(), "Timeout");

        let e = McpError::ToolNotFound("foo".to_string());
        assert_eq!(e.to_string(), "Tool not found: foo");
    }

    #[test]
    fn test_json_rpc_request_serialization() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "tools/list".to_string(),
            params: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"method\":\"tools/list\""));
        // params should be skipped when None
        assert!(!json.contains("params"));
    }

    #[test]
    fn test_json_rpc_request_with_params() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 2,
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({"name": "test"})),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"params\""));
    }

    #[test]
    fn test_json_rpc_response_deserialization() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":{"tools":[]}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_json_rpc_error_response() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":null,"error":{"code":-32601,"message":"Method not found"}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        assert!(resp.error.is_some());
        let err = resp.error.unwrap();
        assert_eq!(err.code, -32601);
        assert_eq!(err.message, "Method not found");
    }

    #[tokio::test]
    async fn test_connect_with_invalid_command() {
        let config = McpServerConfig {
            name: "bad".to_string(),
            command: "/nonexistent/command/that/does/not/exist".to_string(),
            args: vec![],
            env: Default::default(),
            enabled: true,
        };
        let client = StdioMcpClient::new(config);
        let err = client.connect().await.unwrap_err();
        assert!(matches!(err, McpError::Connection(_)));
    }

    #[tokio::test]
    async fn test_list_tools_not_connected() {
        let client = StdioMcpClient::new(make_config());
        let err = client.list_tools().await.unwrap_err();
        assert!(matches!(err, McpError::Connection(_)));
    }

    #[tokio::test]
    async fn test_call_tool_not_connected() {
        let client = StdioMcpClient::new(make_config());
        let err = client
            .call_tool("test", serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(matches!(err, McpError::Connection(_)));
    }

    #[tokio::test]
    async fn test_list_resources_not_connected() {
        let client = StdioMcpClient::new(make_config());
        let err = client.list_resources().await.unwrap_err();
        assert!(matches!(err, McpError::Connection(_)));
    }

    #[tokio::test]
    async fn test_read_resource_not_connected() {
        let client = StdioMcpClient::new(make_config());
        let err = client.read_resource("file:///test").await.unwrap_err();
        assert!(matches!(err, McpError::Connection(_)));
    }
}
