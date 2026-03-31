use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct McpServerConnection {
    pub config: McpServerConfig,
    pub status: ConnectionStatus,
    pub tools: Vec<McpToolDefinition>,
    pub resources: Vec<McpResource>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionStatus {
    Connecting,
    Connected,
    Disconnected,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResource {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolResult {
    pub content: serde_json::Value,
    pub is_error: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config_serialization_roundtrip() {
        let config = McpServerConfig {
            name: "test-server".to_string(),
            command: "node".to_string(),
            args: vec!["server.js".to_string()],
            env: {
                let mut m = std::collections::HashMap::new();
                m.insert("PORT".to_string(), "3000".to_string());
                m
            },
            enabled: true,
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: McpServerConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "test-server");
        assert_eq!(deserialized.command, "node");
        assert_eq!(deserialized.args, vec!["server.js"]);
        assert_eq!(deserialized.env.get("PORT").unwrap(), "3000");
        assert!(deserialized.enabled);
    }

    #[test]
    fn test_server_config_defaults() {
        let json = r#"{"name":"s","command":"c","args":[]}"#;
        let config: McpServerConfig = serde_json::from_str(json).unwrap();
        assert!(!config.enabled);
        assert!(config.env.is_empty());
    }

    #[test]
    fn test_tool_definition_roundtrip() {
        let tool = McpToolDefinition {
            name: "read_file".to_string(),
            description: "Read a file".to_string(),
            input_schema: serde_json::json!({"type": "object", "properties": {"path": {"type": "string"}}}),
        };
        let json = serde_json::to_string(&tool).unwrap();
        let deserialized: McpToolDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "read_file");
        assert_eq!(deserialized.description, "Read a file");
    }

    #[test]
    fn test_resource_roundtrip() {
        let resource = McpResource {
            uri: "file:///tmp/test.txt".to_string(),
            name: "test.txt".to_string(),
            description: Some("A test file".to_string()),
            mime_type: Some("text/plain".to_string()),
        };
        let json = serde_json::to_string(&resource).unwrap();
        let deserialized: McpResource = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.uri, "file:///tmp/test.txt");
        assert_eq!(deserialized.description, Some("A test file".to_string()));
    }

    #[test]
    fn test_resource_optional_fields() {
        let json = r#"{"uri":"file:///a","name":"a","description":null,"mime_type":null}"#;
        let resource: McpResource = serde_json::from_str(json).unwrap();
        assert!(resource.description.is_none());
        assert!(resource.mime_type.is_none());
    }

    #[test]
    fn test_tool_result_roundtrip() {
        let result = McpToolResult {
            content: serde_json::json!({"text": "hello"}),
            is_error: false,
        };
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: McpToolResult = serde_json::from_str(&json).unwrap();
        assert!(!deserialized.is_error);
        assert_eq!(deserialized.content["text"], "hello");
    }

    #[test]
    fn test_connection_status_equality() {
        assert_eq!(ConnectionStatus::Connected, ConnectionStatus::Connected);
        assert_ne!(ConnectionStatus::Connected, ConnectionStatus::Disconnected);
        assert_eq!(
            ConnectionStatus::Error("x".to_string()),
            ConnectionStatus::Error("x".to_string())
        );
        assert_ne!(
            ConnectionStatus::Error("x".to_string()),
            ConnectionStatus::Error("y".to_string())
        );
    }
}
