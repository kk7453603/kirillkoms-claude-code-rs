use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::{Arc, LazyLock, Mutex};

use cc_mcp::client::{McpClient, StdioMcpClient};
use cc_mcp::types::McpToolDefinition;

use crate::trait_def::{SearchReadInfo, Tool, ToolError, ToolResult, ValidationResult};

/// Global registry of connected MCP clients.
/// Tools register clients here; MCP tools look them up by server name.
static MCP_CLIENTS: LazyLock<Mutex<Vec<(String, Arc<StdioMcpClient>)>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

/// Register a connected MCP client in the global registry.
pub fn register_mcp_client(name: String, client: Arc<StdioMcpClient>) {
    let mut clients = MCP_CLIENTS.lock().unwrap();
    // Replace existing entry with same name
    clients.retain(|(n, _)| n != &name);
    clients.push((name, client));
}

/// Remove an MCP client from the global registry.
pub fn unregister_mcp_client(name: &str) {
    let mut clients = MCP_CLIENTS.lock().unwrap();
    clients.retain(|(n, _)| n != name);
}

/// Get all registered MCP client names.
pub fn registered_mcp_servers() -> Vec<String> {
    let clients = MCP_CLIENTS.lock().unwrap();
    clients.iter().map(|(n, _)| n.clone()).collect()
}

fn get_client(name: &str) -> Option<Arc<StdioMcpClient>> {
    let clients = MCP_CLIENTS.lock().unwrap();
    clients
        .iter()
        .find(|(n, _)| n == name)
        .map(|(_, c)| c.clone())
}

fn get_all_clients() -> Vec<(String, Arc<StdioMcpClient>)> {
    let clients = MCP_CLIENTS.lock().unwrap();
    clients.clone()
}

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

    async fn call(&self, input: Value) -> Result<ToolResult, ToolError> {
        let server_filter = input.get("server_name").and_then(|v| v.as_str());

        let clients = if let Some(name) = server_filter {
            match get_client(name) {
                Some(c) => vec![(name.to_string(), c)],
                None => {
                    let servers = registered_mcp_servers();
                    if servers.is_empty() {
                        return Ok(ToolResult::error(
                            "No MCP servers are currently connected. Configure MCP servers in your settings to use this tool.",
                        ));
                    }
                    return Ok(ToolResult::error(&format!(
                        "MCP server '{}' not found. Connected servers: {}",
                        name,
                        servers.join(", ")
                    )));
                }
            }
        } else {
            get_all_clients()
        };

        if clients.is_empty() {
            return Ok(ToolResult::error(
                "No MCP servers are currently connected. Configure MCP servers in your settings to use this tool.",
            ));
        }

        let mut all_resources = Vec::new();
        for (name, client) in &clients {
            match client.list_resources().await {
                Ok(resources) => {
                    for r in resources {
                        all_resources.push(json!({
                            "server": name,
                            "uri": r.uri,
                            "name": r.name,
                            "description": r.description,
                            "mime_type": r.mime_type,
                        }));
                    }
                }
                Err(e) => {
                    all_resources.push(json!({
                        "server": name,
                        "error": format!("Failed to list resources: {}", e),
                    }));
                }
            }
        }

        let result = json!({
            "count": all_resources.len(),
            "resources": all_resources,
        });

        let json_str = serde_json::to_string_pretty(&result)
            .unwrap_or_else(|_| "Error serializing resources".to_string());
        Ok(ToolResult::text(&json_str))
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

    async fn call(&self, input: Value) -> Result<ToolResult, ToolError> {
        let server_name = input.get("server_name").and_then(|v| v.as_str()).ok_or(
            ToolError::ValidationFailed {
                message: "Missing 'server_name' parameter".into(),
            },
        )?;
        let uri = input
            .get("uri")
            .and_then(|v| v.as_str())
            .ok_or(ToolError::ValidationFailed {
                message: "Missing 'uri' parameter".into(),
            })?;

        let client = get_client(server_name).ok_or_else(|| {
            let servers = registered_mcp_servers();
            if servers.is_empty() {
                ToolError::ExecutionFailed {
                    message: "No MCP servers are currently connected. Configure MCP servers in your settings to use this tool.".into(),
                }
            } else {
                ToolError::ExecutionFailed {
                    message: format!(
                        "MCP server '{}' not found. Connected servers: {}",
                        server_name,
                        servers.join(", ")
                    ),
                }
            }
        })?;

        let result = client
            .read_resource(uri)
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                message: format!(
                    "Failed to read resource '{}' from '{}': {}",
                    uri, server_name, e
                ),
            })?;

        let json_str = serde_json::to_string_pretty(&result)
            .unwrap_or_else(|_| "Error serializing resource".to_string());
        Ok(ToolResult::text(&json_str))
    }
}

