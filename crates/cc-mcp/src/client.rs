use async_trait::async_trait;
use crate::types::*;

#[async_trait]
pub trait McpClient: Send + Sync {
    async fn list_tools(&self) -> Result<Vec<McpToolDefinition>, McpError>;
    async fn call_tool(&self, name: &str, input: serde_json::Value) -> Result<McpToolResult, McpError>;
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

/// Stdio-based MCP client that communicates with an MCP server via stdin/stdout.
pub struct StdioMcpClient {
    config: McpServerConfig,
}

impl StdioMcpClient {
    pub fn new(config: McpServerConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &McpServerConfig {
        &self.config
    }
}

#[async_trait]
impl McpClient for StdioMcpClient {
    async fn list_tools(&self) -> Result<Vec<McpToolDefinition>, McpError> {
        // Stub: in a real implementation this would spawn the process and
        // send a JSON-RPC request to list tools.
        Ok(vec![])
    }

    async fn call_tool(&self, name: &str, _input: serde_json::Value) -> Result<McpToolResult, McpError> {
        Err(McpError::ToolNotFound(name.to_string()))
    }

    async fn list_resources(&self) -> Result<Vec<McpResource>, McpError> {
        Ok(vec![])
    }

    async fn read_resource(&self, uri: &str) -> Result<serde_json::Value, McpError> {
        Err(McpError::Protocol(format!("Resource not found: {}", uri)))
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

    #[tokio::test]
    async fn test_stdio_client_list_tools() {
        let client = StdioMcpClient::new(make_config());
        let tools = client.list_tools().await.unwrap();
        assert!(tools.is_empty());
    }

    #[tokio::test]
    async fn test_stdio_client_call_tool_not_found() {
        let client = StdioMcpClient::new(make_config());
        let err = client
            .call_tool("nonexistent", serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(matches!(err, McpError::ToolNotFound(_)));
    }

    #[tokio::test]
    async fn test_stdio_client_list_resources() {
        let client = StdioMcpClient::new(make_config());
        let resources = client.list_resources().await.unwrap();
        assert!(resources.is_empty());
    }

    #[tokio::test]
    async fn test_stdio_client_read_resource_not_found() {
        let client = StdioMcpClient::new(make_config());
        let err = client
            .read_resource("file:///test")
            .await
            .unwrap_err();
        assert!(matches!(err, McpError::Protocol(_)));
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
    fn test_client_config_accessor() {
        let config = make_config();
        let client = StdioMcpClient::new(config);
        assert_eq!(client.config().name, "test");
    }
}
