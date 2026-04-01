use async_trait::async_trait;
use serde_json::{json, Value};

use crate::trait_def::{
    InterruptBehavior, RenderedContent, Tool, ToolError, ToolResult, ValidationResult,
};

pub struct AgentTool;

impl AgentTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AgentTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for AgentTool {
    fn name(&self) -> &str {
        "Agent"
    }

    fn aliases(&self) -> Vec<&str> {
        vec!["SubAgent", "Dispatch"]
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "description": {
                    "type": "string",
                    "description": "A short (3-5 word) description of the task for display purposes"
                },
                "prompt": {
                    "type": "string",
                    "description": "The detailed task description and instructions for the sub-agent"
                },
                "subagent_type": {
                    "type": "string",
                    "description": "The type of sub-agent to spawn",
                    "enum": ["code", "research", "general"]
                },
                "model": {
                    "type": "string",
                    "description": "Optional model override for the sub-agent"
                },
                "run_in_background": {
                    "type": "boolean",
                    "description": "If true, the sub-agent runs in the background and results are returned later"
                }
            },
            "required": ["prompt"]
        })
    }

    fn description(&self) -> String {
        "Launch a sub-agent to handle a complex task independently. The sub-agent has access to all tools and can work on tasks in parallel.".to_string()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        false
    }

    fn is_concurrency_safe(&self, _input: &Value) -> bool {
        true
    }

    fn interrupt_behavior(&self) -> InterruptBehavior {
        InterruptBehavior::Cancel
    }

    fn should_defer(&self) -> bool {
        true
    }

    fn validate_input(&self, input: &Value) -> ValidationResult {
        match input.get("prompt").and_then(|v| v.as_str()) {
            Some(p) if !p.is_empty() => ValidationResult::Ok,
            _ => ValidationResult::Error {
                message: "Missing or empty 'prompt' parameter".to_string(),
            },
        }
    }

    fn render_tool_use(&self, input: &Value) -> RenderedContent {
        let desc = input
            .get("description")
            .and_then(|v| v.as_str())
            .or_else(|| input.get("prompt").and_then(|v| v.as_str()))
            .unwrap_or("sub-agent task");
        RenderedContent::Styled {
            text: format!("Agent: {}", desc),
            bold: true,
            dim: false,
            color: Some("yellow".to_string()),
        }
    }

    async fn call(&self, _input: Value) -> Result<ToolResult, ToolError> {
        Ok(ToolResult::error(
            "Sub-agent spawning is not yet available in this build. The Agent tool requires engine integration to create and manage sub-agent conversations. Please break down the task and handle sub-tasks directly instead.",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name_and_schema() {
        let tool = AgentTool::new();
        assert_eq!(tool.name(), "Agent");
        let schema = tool.input_schema();
        assert!(schema["properties"]["prompt"].is_object());
        assert!(schema["properties"]["description"].is_object());
        assert!(schema["properties"]["subagent_type"].is_object());
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("prompt")));
    }

    #[test]
    fn test_validate_input() {
        let tool = AgentTool::new();
        assert!(matches!(
            tool.validate_input(&json!({"prompt": "do something"})),
            ValidationResult::Ok
        ));
        assert!(matches!(
            tool.validate_input(&json!({})),
            ValidationResult::Error { .. }
        ));
    }

    #[tokio::test]
    async fn test_call_returns_stub() {
        let tool = AgentTool::new();
        let result = tool
            .call(json!({"prompt": "test"}))
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(result.content.as_str().unwrap().contains("not yet available"));
    }

    #[test]
    fn test_should_defer() {
        let tool = AgentTool::new();
        assert!(tool.should_defer());
    }

    #[test]
    fn test_aliases() {
        let tool = AgentTool::new();
        assert!(tool.aliases().contains(&"SubAgent"));
    }
}
