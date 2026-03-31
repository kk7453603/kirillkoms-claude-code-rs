use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::Path;

use crate::trait_def::{RenderedContent, Tool, ToolError, ToolResult, ValidationResult};

pub struct FileEditTool;

impl FileEditTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FileEditTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for FileEditTool {
    fn name(&self) -> &str {
        "Edit"
    }

    fn aliases(&self) -> Vec<&str> {
        vec!["FileEdit", "EditFile"]
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "The absolute path to the file to modify"
                },
                "old_string": {
                    "type": "string",
                    "description": "The text to replace"
                },
                "new_string": {
                    "type": "string",
                    "description": "The text to replace it with (must be different from old_string)"
                },
                "replace_all": {
                    "type": "boolean",
                    "description": "Replace all occurrences of old_string (default false)"
                }
            },
            "required": ["file_path", "old_string", "new_string"]
        })
    }

    fn description(&self) -> String {
        "Performs exact string replacements in files.".to_string()
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
        let old_string = input.get("old_string").and_then(|v| v.as_str());
        let new_string = input.get("new_string").and_then(|v| v.as_str());

        if file_path.is_none() || file_path == Some("") {
            return ValidationResult::Error {
                message: "Missing or empty 'file_path' parameter".to_string(),
            };
        }
        if old_string.is_none() {
            return ValidationResult::Error {
                message: "Missing 'old_string' parameter".to_string(),
            };
        }
        if new_string.is_none() {
            return ValidationResult::Error {
                message: "Missing 'new_string' parameter".to_string(),
            };
        }
        if old_string == new_string {
            return ValidationResult::Error {
                message: "'old_string' and 'new_string' must be different".to_string(),
            };
        }

        ValidationResult::Ok
    }

    fn render_tool_use(&self, input: &Value) -> RenderedContent {
        let path = input
            .get("file_path")
            .and_then(|v| v.as_str())
            .unwrap_or("<unknown>");
        let old = input
            .get("old_string")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let new = input
            .get("new_string")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        RenderedContent::Diff {
            old: old.to_string(),
            new: new.to_string(),
            file_path: Some(path.to_string()),
        }
    }

    async fn call(&self, input: Value) -> Result<ToolResult, ToolError> {
        let file_path = input
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or(ToolError::ValidationFailed {
                message: "Missing 'file_path' parameter".into(),
            })?;
        let old_string = input
            .get("old_string")
            .and_then(|v| v.as_str())
            .ok_or(ToolError::ValidationFailed {
                message: "Missing 'old_string' parameter".into(),
            })?;
        let new_string = input
            .get("new_string")
            .and_then(|v| v.as_str())
            .ok_or(ToolError::ValidationFailed {
                message: "Missing 'new_string' parameter".into(),
            })?;
        let replace_all = input
            .get("replace_all")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let path = Path::new(file_path);
        if !path.exists() {
            return Ok(ToolResult::error(&format!(
                "File not found: {}",
                file_path
            )));
        }

        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                message: format!("Failed to read file '{}': {}", file_path, e),
            })?;

        if !content.contains(old_string) {
            return Ok(ToolResult::error(
                "The old_string was not found in the file. Make sure it matches exactly, including whitespace and indentation.",
            ));
        }

        let count = content.matches(old_string).count();

        // Check uniqueness when not replacing all
        if !replace_all && count > 1 {
            return Ok(ToolResult::error(&format!(
                "The old_string was found {} times in the file. Provide a larger string with more context to make it unique, or set replace_all to true.",
                count
            )));
        }

        let new_content = if replace_all {
            content.replace(old_string, new_string)
        } else {
            content.replacen(old_string, new_string, 1)
        };

        tokio::fs::write(path, &new_content)
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                message: format!("Failed to write file '{}': {}", file_path, e),
            })?;

        let replacements = if replace_all { count } else { 1 };

        Ok(ToolResult::text(&format!(
            "Successfully edited {}. Made {} replacement(s).",
            file_path, replacements
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_name() {
        let tool = FileEditTool::new();
        assert_eq!(tool.name(), "Edit");
    }

    #[test]
    fn test_schema() {
        let tool = FileEditTool::new();
        let schema = tool.input_schema();
        assert!(schema["properties"]["file_path"].is_object());
        assert!(schema["properties"]["old_string"].is_object());
        assert!(schema["properties"]["new_string"].is_object());
        let required = schema["required"].as_array().unwrap();
        assert_eq!(required.len(), 3);
    }

    #[test]
    fn test_not_read_only() {
        let tool = FileEditTool::new();
        assert!(!tool.is_read_only(&json!({})));
    }

    #[test]
    fn test_validate_input() {
        let tool = FileEditTool::new();
        assert!(matches!(
            tool.validate_input(&json!({
                "file_path": "/tmp/test.txt",
                "old_string": "foo",
                "new_string": "bar"
            })),
            ValidationResult::Ok
        ));
        assert!(matches!(
            tool.validate_input(&json!({
                "file_path": "/tmp/test.txt",
                "old_string": "foo",
                "new_string": "foo"
            })),
            ValidationResult::Error { .. }
        ));
    }

    #[tokio::test]
    async fn test_edit_file() {
        let mut tmp = NamedTempFile::new().unwrap();
        write!(tmp, "hello world").unwrap();

        let tool = FileEditTool::new();
        let result = tool
            .call(json!({
                "file_path": tmp.path().to_str().unwrap(),
                "old_string": "hello",
                "new_string": "goodbye"
            }))
            .await
            .unwrap();
        assert!(!result.is_error);

        let content = std::fs::read_to_string(tmp.path()).unwrap();
        assert_eq!(content, "goodbye world");
    }

    #[tokio::test]
    async fn test_edit_replace_all() {
        let mut tmp = NamedTempFile::new().unwrap();
        write!(tmp, "aaa bbb aaa").unwrap();

        let tool = FileEditTool::new();
        let result = tool
            .call(json!({
                "file_path": tmp.path().to_str().unwrap(),
                "old_string": "aaa",
                "new_string": "ccc",
                "replace_all": true
            }))
            .await
            .unwrap();
        assert!(!result.is_error);

        let content = std::fs::read_to_string(tmp.path()).unwrap();
        assert_eq!(content, "ccc bbb ccc");
    }

    #[tokio::test]
    async fn test_edit_non_unique_fails() {
        let mut tmp = NamedTempFile::new().unwrap();
        write!(tmp, "aaa bbb aaa").unwrap();

        let tool = FileEditTool::new();
        let result = tool
            .call(json!({
                "file_path": tmp.path().to_str().unwrap(),
                "old_string": "aaa",
                "new_string": "ccc"
            }))
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(result.content.as_str().unwrap().contains("found 2 times"));
    }

    #[tokio::test]
    async fn test_edit_not_found() {
        let mut tmp = NamedTempFile::new().unwrap();
        write!(tmp, "hello world").unwrap();

        let tool = FileEditTool::new();
        let result = tool
            .call(json!({
                "file_path": tmp.path().to_str().unwrap(),
                "old_string": "xyz",
                "new_string": "abc"
            }))
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(result.content.as_str().unwrap().contains("not found"));
    }

    #[tokio::test]
    async fn test_edit_nonexistent_file() {
        let tool = FileEditTool::new();
        let result = tool
            .call(json!({
                "file_path": "/tmp/nonexistent_cc_test_xyz.txt",
                "old_string": "a",
                "new_string": "b"
            }))
            .await
            .unwrap();
        assert!(result.is_error);
    }
}
