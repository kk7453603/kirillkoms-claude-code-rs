use async_trait::async_trait;
use serde_json::{Value, json};

use crate::trait_def::{Tool, ToolError, ToolResult, ValidationResult};

// ──────────────── BriefTool ────────────────

pub struct BriefTool;

impl BriefTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for BriefTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for BriefTool {
    fn name(&self) -> &str {
        "Brief"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "The brief message or status update to send"
                },
                "attachments": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "type": { "type": "string" },
                            "content": { "type": "string" }
                        }
                    },
                    "description": "Optional attachments to include with the brief"
                },
                "status": {
                    "type": "string",
                    "description": "Optional status indicator",
                    "enum": ["info", "success", "warning", "error", "progress"]
                }
            },
            "required": ["message"]
        })
    }

    fn description(&self) -> String {
        "Send a brief status update or message to the user.".to_string()
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

    fn validate_input(&self, input: &Value) -> ValidationResult {
        match input.get("message").and_then(|v| v.as_str()) {
            Some(m) if !m.is_empty() => ValidationResult::Ok,
            _ => ValidationResult::Error {
                message: "Missing or empty 'message' parameter".to_string(),
            },
        }
    }

    async fn call(&self, input: Value) -> Result<ToolResult, ToolError> {
        let message = input.get("message").and_then(|v| v.as_str()).unwrap_or("");
        let status = input
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("info");

        Ok(ToolResult::text(&format!("[{}] {}", status, message)))
    }
}

// ──────────────── SendMessageTool ────────────────

pub struct SendMessageTool;

impl SendMessageTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SendMessageTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for SendMessageTool {
    fn name(&self) -> &str {
        "SendMessage"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "The message to send"
                },
                "to": {
                    "type": "string",
                    "description": "Optional recipient identifier"
                },
                "cc": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Optional list of CC recipients"
                }
            },
            "required": ["message"]
        })
    }

    fn description(&self) -> String {
        "Send a message to a user or system component.".to_string()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        false
    }

    fn is_concurrency_safe(&self, _input: &Value) -> bool {
        true
    }

    fn should_defer(&self) -> bool {
        true
    }

    fn validate_input(&self, input: &Value) -> ValidationResult {
        match input.get("message").and_then(|v| v.as_str()) {
            Some(m) if !m.is_empty() => ValidationResult::Ok,
            _ => ValidationResult::Error {
                message: "Missing or empty 'message' parameter".to_string(),
            },
        }
    }

    async fn call(&self, input: Value) -> Result<ToolResult, ToolError> {
        let message = input.get("message").and_then(|v| v.as_str()).unwrap_or("");
        let to = input.get("to").and_then(|v| v.as_str()).unwrap_or("user");

        Ok(ToolResult::text(&format!(
            "Message sent to {}: {}",
            to, message
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_brief_schema() {
        let tool = BriefTool::new();
        assert_eq!(tool.name(), "Brief");
        let schema = tool.input_schema();
        assert!(schema["properties"]["message"].is_object());
        assert!(schema["properties"]["status"].is_object());
    }

    #[test]
    fn test_send_message_schema() {
        let tool = SendMessageTool::new();
        assert_eq!(tool.name(), "SendMessage");
        let schema = tool.input_schema();
        assert!(schema["properties"]["message"].is_object());
        assert!(schema["properties"]["to"].is_object());
    }

    #[tokio::test]
    async fn test_brief_call() {
        let tool = BriefTool::new();
        let result = tool
            .call(json!({"message": "hello", "status": "success"}))
            .await
            .unwrap();
        assert!(!result.is_error);
        let text = result.content.as_str().unwrap();
        assert!(text.contains("[success]"));
        assert!(text.contains("hello"));
    }

    #[tokio::test]
    async fn test_send_message_call() {
        let tool = SendMessageTool::new();
        let result = tool
            .call(json!({"message": "test msg", "to": "admin"}))
            .await
            .unwrap();
        assert!(!result.is_error);
        let text = result.content.as_str().unwrap();
        assert!(text.contains("admin"));
        assert!(text.contains("test msg"));
    }

    #[test]
    fn test_validate_brief() {
        let tool = BriefTool::new();
        assert!(matches!(
            tool.validate_input(&json!({"message": "hi"})),
            ValidationResult::Ok
        ));
        assert!(matches!(
            tool.validate_input(&json!({})),
            ValidationResult::Error { .. }
        ));
    }
}
