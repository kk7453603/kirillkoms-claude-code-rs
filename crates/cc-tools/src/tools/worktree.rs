use async_trait::async_trait;
use serde_json::{json, Value};

use crate::trait_def::{Tool, ToolError, ToolResult, ValidationResult};

// ──────────────── EnterWorktreeTool ────────────────

pub struct EnterWorktreeTool;

impl EnterWorktreeTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for EnterWorktreeTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for EnterWorktreeTool {
    fn name(&self) -> &str {
        "EnterWorktree"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Optional name for the worktree. If not provided, a unique name will be generated."
                }
            },
            "required": []
        })
    }

    fn description(&self) -> String {
        "Create and enter a git worktree for isolated work. Uses 'git worktree add' to create a new worktree.".to_string()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        false
    }

    fn is_concurrency_safe(&self, _input: &Value) -> bool {
        false
    }

    fn should_defer(&self) -> bool {
        true
    }

    fn validate_input(&self, _input: &Value) -> ValidationResult {
        ValidationResult::Ok
    }

    async fn call(&self, input: Value) -> Result<ToolResult, ToolError> {
        let name = input
            .get("name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("worktree-{}", &uuid::Uuid::new_v4().to_string()[..8]));

        let cwd = std::env::current_dir().unwrap_or_default();
        let worktree_path = cwd.join("..").join(&name);

        let output = tokio::process::Command::new("git")
            .args(["worktree", "add", "-b", &name, worktree_path.to_str().unwrap_or(&name)])
            .current_dir(&cwd)
            .output()
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                message: format!("Failed to run git worktree add: {}", e),
            })?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(ToolResult::text(&format!(
                "Created worktree '{}' at {}\n{}",
                name,
                worktree_path.display(),
                stdout
            )))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Ok(ToolResult::error(&format!(
                "Failed to create worktree: {}",
                stderr
            )))
        }
    }
}

// ──────────────── ExitWorktreeTool ────────────────

pub struct ExitWorktreeTool;

impl ExitWorktreeTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ExitWorktreeTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ExitWorktreeTool {
    fn name(&self) -> &str {
        "ExitWorktree"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "What to do with the worktree: 'keep' to preserve it, 'remove' to delete it.",
                    "enum": ["keep", "remove"]
                },
                "discard_changes": {
                    "type": "boolean",
                    "description": "If true, discard any uncommitted changes before removing the worktree."
                }
            },
            "required": ["action"]
        })
    }

    fn description(&self) -> String {
        "Exit the current git worktree. Optionally remove it and discard changes.".to_string()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        false
    }

    fn is_concurrency_safe(&self, _input: &Value) -> bool {
        false
    }

    fn should_defer(&self) -> bool {
        true
    }

    fn validate_input(&self, input: &Value) -> ValidationResult {
        match input.get("action").and_then(|v| v.as_str()) {
            Some("keep") | Some("remove") => ValidationResult::Ok,
            _ => ValidationResult::Error {
                message: "Missing or invalid 'action' parameter. Must be 'keep' or 'remove'.".to_string(),
            },
        }
    }

    async fn call(&self, input: Value) -> Result<ToolResult, ToolError> {
        let action = input
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or(ToolError::ValidationFailed {
                message: "Missing 'action' parameter".into(),
            })?;

        let discard = input
            .get("discard_changes")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let cwd = std::env::current_dir().unwrap_or_default();

        if action == "remove" {
            // Optionally discard changes
            if discard {
                let _ = tokio::process::Command::new("git")
                    .args(["checkout", "."])
                    .current_dir(&cwd)
                    .output()
                    .await;
                let _ = tokio::process::Command::new("git")
                    .args(["clean", "-fd"])
                    .current_dir(&cwd)
                    .output()
                    .await;
            }

            let output = tokio::process::Command::new("git")
                .args(["worktree", "remove", cwd.to_str().unwrap_or("."), "--force"])
                .output()
                .await
                .map_err(|e| ToolError::ExecutionFailed {
                    message: format!("Failed to run git worktree remove: {}", e),
                })?;

            if output.status.success() {
                Ok(ToolResult::text("Worktree removed successfully."))
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Ok(ToolResult::error(&format!(
                    "Failed to remove worktree: {}",
                    stderr
                )))
            }
        } else {
            Ok(ToolResult::text(
                "Keeping worktree. You can return to the main repository.",
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enter_worktree_schema() {
        let tool = EnterWorktreeTool::new();
        assert_eq!(tool.name(), "EnterWorktree");
        let schema = tool.input_schema();
        assert!(schema["properties"]["name"].is_object());
    }

    #[test]
    fn test_exit_worktree_schema() {
        let tool = ExitWorktreeTool::new();
        assert_eq!(tool.name(), "ExitWorktree");
        let schema = tool.input_schema();
        assert!(schema["properties"]["action"].is_object());
        assert!(schema["properties"]["discard_changes"].is_object());
    }

    #[test]
    fn test_exit_worktree_validate() {
        let tool = ExitWorktreeTool::new();
        assert!(matches!(
            tool.validate_input(&json!({"action": "keep"})),
            ValidationResult::Ok
        ));
        assert!(matches!(
            tool.validate_input(&json!({"action": "remove"})),
            ValidationResult::Ok
        ));
        assert!(matches!(
            tool.validate_input(&json!({"action": "invalid"})),
            ValidationResult::Error { .. }
        ));
    }

    #[tokio::test]
    async fn test_exit_worktree_keep() {
        let tool = ExitWorktreeTool::new();
        let result = tool.call(json!({"action": "keep"})).await.unwrap();
        assert!(!result.is_error);
        assert!(result.content.as_str().unwrap().contains("Keeping"));
    }

    #[test]
    fn test_should_defer() {
        assert!(EnterWorktreeTool::new().should_defer());
        assert!(ExitWorktreeTool::new().should_defer());
    }
}