// ──────────────── McpDynamicTool ────────────────

/// A dynamic tool that wraps a single MCP server tool.
///
/// Instances are created at startup for each tool exposed by each connected
/// MCP server.  The tool name uses the normalized form `mcp__<server>__<tool>`.
pub struct McpDynamicTool {
    /// Normalized tool name: `mcp__<server>__<tool>`
    normalized_name: String,
    /// Raw MCP tool name (as reported by the server)
    raw_tool_name: String,
    /// Tool definition (description + input schema) from the server
    definition: McpToolDefinition,
    /// Connected client for this server
    client: Arc<StdioMcpClient>,
}

impl McpDynamicTool {
    pub fn new(
        server_name: &str,
        definition: McpToolDefinition,
        client: Arc<StdioMcpClient>,
    ) -> Self {
        let normalized_name =
            cc_mcp::normalization::normalize_tool_name(server_name, &definition.name);
        let raw_tool_name = definition.name.clone();
        Self {
            normalized_name,
            raw_tool_name,
            definition,
            client,
        }
    }
}

#[async_trait]
impl Tool for McpDynamicTool {
    fn name(&self) -> &str {
        &self.normalized_name
    }

    fn input_schema(&self) -> Value {
        self.definition.input_schema.clone()
    }

    fn description(&self) -> String {
        self.definition.description.clone()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        // MCP tools are treated as non-read-only (conservative default)
        false
    }

    fn is_concurrency_safe(&self, _input: &Value) -> bool {
        false
    }

    async fn call(&self, input: Value) -> Result<ToolResult, ToolError> {
        let result = self
            .client
            .call_tool(&self.raw_tool_name, input)
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                message: format!("MCP tool '{}' failed: {}", self.normalized_name, e),
            })?;

        if result.is_error {
            Ok(ToolResult::error(
                &result
                    .content
                    .as_str()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| result.content.to_string()),
            ))
        } else {
            let text = match &result.content {
                Value::String(s) => s.clone(),
                Value::Array(arr) => {
                    // MCP content array: each item may have type/text
                    arr.iter()
                        .filter_map(|item| {
                            item.get("text").and_then(|t| t.as_str()).map(|s| s.to_string())
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                }
                other => serde_json::to_string_pretty(other)
                    .unwrap_or_else(|_| other.to_string()),
            };
            Ok(ToolResult::text(&text))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cc_mcp::types::McpServerConfig;

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
    async fn test_list_no_servers() {
        let tool = ListMcpResourcesTool::new();
        // Clear any registered clients to ensure clean state
        {
            let mut clients = MCP_CLIENTS.lock().unwrap();
            clients.clear();
        }
        let result = tool.call(json!({})).await.unwrap();
        assert!(result.is_error);
        assert!(result.content.as_str().unwrap().contains("No MCP servers"));
    }

    #[tokio::test]
    async fn test_read_no_servers() {
        let tool = ReadMcpResourceTool::new();
        let result = tool.call(json!({"server_name": "s", "uri": "u"})).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_register_and_list_servers() {
        // Clean slate
        {
            let mut clients = MCP_CLIENTS.lock().unwrap();
            clients.clear();
        }

        let config = McpServerConfig {
            name: "test-server".to_string(),
            command: "echo".to_string(),
            args: vec![],
            env: Default::default(),
            enabled: true,
        };
        let client = Arc::new(StdioMcpClient::new(config));
        register_mcp_client("test-server".to_string(), client);

        let servers = registered_mcp_servers();
        assert!(servers.contains(&"test-server".to_string()));

        unregister_mcp_client("test-server");
        let servers = registered_mcp_servers();
        assert!(!servers.contains(&"test-server".to_string()));
    }

    #[test]
    fn test_register_replaces_existing() {
        {
            let mut clients = MCP_CLIENTS.lock().unwrap();
            clients.clear();
        }

        let config1 = McpServerConfig {
            name: "dup".to_string(),
            command: "echo".to_string(),
            args: vec![],
            env: Default::default(),
            enabled: true,
        };
        let config2 = McpServerConfig {
            name: "dup".to_string(),
            command: "cat".to_string(),
            args: vec![],
            env: Default::default(),
            enabled: true,
        };

        register_mcp_client("dup".to_string(), Arc::new(StdioMcpClient::new(config1)));
        register_mcp_client("dup".to_string(), Arc::new(StdioMcpClient::new(config2)));

        let clients = MCP_CLIENTS.lock().unwrap();
        let count = clients.iter().filter(|(n, _)| n == "dup").count();
        assert_eq!(count, 1);
        // Should be the second (cat) config
        let (_, client) = clients.iter().find(|(n, _)| n == "dup").unwrap();
        assert_eq!(client.config().command, "cat");
    }
}
