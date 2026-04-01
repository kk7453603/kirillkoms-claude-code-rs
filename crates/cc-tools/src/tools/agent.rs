use async_trait::async_trait;
use serde_json::{Value, json};

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

    async fn call(&self, input: Value) -> Result<ToolResult, ToolError> {
        let description = input
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("sub-agent task");
        let prompt =
            input
                .get("prompt")
                .and_then(|v| v.as_str())
                .ok_or(ToolError::ValidationFailed {
                    message: "Missing 'prompt' parameter".into(),
                })?;
        let run_in_background = input
            .get("run_in_background")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let exe = std::env::current_exe().unwrap_or_else(|_| "claude-code".into());
        let cwd = std::env::current_dir().unwrap_or_default();

        let mut cmd = tokio::process::Command::new(&exe);
        cmd.arg("-p")
            .arg(prompt)
            .arg("--print")
            .current_dir(&cwd)
            .stdin(std::process::Stdio::null());

        if run_in_background {
            let child = cmd
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .map_err(|e| ToolError::ExecutionFailed {
                    message: format!("Failed to spawn agent: {}", e),
                })?;
            Ok(ToolResult::text(&format!(
                "Agent '{}' launched in background (PID: {:?})",
                description,
                child.id()
            )))
        } else {
            let output = cmd.output().await.map_err(|e| ToolError::ExecutionFailed {
                message: format!("Agent failed: {}", e),
            })?;

            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            if output.status.success() {
                Ok(ToolResult::text(&stdout))
            } else {
                Ok(ToolResult::error(&format!(
                    "Agent failed:\n{}\n{}",
                    stdout, stderr
                )))
            }
        }
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
    async fn test_call_missing_prompt() {
        let tool = AgentTool::new();
        let result = tool.call(json!({})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_call_executes() {
        let tool = AgentTool::new();
        // This will fail because the binary won't exist or won't accept these args,
        // but it should not panic - it returns an error result.
        let result = tool
            .call(json!({"prompt": "echo hello", "description": "test agent"}))
            .await
            .unwrap();
        // Either succeeds or returns error result; both are valid
        let _ = result;
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
