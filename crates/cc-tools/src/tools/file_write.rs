use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::Path;

use crate::trait_def::{RenderedContent, Tool, ToolError, ToolResult, ValidationResult};

pub struct FileWriteTool;

impl FileWriteTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FileWriteTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for FileWriteTool {
    fn name(&self) -> &str {
        "Write"
    }

    fn aliases(&self) -> Vec<&str> {
        vec!["FileWrite", "WriteFile"]
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "The absolute path to the file to write"
                },
                "content": {
                    "type": "string",
                    "description": "The content to write to the file"
                }
            },
            "required": ["file_path", "content"]
        })
    }

    fn description(&self) -> String {
        "Writes a file to the local filesystem. Creates parent directories if needed.".to_string()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        false
    }

    fn is_concurrency_safe(&self, _input: &Value) -> bool {
        false
    }

    fn is_destructive(&self, _input: &Value) -> bool {
        false
    }

    fn validate_input(&self, input: &Value) -> ValidationResult {
        let file_path = input.get("file_path").and_then(|v| v.as_str());
        let content = input.get("content");

        if file_path.is_none() || file_path == Some("") {
            return ValidationResult::Error {
                message: "Missing or empty 'file_path' parameter".to_string(),
            };
        }
        if content.is_none() {
            return ValidationResult::Error {
                message: "Missing 'content' parameter".to_string(),
            };
        }

        ValidationResult::Ok
    }

    fn render_tool_use(&self, input: &Value) -> RenderedContent {
        let path = input
            .get("file_path")
            .and_then(|v| v.as_str())
            .unwrap_or("<unknown>");
        RenderedContent::Styled {
            text: format!("Write: {}", path),
            bold: true,
            dim: false,
            color: Some("yellow".to_string()),
        }
    }

    async fn call(&self, input: Value) -> Result<ToolResult, ToolError> {
        let file_path = input
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or(ToolError::ValidationFailed {
                message: "Missing 'file_path' parameter".into(),
            })?;
        let content = input
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or(ToolError::ValidationFailed {
                message: "Missing 'content' parameter".into(),
            })?;

        let path = Path::new(file_path);

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|e| ToolError::ExecutionFailed {
                        message: format!(
                            "Failed to create parent directories for '{}': {}",
                            file_path, e
                        ),
                    })?;
            }
        }

        let is_new = !path.exists();

        tokio::fs::write(path, content)
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                message: format!("Failed to write file '{}': {}", file_path, e),
            })?;

        let action = if is_new { "Created" } else { "Wrote" };
        let bytes = content.len();
        Ok(ToolResult::text(&format!(
            "{} {} ({} bytes)",
            action, file_path, bytes
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_name() {
        let tool = FileWriteTool::new();
        assert_eq!(tool.name(), "Write");
    }

    #[test]
    fn test_schema() {
        let tool = FileWriteTool::new();
        let schema = tool.input_schema();
        assert!(schema["properties"]["file_path"].is_object());
        assert!(schema["properties"]["content"].is_object());
        let required = schema["required"].as_array().unwrap();
        assert_eq!(required.len(), 2);
    }

    #[test]
    fn test_not_read_only() {
        let tool = FileWriteTool::new();
        assert!(!tool.is_read_only(&json!({})));
    }

    #[tokio::test]
    async fn test_write_new_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.txt");

        let tool = FileWriteTool::new();
        let result = tool
            .call(json!({
                "file_path": path.to_str().unwrap(),
                "content": "hello world"
            }))
            .await
            .unwrap();
        assert!(!result.is_error);
        assert!(result.content.as_str().unwrap().contains("Created"));

        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "hello world");
    }

    #[tokio::test]
    async fn test_write_creates_parent_dirs() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("sub").join("dir").join("test.txt");

        let tool = FileWriteTool::new();
        let result = tool
            .call(json!({
                "file_path": path.to_str().unwrap(),
                "content": "nested content"
            }))
            .await
            .unwrap();
        assert!(!result.is_error);

        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "nested content");
    }

    #[tokio::test]
    async fn test_write_overwrites_existing() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, "old content").unwrap();

        let tool = FileWriteTool::new();
        let result = tool
            .call(json!({
                "file_path": path.to_str().unwrap(),
                "content": "new content"
            }))
            .await
            .unwrap();
        assert!(!result.is_error);
        assert!(result.content.as_str().unwrap().contains("Wrote"));

        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "new content");
    }

    #[tokio::test]
    async fn test_write_missing_content() {
        let tool = FileWriteTool::new();
        let result = tool.call(json!({"file_path": "/tmp/test.txt"})).await;
        assert!(result.is_err());
    }
}
