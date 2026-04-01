use async_trait::async_trait;
use serde_json::{json, Value};

use crate::trait_def::{SearchReadInfo, Tool, ToolError, ToolResult, ValidationResult};

// ──────────────── ListMcpResourcesTool ────────────────

pub struct ListMcpResourcesTool;

impl ListMcpResourcesTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ListMcpResourcesTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ListMcpResourcesTool {
    fn name(&self) -> &str {
        "ListMcpResources"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "server_name": {
                    "type": "string",
                    "description": "Optional MCP server name to filter resources"
                },
                "resource_type": {
                    "type": "string",
                    "description": "Optional resource type filter"
                }
            },
            "required": []
        })
    }

    fn description(&self) -> String {
        "List available MCP (Model Context Protocol) resources from connected servers.".to_string()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        true
    }

    fn is_concurrency_safe(&self, _input: &Value) -> bool {
        true
    }

    fn should_defer(&self) -> bool {
        true
    }

    fn search_read_info(&self, _input: &Value) -> SearchReadInfo {
        SearchReadInfo {
            is_search: false,
            is_read: false,
            is_list: true,
        }
    }

    fn validate_input(&self, _input: &Value) -> ValidationResult {
        ValidationResult::Ok
    }

    async fn call(&self, _input: Value) -> Result<ToolResult, ToolError> {
        Ok(ToolResult::error(
            "No MCP servers are currently connected. Configure MCP servers in your settings to use this tool.",
        ))
    }
}

// ──────────────── ReadMcpResourceTool ────────────────

pub struct ReadMcpResourceTool;

impl ReadMcpResourceTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ReadMcpResourceTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ReadMcpResourceTool {
    fn name(&self) -> &str {
        "ReadMcpResource"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "server_name": {
                    "type": "string",
                    "description": "The name of the MCP server providing the resource"
                },
                "uri": {
                    "type": "string",
                    "description": "The URI of the resource to read"
                }
            },
            "required": ["server_name", "uri"]
        })
    }

    fn description(&self) -> String {
        "Read a specific resource from an MCP server by its URI.".to_string()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        true
    }

    fn is_concurrency_safe(&self, _input: &Value) -> bool {
        true
    }

    fn should_defer(&self) -> bool {
        true
    }

    fn search_read_info(&self, _input: &Value) -> SearchReadInfo {
        SearchReadInfo {
            is_search: false,
            is_read: true,
            is_list: false,
        }
    }

    fn validate_input(&self, input: &Value) -> ValidationResult {
        let server = input.get("server_name").and_then(|v| v.as_str());
        let uri = input.get("uri").and_then(|v| v.as_str());
        if server.is_none() || server == Some("") {
            return ValidationResult::Error {
                message: "Missing or empty 'server_name' parameter".to_string(),
            };
        }
        if uri.is_none() || uri == Some("") {
            return ValidationResult::Error {
                message: "Missing or empty 'uri' parameter".to_string(),
            };
        }
        ValidationResult::Ok
    }

    async fn call(&self, _input: Value) -> Result<ToolResult, ToolError> {
        Ok(ToolResult::error(
            "No MCP servers are currently connected. Configure MCP servers in your settings to use this tool.",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_mcp_resources_schema() {
        let tool = ListMcpResourcesTool::new();
        assert_eq!(tool.name(), "ListMcpResources");
        assert!(tool.is_read_only(&json!({})));
        assert!(tool.should_defer());
    }

    #[test]
    fn test_read_mcp_resource_schema() {
        let tool = ReadMcpResourceTool::new();
        assert_eq!(tool.name(), "ReadMcpResource");
        let schema = tool.input_schema();
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("server_name")));
        assert!(required.contains(&json!("uri")));
    }

    #[test]
    fn test_read_mcp_validate() {
        let tool = ReadMcpResourceTool::new();
        assert!(matches!(
            tool.validate_input(&json!({"server_name": "test", "uri": "file:///test"})),
            ValidationResult::Ok
        ));
        assert!(matches!(
            tool.validate_input(&json!({})),
            ValidationResult::Error { .. }
        ));
    }

    #[tokio::test]
    async fn test_list_stub() {
        let tool = ListMcpResourcesTool::new();
        let result = tool.call(json!({})).await.unwrap();
        assert!(result.is_error);
        assert!(result.content.as_str().unwrap().contains("MCP"));
    }

    #[tokio::test]
    async fn test_read_stub() {
        let tool = ReadMcpResourceTool::new();
        let result = tool
            .call(json!({"server_name": "s", "uri": "u"}))
            .await
            .unwrap();
        assert!(result.is_error);
    }
}
