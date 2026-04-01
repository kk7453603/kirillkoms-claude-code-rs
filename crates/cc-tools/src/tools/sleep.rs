use async_trait::async_trait;
use serde_json::{json, Value};

use crate::trait_def::{Tool, ToolError, ToolResult, ValidationResult};

pub struct SleepTool;

impl SleepTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SleepTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for SleepTool {
    fn name(&self) -> &str {
        "Sleep"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "duration_ms": {
                    "type": "number",
                    "description": "Duration to sleep in milliseconds (max 300000 = 5 minutes)"
                }
            },
            "required": ["duration_ms"]
        })
    }

    fn description(&self) -> String {
        "Pause execution for a specified duration. Useful for waiting between polling operations."
            .to_string()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        true
    }

    fn is_concurrency_safe(&self, _input: &Value) -> bool {
        true
    }

    fn validate_input(&self, input: &Value) -> ValidationResult {
        match input.get("duration_ms").and_then(|v| v.as_u64()) {
            Some(ms) if ms > 0 && ms <= 300_000 => ValidationResult::Ok,
            Some(ms) if ms > 300_000 => ValidationResult::Error {
                message: "duration_ms must be <= 300000 (5 minutes)".to_string(),
            },
            _ => ValidationResult::Error {
                message: "Missing or invalid 'duration_ms' parameter (must be a positive number)"
                    .to_string(),
            },
        }
    }

    async fn call(&self, input: Value) -> Result<ToolResult, ToolError> {
        let duration_ms = input
            .get("duration_ms")
            .and_then(|v| v.as_u64())
            .ok_or(ToolError::ValidationFailed {
                message: "Missing 'duration_ms' parameter".into(),
            })?;

        if duration_ms > 300_000 {
            return Ok(ToolResult::error(
                "Sleep duration cannot exceed 300000ms (5 minutes)",
            ));
        }

        tokio::time::sleep(std::time::Duration::from_millis(duration_ms)).await;

        Ok(ToolResult::text(&format!("Slept for {}ms", duration_ms)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name_and_schema() {
        let tool = SleepTool::new();
        assert_eq!(tool.name(), "Sleep");
        let schema = tool.input_schema();
        assert!(schema["properties"]["duration_ms"].is_object());
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("duration_ms")));
    }

    #[test]
    fn test_validate_input() {
        let tool = SleepTool::new();
        assert!(matches!(
            tool.validate_input(&json!({"duration_ms": 1000})),
            ValidationResult::Ok
        ));
        assert!(matches!(
            tool.validate_input(&json!({"duration_ms": 500000})),
            ValidationResult::Error { .. }
        ));
        assert!(matches!(
            tool.validate_input(&json!({})),
            ValidationResult::Error { .. }
        ));
    }

    #[tokio::test]
    async fn test_sleep_short() {
        let tool = SleepTool::new();
        let start = std::time::Instant::now();
        let result = tool.call(json!({"duration_ms": 50})).await.unwrap();
        let elapsed = start.elapsed();
        assert!(!result.is_error);
        assert!(elapsed.as_millis() >= 40); // Allow some slack
        assert!(result.content.as_str().unwrap().contains("50ms"));
    }

    #[tokio::test]
    async fn test_sleep_too_long() {
        let tool = SleepTool::new();
        let result = tool.call(json!({"duration_ms": 500000})).await.unwrap();
        assert!(result.is_error);
    }

    #[test]
    fn test_is_read_only() {
        let tool = SleepTool::new();
        assert!(tool.is_read_only(&json!({})));
    }
}
