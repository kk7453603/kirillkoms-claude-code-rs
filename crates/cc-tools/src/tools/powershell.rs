use async_trait::async_trait;
use serde_json::{json, Value};

use crate::trait_def::{InterruptBehavior, RenderedContent, Tool, ToolError, ToolResult, ValidationResult};

pub struct PowerShellTool;

impl PowerShellTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PowerShellTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for PowerShellTool {
    fn name(&self) -> &str {
        "PowerShell"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The PowerShell command to execute"
                },
                "timeout": {
                    "type": "number",
                    "description": "Optional timeout in milliseconds (max 600000)"
                },
                "description": {
                    "type": "string",
                    "description": "Clear, concise description of what this command does"
                }
            },
            "required": ["command"]
        })
    }

    fn description(&self) -> String {
        "Executes a PowerShell command and returns its output. Available on Windows systems with PowerShell installed.".to_string()
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

    fn is_enabled(&self) -> bool {
        // Only enabled if PowerShell is available
        which::which("pwsh")
            .or_else(|_| which::which("powershell"))
            .is_ok()
    }

    fn validate_input(&self, input: &Value) -> ValidationResult {
        match input.get("command").and_then(|v| v.as_str()) {
            Some(cmd) if !cmd.is_empty() => ValidationResult::Ok,
            _ => ValidationResult::Error {
                message: "Missing or empty 'command' parameter".to_string(),
            },
        }
    }

    fn render_tool_use(&self, input: &Value) -> RenderedContent {
        let cmd = input
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("<no command>");
        RenderedContent::Styled {
            text: format!("PS> {}", cmd),
            bold: true,
            dim: false,
            color: Some("blue".to_string()),
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

        // Try pwsh (PowerShell Core) first, then powershell (Windows PowerShell)
        let ps_binary = which::which("pwsh")
            .or_else(|_| which::which("powershell"))
            .map_err(|_| ToolError::ExecutionFailed {
                message: "PowerShell is not installed. Install PowerShell Core (pwsh) or ensure powershell is in PATH.".into(),
            })?;

        let output_future = tokio::process::Command::new(ps_binary)
            .args(["-NoProfile", "-NonInteractive", "-Command", command])
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
    fn test_name_and_schema() {
        let tool = PowerShellTool::new();
        assert_eq!(tool.name(), "PowerShell");
        let schema = tool.input_schema();
        assert!(schema["properties"]["command"].is_object());
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("command")));
    }

    #[test]
    fn test_validate_input() {
        let tool = PowerShellTool::new();
        assert!(matches!(
            tool.validate_input(&json!({"command": "Get-Process"})),
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

    #[test]
    fn test_interrupt_behavior() {
        let tool = PowerShellTool::new();
        assert_eq!(tool.interrupt_behavior(), InterruptBehavior::Cancel);
    }

    #[test]
    fn test_description() {
        let tool = PowerShellTool::new();
        assert!(tool.description().contains("PowerShell"));
    }

    #[test]
    fn test_render_tool_use() {
        let tool = PowerShellTool::new();
        let rendered = tool.render_tool_use(&json!({"command": "Get-Date"}));
        match rendered {
            RenderedContent::Styled { text, .. } => {
                assert!(text.contains("PS>"));
                assert!(text.contains("Get-Date"));
            }
            _ => panic!("Expected Styled content"),
        }
    }
}
