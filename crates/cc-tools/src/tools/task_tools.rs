use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::{LazyLock, Mutex};

use cc_tasks::types::{TaskInfo, TaskManager, TaskStatus};

use crate::trait_def::{Tool, ToolError, ToolResult, ValidationResult};

static TASK_MANAGER: LazyLock<Mutex<TaskManager>> =
    LazyLock::new(|| Mutex::new(TaskManager::new()));

fn parse_status(s: &str) -> Option<TaskStatus> {
    match s {
        "pending" => Some(TaskStatus::Pending),
        "running" => Some(TaskStatus::Running),
        "completed" => Some(TaskStatus::Completed),
        "failed" => Some(TaskStatus::Failed),
        "cancelled" => Some(TaskStatus::Cancelled),
        _ => None,
    }
}

fn task_to_json(task: &TaskInfo) -> Value {
    json!({
        "id": task.id,
        "name": task.name,
        "status": task.status,
        "created_at": task.created_at,
        "description": task.description,
        "output": task.output,
    })
}

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
        match input.get("prompt").and_then(|v| v.as_str()) {
            Some(p) if !p.is_empty() => ValidationResult::Ok,
            _ => ValidationResult::Error {
                message: "Missing or empty 'prompt' parameter".into(),
            },
        }
    }

    async fn call(&self, input: Value) -> Result<ToolResult, ToolError> {
        let prompt =
            input
                .get("prompt")
                .and_then(|v| v.as_str())
                .ok_or(ToolError::ValidationFailed {
                    message: "Missing 'prompt' parameter".into(),
                })?;
        let description = input.get("description").and_then(|v| v.as_str());

        let task_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        let task = TaskInfo {
            id: task_id.clone(),
            name: description.unwrap_or(prompt).chars().take(80).collect(),
            status: TaskStatus::Pending,
            created_at: now,
            description: description.map(|s| s.to_string()),
            output: None,
        };

        let mut mgr = TASK_MANAGER.lock().unwrap();
        mgr.add_task(task);

        Ok(ToolResult::text(&format!(
            "Task created with ID: {}\nStatus: pending",
            task_id
        )))
    }
}

// ──────────────── TaskGetTool ────────────────

pub struct TaskGetTool;

impl TaskGetTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TaskGetTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for TaskGetTool {
    fn name(&self) -> &str {
        "TaskGet"
    }

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

    fn is_read_only(&self, _input: &Value) -> bool {
        true
    }
    fn is_concurrency_safe(&self, _input: &Value) -> bool {
        true
    }
    fn should_defer(&self) -> bool {
        true
    }

    fn validate_input(&self, input: &Value) -> ValidationResult {
        match input.get("task_id").and_then(|v| v.as_str()) {
            Some(id) if !id.is_empty() => ValidationResult::Ok,
            _ => ValidationResult::Error {
                message: "Missing or empty 'task_id' parameter".into(),
            },
        }
    }

    async fn call(&self, input: Value) -> Result<ToolResult, ToolError> {
        let task_id =
            input
                .get("task_id")
                .and_then(|v| v.as_str())
                .ok_or(ToolError::ValidationFailed {
                    message: "Missing 'task_id' parameter".into(),
                })?;

        let mgr = TASK_MANAGER.lock().unwrap();
        match mgr.get_task(task_id) {
            Some(task) => {
                let json = serde_json::to_string_pretty(&task_to_json(task))
                    .unwrap_or_else(|_| "Error serializing task".to_string());
                Ok(ToolResult::text(&json))
            }
            None => Ok(ToolResult::error(&format!("Task '{}' not found", task_id))),
        }
    }
}

// ──────────────── TaskUpdateTool ────────────────

pub struct TaskUpdateTool;

impl TaskUpdateTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TaskUpdateTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for TaskUpdateTool {
    fn name(&self) -> &str {
        "TaskUpdate"
    }

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
        match input.get("task_id").and_then(|v| v.as_str()) {
            Some(id) if !id.is_empty() => ValidationResult::Ok,
            _ => ValidationResult::Error {
                message: "Missing or empty 'task_id' parameter".into(),
            },
        }
    }

    async fn call(&self, input: Value) -> Result<ToolResult, ToolError> {
        let task_id =
            input
                .get("task_id")
                .and_then(|v| v.as_str())
                .ok_or(ToolError::ValidationFailed {
                    message: "Missing 'task_id' parameter".into(),
                })?;
        let status_str = input.get("status").and_then(|v| v.as_str());
        let result_str = input.get("result").and_then(|v| v.as_str());

        let mut mgr = TASK_MANAGER.lock().unwrap();

        // Check task exists
        if mgr.get_task(task_id).is_none() {
            return Ok(ToolResult::error(&format!("Task '{}' not found", task_id)));
        }

        let mut updated = Vec::new();

        if let Some(status_s) = status_str {
            match parse_status(status_s) {
                Some(status) => {
                    mgr.update_status(task_id, status);
                    updated.push(format!("status -> {}", status_s));
                }
                None => {
                    return Ok(ToolResult::error(&format!(
                        "Invalid status: '{}'",
                        status_s
                    )));
                }
            }
        }

        if let Some(result) = result_str {
            mgr.set_output(task_id, result.to_string());
            updated.push("output updated".to_string());
        }

        if updated.is_empty() {
            Ok(ToolResult::text(&format!(
                "Task '{}': no changes applied (provide 'status' or 'result')",
                task_id
            )))
        } else {
            Ok(ToolResult::text(&format!(
                "Task '{}' updated: {}",
                task_id,
                updated.join(", ")
            )))
        }
    }
}

