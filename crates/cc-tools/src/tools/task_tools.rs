use async_trait::async_trait;
use serde_json::{json, Value};

use crate::trait_def::{Tool, ToolError, ToolResult, ValidationResult};

// ──────────────── TaskCreateTool ────────────────

pub struct TaskCreateTool;

impl TaskCreateTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TaskCreateTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for TaskCreateTool {
    fn name(&self) -> &str {
        "TaskCreate"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "description": {
                    "type": "string",
                    "description": "A short description of the task"
                },
                "prompt": {
                    "type": "string",
                    "description": "The full prompt/instructions for the task"
                },
                "priority": {
                    "type": "string",
                    "description": "Task priority",
                    "enum": ["low", "medium", "high", "critical"]
                }
            },
            "required": ["prompt"]
        })
    }

    fn description(&self) -> String {
        "Create a new background task that can run independently.".to_string()
    }

    fn is_read_only(&self, _input: &Value) -> bool { false }
    fn is_concurrency_safe(&self, _input: &Value) -> bool { true }
    fn should_defer(&self) -> bool { true }

    fn validate_input(&self, input: &Value) -> ValidationResult {
        match input.get("prompt").and_then(|v| v.as_str()) {
            Some(p) if !p.is_empty() => ValidationResult::Ok,
            _ => ValidationResult::Error { message: "Missing or empty 'prompt' parameter".into() },
        }
    }

    async fn call(&self, _input: Value) -> Result<ToolResult, ToolError> {
        Ok(ToolResult::error(
            "Task creation requires engine integration and is not yet available. Use the Agent tool or handle the task directly.",
        ))
    }
}

// ──────────────── TaskGetTool ────────────────

pub struct TaskGetTool;

impl TaskGetTool {
    pub fn new() -> Self { Self }
}

impl Default for TaskGetTool {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl Tool for TaskGetTool {
    fn name(&self) -> &str { "TaskGet" }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "task_id": {
                    "type": "string",
                    "description": "The ID of the task to retrieve"
                }
            },
            "required": ["task_id"]
        })
    }

    fn description(&self) -> String {
        "Get the status and details of a task by its ID.".to_string()
    }

    fn is_read_only(&self, _input: &Value) -> bool { true }
    fn is_concurrency_safe(&self, _input: &Value) -> bool { true }
    fn should_defer(&self) -> bool { true }

    fn validate_input(&self, input: &Value) -> ValidationResult {
        match input.get("task_id").and_then(|v| v.as_str()) {
            Some(id) if !id.is_empty() => ValidationResult::Ok,
            _ => ValidationResult::Error { message: "Missing or empty 'task_id' parameter".into() },
        }
    }

    async fn call(&self, _input: Value) -> Result<ToolResult, ToolError> {
        Ok(ToolResult::error("Task management is not yet available."))
    }
}

// ──────────────── TaskUpdateTool ────────────────

pub struct TaskUpdateTool;

impl TaskUpdateTool {
    pub fn new() -> Self { Self }
}

impl Default for TaskUpdateTool {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl Tool for TaskUpdateTool {
    fn name(&self) -> &str { "TaskUpdate" }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "task_id": {
                    "type": "string",
                    "description": "The ID of the task to update"
                },
                "status": {
                    "type": "string",
                    "description": "New status for the task",
                    "enum": ["pending", "running", "completed", "failed", "cancelled"]
                },
                "result": {
                    "type": "string",
                    "description": "Result or output of the task"
                }
            },
            "required": ["task_id"]
        })
    }

    fn description(&self) -> String {
        "Update the status or result of an existing task.".to_string()
    }

    fn is_read_only(&self, _input: &Value) -> bool { false }
    fn is_concurrency_safe(&self, _input: &Value) -> bool { true }
    fn should_defer(&self) -> bool { true }

    fn validate_input(&self, input: &Value) -> ValidationResult {
        match input.get("task_id").and_then(|v| v.as_str()) {
            Some(id) if !id.is_empty() => ValidationResult::Ok,
            _ => ValidationResult::Error { message: "Missing or empty 'task_id' parameter".into() },
        }
    }

    async fn call(&self, _input: Value) -> Result<ToolResult, ToolError> {
        Ok(ToolResult::error("Task management is not yet available."))
    }
}

// ──────────────── TaskStopTool ────────────────

pub struct TaskStopTool;

impl TaskStopTool {
    pub fn new() -> Self { Self }
}

