use async_trait::async_trait;
use serde_json::{Value, json};

use crate::trait_def::{Tool, ToolError, ToolResult, ValidationResult};

pub struct ConfigTool;

impl ConfigTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ConfigTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ConfigTool {
    fn name(&self) -> &str {
        "Config"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "setting": {
                    "type": "string",
                    "description": "The configuration key to get or set (e.g., 'model', 'theme', 'permissions.auto_approve')"
                },
                "value": {
                    "type": "string",
                    "description": "Optional value to set. If omitted, the current value is returned."
                }
            },
            "required": ["setting"]
        })
    }

    fn description(&self) -> String {
        "Get or set configuration values. When value is provided, sets the setting. When omitted, returns the current value.".to_string()
    }

    fn is_read_only(&self, input: &Value) -> bool {
        input.get("value").is_none()
    }

    fn is_concurrency_safe(&self, _input: &Value) -> bool {
        false
    }

    fn should_defer(&self) -> bool {
        true
    }

    fn validate_input(&self, input: &Value) -> ValidationResult {
        match input.get("setting").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => ValidationResult::Ok,
            _ => ValidationResult::Error {
                message: "Missing or empty 'setting' parameter".to_string(),
            },
        }
    }

    async fn call(&self, input: Value) -> Result<ToolResult, ToolError> {
        let setting = input
            .get("setting")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        if let Some(value) = input.get("value").and_then(|v| v.as_str()) {
            Ok(ToolResult::error(&format!(
                "Configuration management is not yet connected to the settings backend. Cannot set '{}' to '{}'.",
                setting, value
            )))
        } else {
            Ok(ToolResult::error(&format!(
                "Configuration management is not yet connected to the settings backend. Cannot read '{}'.",
                setting
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name_and_schema() {
        let tool = ConfigTool::new();
        assert_eq!(tool.name(), "Config");
        let schema = tool.input_schema();
        assert!(schema["properties"]["setting"].is_object());
        assert!(schema["properties"]["value"].is_object());
    }

    #[test]
    fn test_validate_input() {
        let tool = ConfigTool::new();
        assert!(matches!(
            tool.validate_input(&json!({"setting": "model"})),
            ValidationResult::Ok
        ));
        assert!(matches!(
            tool.validate_input(&json!({})),
            ValidationResult::Error { .. }
        ));
    }

    #[test]
    fn test_is_read_only() {
        let tool = ConfigTool::new();
        assert!(tool.is_read_only(&json!({"setting": "model"})));
        assert!(!tool.is_read_only(&json!({"setting": "model", "value": "opus"})));
    }

    #[tokio::test]
    async fn test_call_get() {
        let tool = ConfigTool::new();
        let result = tool.call(json!({"setting": "model"})).await.unwrap();
        assert!(result.is_error);
        assert!(result.content.as_str().unwrap().contains("model"));
    }

    #[tokio::test]
    async fn test_call_set() {
        let tool = ConfigTool::new();
        let result = tool
            .call(json!({"setting": "model", "value": "opus"}))
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(result.content.as_str().unwrap().contains("model"));
    }
}
