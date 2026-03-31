use async_trait::async_trait;
use serde_json::{json, Value};

use crate::trait_def::{
    InterruptBehavior, RenderedContent, SearchReadInfo, Tool, ToolError, ToolResult,
    ValidationResult,
};

pub struct BashTool;

impl BashTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for BashTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "Bash"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The bash command to execute"
                },
                "timeout": {
                    "type": "number",
                    "description": "Optional timeout in milliseconds (max 600000)"
                },
                "description": {
                    "type": "string",
                    "description": "Clear, concise description of what this command does"
                },
                "run_in_background": {
                    "type": "boolean",
                    "description": "Set to true to run this command in the background"
                }
            },
            "required": ["command"]
        })
    }

    fn description(&self) -> String {
        "Executes a given bash command and returns its output.".to_string()
    }

    fn is_read_only(&self, input: &Value) -> bool {
        let cmd = input
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        cc_permissions::bash_security::analyze_command(cmd).is_read_only
    }

    fn is_concurrency_safe(&self, _input: &Value) -> bool {
        true
    }

    fn is_destructive(&self, input: &Value) -> bool {
        let cmd = input
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        cc_permissions::bash_security::analyze_command(cmd).is_destructive
    }

    fn interrupt_behavior(&self) -> InterruptBehavior {
        InterruptBehavior::Cancel
    }

    fn validate_input(&self, input: &Value) -> ValidationResult {
        match input.get("command").and_then(|v| v.as_str()) {
            Some(cmd) if !cmd.is_empty() => ValidationResult::Ok,
            _ => ValidationResult::Error {
                message: "Missing or empty 'command' parameter".to_string(),
            },
        }
    }

    fn search_read_info(&self, input: &Value) -> SearchReadInfo {
        let cmd = input
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let lower = cmd.to_lowercase();
        SearchReadInfo {
            is_search: lower.starts_with("grep")
                || lower.starts_with("rg")
                || lower.starts_with("find")
                || lower.starts_with("ag"),
            is_read: lower.starts_with("cat")
                || lower.starts_with("head")
                || lower.starts_with("tail")
                || lower.starts_with("less"),
            is_list: lower.starts_with("ls") || lower.starts_with("tree"),
        }
    }

    fn render_tool_use(&self, input: &Value) -> RenderedContent {
        let cmd = input
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("<no command>");
        RenderedContent::Styled {
            text: format!("$ {}", cmd),
            bold: true,
            dim: false,
            color: Some("green".to_string()),
        }
    }

    async fn call(&self, input: Value) -> Result<ToolResult, ToolError> {
        let command = input
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or(ToolError::ValidationFailed {
                message: "Missing 'command' parameter".into(),
            })?;

        let timeout_ms = input
            .get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(120_000);

        let output_future = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(std::env::current_dir().unwrap_or_default())
            .output();

        let output = tokio::time::timeout(
            std::time::Duration::from_millis(timeout_ms),
            output_future,
        )
        .await
        .map_err(|_| ToolError::Timeout { timeout_ms })?
        .map_err(|e| ToolError::ExecutionFailed {
            message: e.to_string(),
        })?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = if stderr.is_empty() {
            stdout.to_string()
        } else if stdout.is_empty() {
            stderr.to_string()
        } else {
            format!("{}\n{}", stdout, stderr)
        };

        if output.status.success() {
            Ok(ToolResult::text(&combined))
        } else {
            Ok(ToolResult::error(&format!(
                "Exit code {}: {}",
                output.status.code().unwrap_or(-1),
                combined
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name() {
        let tool = BashTool::new();
        assert_eq!(tool.name(), "Bash");
    }

    #[test]
    fn test_schema_has_command() {
        let tool = BashTool::new();
        let schema = tool.input_schema();
        assert!(schema["properties"]["command"].is_object());
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("command")));
    }

    #[test]
    fn test_is_read_only() {
        let tool = BashTool::new();
        assert!(tool.is_read_only(&json!({"command": "ls -la"})));
        assert!(!tool.is_read_only(&json!({"command": "rm -rf /tmp/foo"})));
    }

    #[test]
    fn test_is_destructive() {
        let tool = BashTool::new();
        assert!(tool.is_destructive(&json!({"command": "rm -rf /tmp/foo"})));
        assert!(!tool.is_destructive(&json!({"command": "ls"})));
    }

    #[test]
    fn test_validate_input() {
        let tool = BashTool::new();
        assert!(matches!(
            tool.validate_input(&json!({"command": "echo hi"})),
            ValidationResult::Ok
        ));
        assert!(matches!(
            tool.validate_input(&json!({})),
            ValidationResult::Error { .. }
        ));
        assert!(matches!(
            tool.validate_input(&json!({"command": ""})),
            ValidationResult::Error { .. }
        ));
    }

    #[tokio::test]
    async fn test_call_echo() {
        let tool = BashTool::new();
        let result = tool
            .call(json!({"command": "echo hello_world"}))
            .await
            .unwrap();
        assert!(!result.is_error);
        let text = result.content.as_str().unwrap();
        assert!(text.contains("hello_world"));
    }

    #[tokio::test]
    async fn test_call_failing_command() {
        let tool = BashTool::new();
        let result = tool.call(json!({"command": "false"})).await.unwrap();
        assert!(result.is_error);
    }

    #[tokio::test]
    async fn test_call_missing_command() {
        let tool = BashTool::new();
        let result = tool.call(json!({})).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_search_read_info() {
        let tool = BashTool::new();
        let info = tool.search_read_info(&json!({"command": "grep foo bar"}));
        assert!(info.is_search);
        let info = tool.search_read_info(&json!({"command": "cat file.txt"}));
        assert!(info.is_read);
        let info = tool.search_read_info(&json!({"command": "ls -la"}));
        assert!(info.is_list);
    }
}