// ──────────────── TaskStopTool ────────────────

pub struct TaskStopTool;

impl TaskStopTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TaskStopTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for TaskStopTool {
    fn name(&self) -> &str {
        "TaskStop"
    }

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
        match input.get("task_id").and_then(|v| v.as_str()) {
            Some(id) if !id.is_empty() => ValidationResult::Ok,
            _ => ValidationResult::Error {
                message: "Missing or empty 'task_id' parameter".into(),
            },
        }
    }

    async fn call(&self, input: Value) -> Result<ToolResult, ToolError> {
        let task_id =
            input
                .get("task_id")
                .and_then(|v| v.as_str())
                .ok_or(ToolError::ValidationFailed {
                    message: "Missing 'task_id' parameter".into(),
                })?;

        let mut mgr = TASK_MANAGER.lock().unwrap();
        match mgr.get_task(task_id) {
            Some(task) => match task.status {
                TaskStatus::Running | TaskStatus::Pending => {
                    mgr.update_status(task_id, TaskStatus::Cancelled);
                    Ok(ToolResult::text(&format!(
                        "Task '{}' has been cancelled",
                        task_id
                    )))
                }
                other => Ok(ToolResult::error(&format!(
                    "Task '{}' cannot be stopped (current status: {:?})",
                    task_id, other
                ))),
            },
            None => Ok(ToolResult::error(&format!("Task '{}' not found", task_id))),
        }
    }
}

// ──────────────── TaskListTool ────────────────

pub struct TaskListTool;

impl TaskListTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TaskListTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for TaskListTool {
    fn name(&self) -> &str {
        "TaskList"
    }

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
        let status_filter = input.get("status_filter").and_then(|v| v.as_str());

        let mgr = TASK_MANAGER.lock().unwrap();
        let all_tasks = mgr.list_tasks();

        let tasks: Vec<Value> = if let Some(filter_str) = status_filter {
            match parse_status(filter_str) {
                Some(status) => all_tasks
                    .iter()
                    .filter(|t| t.status == status)
                    .map(task_to_json)
                    .collect(),
                None => {
                    return Ok(ToolResult::error(&format!(
                        "Invalid status filter: '{}'",
                        filter_str
                    )));
                }
            }
        } else {
            all_tasks.iter().map(task_to_json).collect()
        };

        let result = json!({
            "count": tasks.len(),
            "tasks": tasks
        });

        let json_str = serde_json::to_string_pretty(&result)
            .unwrap_or_else(|_| "Error serializing tasks".to_string());
        Ok(ToolResult::text(&json_str))
    }
}

// ──────────────── TaskOutputTool ────────────────

pub struct TaskOutputTool;

impl TaskOutputTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TaskOutputTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for TaskOutputTool {
    fn name(&self) -> &str {
        "TaskOutput"
    }

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

    fn is_read_only(&self, _input: &Value) -> bool {
        true
    }
    fn is_concurrency_safe(&self, _input: &Value) -> bool {
        true
    }
    fn should_defer(&self) -> bool {
        true
    }

    fn validate_input(&self, input: &Value) -> ValidationResult {
        match input.get("task_id").and_then(|v| v.as_str()) {
            Some(id) if !id.is_empty() => ValidationResult::Ok,
            _ => ValidationResult::Error {
                message: "Missing or empty 'task_id' parameter".into(),
            },
        }
    }

