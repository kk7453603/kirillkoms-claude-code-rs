use async_trait::async_trait;
use serde_json::{Value, json};

use crate::trait_def::{Tool, ToolError, ToolResult, ValidationResult};

pub struct TodoWriteTool;

impl TodoWriteTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TodoWriteTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for TodoWriteTool {
    fn name(&self) -> &str {
        "TodoWrite"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "todos": {
                    "type": "array",
                    "description": "Array of todo items to write. This replaces the entire todo list.",
                    "items": {
                        "type": "object",
                        "properties": {
                            "content": {
                                "type": "string",
                                "description": "The text content of the todo item"
                            },
                            "status": {
                                "type": "string",
                                "description": "Status of the todo item",
                                "enum": ["pending", "in_progress", "completed"]
                            },
                            "priority": {
                                "type": "string",
                                "description": "Priority level",
                                "enum": ["low", "medium", "high"]
                            }
                        },
                        "required": ["content", "status"]
                    }
                }
            },
            "required": ["todos"]
        })
    }

    fn description(&self) -> String {
        "Write and update a todo list for tracking progress on tasks. Each call replaces the entire list.".to_string()
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
        match input.get("todos").and_then(|v| v.as_array()) {
            Some(arr) => {
                for (i, item) in arr.iter().enumerate() {
                    if item.get("content").and_then(|v| v.as_str()).is_none() {
                        return ValidationResult::Error {
                            message: format!("Todo item {} is missing 'content' field", i),
                        };
                    }
                    if item.get("status").and_then(|v| v.as_str()).is_none() {
                        return ValidationResult::Error {
                            message: format!("Todo item {} is missing 'status' field", i),
                        };
                    }
                }
                ValidationResult::Ok
            }
            None => ValidationResult::Error {
                message: "Missing or invalid 'todos' array parameter".to_string(),
            },
        }
    }

    async fn call(&self, input: Value) -> Result<ToolResult, ToolError> {
        let todos =
            input
                .get("todos")
                .and_then(|v| v.as_array())
                .ok_or(ToolError::ValidationFailed {
                    message: "Missing 'todos' parameter".into(),
                })?;

        let total = todos.len();
        let completed = todos
            .iter()
            .filter(|t| t.get("status").and_then(|s| s.as_str()) == Some("completed"))
            .count();
        let in_progress = todos
            .iter()
            .filter(|t| t.get("status").and_then(|s| s.as_str()) == Some("in_progress"))
            .count();
        let pending = total - completed - in_progress;

        let mut summary = format!(
            "Todo list updated ({} items: {} pending, {} in progress, {} completed)\n\n",
            total, pending, in_progress, completed
        );

        for item in todos {
            let content = item
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("(no content)");
            let status = item
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("pending");
            let marker = match status {
                "completed" => "[x]",
                "in_progress" => "[~]",
                _ => "[ ]",
            };
            summary.push_str(&format!("{} {}\n", marker, content));
        }

        Ok(ToolResult::text(&summary))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name_and_schema() {
        let tool = TodoWriteTool::new();
        assert_eq!(tool.name(), "TodoWrite");
        let schema = tool.input_schema();
        assert!(schema["properties"]["todos"].is_object());
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("todos")));
    }

    #[test]
    fn test_validate_input() {
        let tool = TodoWriteTool::new();
        assert!(matches!(
            tool.validate_input(&json!({
                "todos": [
                    {"content": "task 1", "status": "pending"},
                    {"content": "task 2", "status": "completed"}
                ]
            })),
            ValidationResult::Ok
        ));
        assert!(matches!(
            tool.validate_input(&json!({})),
            ValidationResult::Error { .. }
        ));
        assert!(matches!(
            tool.validate_input(&json!({"todos": [{"content": "x"}]})),
            ValidationResult::Error { .. }
        ));
    }

    #[tokio::test]
    async fn test_todo_write() {
        let tool = TodoWriteTool::new();
        let result = tool
            .call(json!({
                "todos": [
                    {"content": "implement grep", "status": "completed"},
                    {"content": "implement agent", "status": "in_progress"},
                    {"content": "add tests", "status": "pending"}
                ]
            }))
            .await
            .unwrap();
        assert!(!result.is_error);
        let text = result.content.as_str().unwrap();
        assert!(text.contains("[x] implement grep"));
        assert!(text.contains("[~] implement agent"));
        assert!(text.contains("[ ] add tests"));
        assert!(text.contains("3 items"));
    }

    #[tokio::test]
    async fn test_todo_empty_list() {
        let tool = TodoWriteTool::new();
        let result = tool.call(json!({"todos": []})).await.unwrap();
        assert!(!result.is_error);
        assert!(result.content.as_str().unwrap().contains("0 items"));
    }

    #[test]
    fn test_should_defer() {
        let tool = TodoWriteTool::new();
        assert!(tool.should_defer());
    }
}
