use async_trait::async_trait;
use serde_json::{json, Value};

use crate::trait_def::{Tool, ToolError, ToolResult, ValidationResult};

// ──────────────── EnterPlanModeTool ────────────────

pub struct EnterPlanModeTool;

impl EnterPlanModeTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for EnterPlanModeTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for EnterPlanModeTool {
    fn name(&self) -> &str {
        "EnterPlanMode"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    fn description(&self) -> String {
        "Enter plan mode. In plan mode, the assistant focuses on understanding the task and creating a plan before executing. No tools other than read-only tools are available in plan mode.".to_string()
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

    fn validate_input(&self, _input: &Value) -> ValidationResult {
        ValidationResult::Ok
    }

    async fn call(&self, _input: Value) -> Result<ToolResult, ToolError> {
        Ok(ToolResult::text(
            "Entered plan mode. In plan mode, focus on understanding the task and creating a plan. Only read-only tools are available. Use ExitPlanMode when ready to execute.",
        ))
    }
}

// ──────────────── ExitPlanModeV2Tool ────────────────

pub struct ExitPlanModeV2Tool;

impl ExitPlanModeV2Tool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ExitPlanModeV2Tool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ExitPlanModeV2Tool {
    fn name(&self) -> &str {
        "ExitPlanMode"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "allowedPrompts": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Optional list of allowed follow-up prompts when exiting plan mode"
                }
            },
            "required": []
        })
    }

    fn description(&self) -> String {
        "Exit plan mode and return to execution mode. Optionally provide allowed follow-up prompts."
            .to_string()
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

    fn validate_input(&self, _input: &Value) -> ValidationResult {
        ValidationResult::Ok
    }

    async fn call(&self, input: Value) -> Result<ToolResult, ToolError> {
        let prompts = input
            .get("allowedPrompts")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            });

        let msg = if let Some(p) = prompts {
            format!(
                "Exited plan mode. All tools are now available. Suggested follow-ups: {}",
                p
            )
        } else {
            "Exited plan mode. All tools are now available.".to_string()
        };

        Ok(ToolResult::text(&msg))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enter_plan_mode_schema() {
        let tool = EnterPlanModeTool::new();
        assert_eq!(tool.name(), "EnterPlanMode");
        let schema = tool.input_schema();
        assert_eq!(schema["type"], "object");
    }

    #[test]
    fn test_exit_plan_mode_schema() {
        let tool = ExitPlanModeV2Tool::new();
        assert_eq!(tool.name(), "ExitPlanMode");
        let schema = tool.input_schema();
        assert!(schema["properties"]["allowedPrompts"].is_object());
    }

    #[tokio::test]
    async fn test_enter_plan_mode_call() {
        let tool = EnterPlanModeTool::new();
        let result = tool.call(json!({})).await.unwrap();
        assert!(!result.is_error);
        assert!(result.content.as_str().unwrap().contains("plan mode"));
    }

    #[tokio::test]
    async fn test_exit_plan_mode_call() {
        let tool = ExitPlanModeV2Tool::new();
        let result = tool.call(json!({})).await.unwrap();
        assert!(!result.is_error);
        assert!(result.content.as_str().unwrap().contains("Exited plan mode"));
    }

    #[tokio::test]
    async fn test_exit_plan_mode_with_prompts() {
        let tool = ExitPlanModeV2Tool::new();
        let result = tool
            .call(json!({"allowedPrompts": ["fix tests", "deploy"]}))
            .await
            .unwrap();
        assert!(!result.is_error);
        let text = result.content.as_str().unwrap();
        assert!(text.contains("fix tests"));
        assert!(text.contains("deploy"));
    }
}