impl Default for TaskStopTool {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl Tool for TaskStopTool {
    fn name(&self) -> &str { "TaskStop" }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "task_id": {
                    "type": "string",
                    "description": "The ID of the task to stop"
                }
            },
            "required": ["task_id"]
        })
    }

    fn description(&self) -> String {
        "Stop a running task.".to_string()
    }

    fn is_read_only(&self, _input: &Value) -> bool { false }
    fn is_concurrency_safe(&self, _input: &Value) -> bool { true }
    fn should_defer(&self) -> bool { true }

    fn validate_input(&self, input: &Value) -> ValidationResult {
        match input.get("task_id").and_then(|v| v.as_str()) {
            Some(id) if !id.is_empty() => ValidationResult::Ok,
            _ => ValidationResult::Error { message: "Missing or empty 'task_id' parameter".into() },
        }
    }

    async fn call(&self, _input: Value) -> Result<ToolResult, ToolError> {
        Ok(ToolResult::error("Task management is not yet available."))
    }
}

// ──────────────── TaskListTool ────────────────

pub struct TaskListTool;

impl TaskListTool {
    pub fn new() -> Self { Self }
}

impl Default for TaskListTool {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl Tool for TaskListTool {
    fn name(&self) -> &str { "TaskList" }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "status_filter": {
                    "type": "string",
                    "description": "Optional filter by task status",
                    "enum": ["pending", "running", "completed", "failed", "cancelled"]
                }
            },
            "required": []
        })
    }

    fn description(&self) -> String {
        "List all tasks, optionally filtered by status.".to_string()
    }

    fn is_read_only(&self, _input: &Value) -> bool { true }
    fn is_concurrency_safe(&self, _input: &Value) -> bool { true }
    fn should_defer(&self) -> bool { true }

    fn validate_input(&self, _input: &Value) -> ValidationResult {
        ValidationResult::Ok
    }

    async fn call(&self, _input: Value) -> Result<ToolResult, ToolError> {
        Ok(ToolResult::error("Task management is not yet available."))
    }
}

// ──────────────── TaskOutputTool ────────────────

pub struct TaskOutputTool;

impl TaskOutputTool {
    pub fn new() -> Self { Self }
}

impl Default for TaskOutputTool {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl Tool for TaskOutputTool {
    fn name(&self) -> &str { "TaskOutput" }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "task_id": {
                    "type": "string",
                    "description": "The ID of the task to get output from"
                },
                "tail": {
                    "type": "number",
                    "description": "Number of lines from the end to return"
                }
            },
            "required": ["task_id"]
        })
    }

    fn description(&self) -> String {
        "Get the output/logs of a task.".to_string()
    }

    fn is_read_only(&self, _input: &Value) -> bool { true }
    fn is_concurrency_safe(&self, _input: &Value) -> bool { true }
    fn should_defer(&self) -> bool { true }

    fn validate_input(&self, input: &Value) -> ValidationResult {
        match input.get("task_id").and_then(|v| v.as_str()) {
            Some(id) if !id.is_empty() => ValidationResult::Ok,
            _ => ValidationResult::Error { message: "Missing or empty 'task_id' parameter".into() },
        }
    }

    async fn call(&self, _input: Value) -> Result<ToolResult, ToolError> {
        Ok(ToolResult::error("Task management is not yet available."))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_create_schema() {
        let tool = TaskCreateTool::new();
        assert_eq!(tool.name(), "TaskCreate");
        let schema = tool.input_schema();
        assert!(schema["properties"]["prompt"].is_object());
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("prompt")));
    }

    #[test]
    fn test_task_get_schema() {
        let tool = TaskGetTool::new();
        assert_eq!(tool.name(), "TaskGet");
        assert!(tool.is_read_only(&json!({})));
    }

    #[test]
    fn test_task_list_schema() {
        let tool = TaskListTool::new();
        assert_eq!(tool.name(), "TaskList");
        assert!(tool.is_read_only(&json!({})));
    }

    #[tokio::test]
    async fn test_task_create_stub() {
        let tool = TaskCreateTool::new();
        let result = tool.call(json!({"prompt": "test"})).await.unwrap();
        assert!(result.is_error);
        assert!(result.content.as_str().unwrap().contains("not yet available"));
    }

    #[tokio::test]
    async fn test_task_list_stub() {
        let tool = TaskListTool::new();
        let result = tool.call(json!({})).await.unwrap();
        assert!(result.is_error);
    }
}