    async fn call(&self, input: Value) -> Result<ToolResult, ToolError> {
        let task_id =
            input
                .get("task_id")
                .and_then(|v| v.as_str())
                .ok_or(ToolError::ValidationFailed {
                    message: "Missing 'task_id' parameter".into(),
                })?;
        let tail = input.get("tail").and_then(|v| v.as_u64());

        let mgr = TASK_MANAGER.lock().unwrap();
        match mgr.get_task(task_id) {
            Some(task) => match &task.output {
                Some(output) => {
                    let text = if let Some(n) = tail {
                        let lines: Vec<&str> = output.lines().collect();
                        let start = lines.len().saturating_sub(n as usize);
                        lines[start..].join("\n")
                    } else {
                        output.clone()
                    };
                    Ok(ToolResult::text(&format!(
                        "Task '{}' (status: {:?}):\n{}",
                        task_id, task.status, text
                    )))
                }
                None => Ok(ToolResult::text(&format!(
                    "Task '{}' (status: {:?}): no output yet",
                    task_id, task.status
                ))),
            },
            None => Ok(ToolResult::error(&format!("Task '{}' not found", task_id))),
        }
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
    async fn test_task_create_and_get() {
        let create = TaskCreateTool::new();
        let result = create
            .call(json!({"prompt": "test task", "description": "My task"}))
            .await
            .unwrap();
        assert!(!result.is_error);
        let text = result.content.as_str().unwrap();
        assert!(text.contains("Task created with ID:"));

        // Extract task ID from result
        let task_id = text
            .split("ID: ")
            .nth(1)
            .unwrap()
            .split('\n')
            .next()
            .unwrap();

        // Get the task
        let get = TaskGetTool::new();
        let result = get.call(json!({"task_id": task_id})).await.unwrap();
        assert!(!result.is_error);
        let text = result.content.as_str().unwrap();
        assert!(text.contains("pending"));
    }

    #[tokio::test]
    async fn test_task_update() {
        let create = TaskCreateTool::new();
        let result = create.call(json!({"prompt": "update test"})).await.unwrap();
        let text = result.content.as_str().unwrap();
        let task_id = text
            .split("ID: ")
            .nth(1)
            .unwrap()
            .split('\n')
            .next()
            .unwrap();

        let update = TaskUpdateTool::new();
        let result = update
            .call(json!({"task_id": task_id, "status": "running"}))
            .await
            .unwrap();
        assert!(!result.is_error);
        assert!(
            result
                .content
                .as_str()
                .unwrap()
                .contains("status -> running")
        );

        // Update with result
        let result = update
            .call(json!({"task_id": task_id, "result": "done!"}))
            .await
            .unwrap();
        assert!(!result.is_error);
        assert!(result.content.as_str().unwrap().contains("output updated"));
    }

    #[tokio::test]
    async fn test_task_stop() {
        let create = TaskCreateTool::new();
        let result = create.call(json!({"prompt": "stop test"})).await.unwrap();
        let text = result.content.as_str().unwrap();
        let task_id = text
            .split("ID: ")
            .nth(1)
            .unwrap()
            .split('\n')
            .next()
            .unwrap();

        let stop = TaskStopTool::new();
        let result = stop.call(json!({"task_id": task_id})).await.unwrap();
        assert!(!result.is_error);
        assert!(result.content.as_str().unwrap().contains("cancelled"));
    }

    #[tokio::test]
    async fn test_task_list() {
        let list = TaskListTool::new();
        let result = list.call(json!({})).await.unwrap();
        assert!(!result.is_error);
        assert!(result.content.as_str().unwrap().contains("count"));
    }

    #[tokio::test]
    async fn test_task_output() {
        let create = TaskCreateTool::new();
        let result = create.call(json!({"prompt": "output test"})).await.unwrap();
        let text = result.content.as_str().unwrap();
        let task_id = text
            .split("ID: ")
            .nth(1)
            .unwrap()
            .split('\n')
            .next()
            .unwrap();

        let output = TaskOutputTool::new();
        let result = output.call(json!({"task_id": task_id})).await.unwrap();
        assert!(!result.is_error);
        assert!(result.content.as_str().unwrap().contains("no output yet"));

        // Add output
        let update = TaskUpdateTool::new();
        update
            .call(json!({"task_id": task_id, "result": "line1\nline2\nline3"}))
            .await
            .unwrap();

        // Get output with tail
        let result = output
            .call(json!({"task_id": task_id, "tail": 2}))
            .await
            .unwrap();
        assert!(!result.is_error);
        let text = result.content.as_str().unwrap();
        assert!(text.contains("line2"));
        assert!(text.contains("line3"));
    }

    #[tokio::test]
    async fn test_task_get_not_found() {
        let get = TaskGetTool::new();
        let result = get
            .call(json!({"task_id": "nonexistent-id"}))
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(result.content.as_str().unwrap().contains("not found"));
    }

    #[test]
    fn test_parse_status() {
        assert_eq!(parse_status("pending"), Some(TaskStatus::Pending));
        assert_eq!(parse_status("running"), Some(TaskStatus::Running));
        assert_eq!(parse_status("completed"), Some(TaskStatus::Completed));
        assert_eq!(parse_status("failed"), Some(TaskStatus::Failed));
        assert_eq!(parse_status("cancelled"), Some(TaskStatus::Cancelled));
        assert_eq!(parse_status("invalid"), None);
    }
}
