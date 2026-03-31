use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::Path;

use crate::trait_def::{
    RenderedContent, SearchReadInfo, Tool, ToolError, ToolResult, ValidationResult,
};

pub struct FileReadTool;

impl FileReadTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FileReadTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for FileReadTool {
    fn name(&self) -> &str {
        "Read"
    }

    fn aliases(&self) -> Vec<&str> {
        vec!["FileRead", "ReadFile"]
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "The absolute path to the file to read"
                },
                "offset": {
                    "type": "number",
                    "description": "The line number to start reading from (1-based)"
                },
                "limit": {
                    "type": "number",
                    "description": "The number of lines to read"
                }
            },
            "required": ["file_path"]
        })
    }

    fn description(&self) -> String {
        "Reads a file from the local filesystem. Returns the file content with line numbers."
            .to_string()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        true
    }

    fn is_concurrency_safe(&self, _input: &Value) -> bool {
        true
    }

    fn is_destructive(&self, _input: &Value) -> bool {
        false
    }

    fn validate_input(&self, input: &Value) -> ValidationResult {
        match input.get("file_path").and_then(|v| v.as_str()) {
            Some(p) if !p.is_empty() => ValidationResult::Ok,
            _ => ValidationResult::Error {
                message: "Missing or empty 'file_path' parameter".to_string(),
            },
        }
    }

    fn search_read_info(&self, _input: &Value) -> SearchReadInfo {
        SearchReadInfo {
            is_search: false,
            is_read: true,
            is_list: false,
        }
    }

    fn render_tool_use(&self, input: &Value) -> RenderedContent {
        let path = input
            .get("file_path")
            .and_then(|v| v.as_str())
            .unwrap_or("<unknown>");
        RenderedContent::Styled {
            text: format!("Read: {}", path),
            bold: true,
            dim: false,
            color: Some("blue".to_string()),
        }
    }

    async fn call(&self, input: Value) -> Result<ToolResult, ToolError> {
        let file_path = input
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or(ToolError::ValidationFailed {
                message: "Missing 'file_path' parameter".into(),
            })?;

        let path = Path::new(file_path);
        if !path.exists() {
            return Ok(ToolResult::error(&format!(
                "File not found: {}",
                file_path
            )));
        }

        if path.is_dir() {
            return Ok(ToolResult::error(&format!(
                "Path is a directory, not a file: {}. Use ls via Bash to list directory contents.",
                file_path
            )));
        }

        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                message: format!("Failed to read file '{}': {}", file_path, e),
            })?;

        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        let offset = input
            .get("offset")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(1);
        let limit = input
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(2000);

        // offset is 1-based
        let start = if offset > 0 { offset - 1 } else { 0 };
        let end = (start + limit).min(total_lines);

        let mut result = String::new();
        for (i, line) in lines.iter().enumerate().skip(start).take(end - start) {
            result.push_str(&format!("{}\t{}\n", i + 1, line));
        }

        if end < total_lines {
            result.push_str(&format!(
                "\n... ({} more lines, {} total)",
                total_lines - end,
                total_lines
            ));
        }

        Ok(ToolResult::text(&result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_name() {
        let tool = FileReadTool::new();
        assert_eq!(tool.name(), "Read");
    }

    #[test]
    fn test_aliases() {
        let tool = FileReadTool::new();
        assert!(tool.aliases().contains(&"FileRead"));
    }

    #[test]
    fn test_schema() {
        let tool = FileReadTool::new();
        let schema = tool.input_schema();
        assert!(schema["properties"]["file_path"].is_object());
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("file_path")));
    }

    #[test]
    fn test_is_read_only() {
        let tool = FileReadTool::new();
        assert!(tool.is_read_only(&json!({})));
    }

    #[test]
    fn test_validate_input() {
        let tool = FileReadTool::new();
        assert!(matches!(
            tool.validate_input(&json!({"file_path": "/tmp/test.txt"})),
            ValidationResult::Ok
        ));
        assert!(matches!(
            tool.validate_input(&json!({})),
            ValidationResult::Error { .. }
        ));
    }

    #[tokio::test]
    async fn test_read_file() {
        let mut tmp = NamedTempFile::new().unwrap();
        writeln!(tmp, "line one").unwrap();
        writeln!(tmp, "line two").unwrap();
        writeln!(tmp, "line three").unwrap();

        let tool = FileReadTool::new();
        let result = tool
            .call(json!({"file_path": tmp.path().to_str().unwrap()}))
            .await
            .unwrap();
        assert!(!result.is_error);
        let text = result.content.as_str().unwrap();
        assert!(text.contains("line one"));
        assert!(text.contains("line two"));
        assert!(text.contains("line three"));
    }

    #[tokio::test]
    async fn test_read_with_offset_and_limit() {
        let mut tmp = NamedTempFile::new().unwrap();
        for i in 1..=10 {
            writeln!(tmp, "line {}", i).unwrap();
        }

        let tool = FileReadTool::new();
        let result = tool
            .call(json!({"file_path": tmp.path().to_str().unwrap(), "offset": 3, "limit": 2}))
            .await
            .unwrap();
        assert!(!result.is_error);
        let text = result.content.as_str().unwrap();
        assert!(text.contains("line 3"));
        assert!(text.contains("line 4"));
    }

    #[tokio::test]
    async fn test_read_nonexistent_file() {
        let tool = FileReadTool::new();
        let result = tool
            .call(json!({"file_path": "/tmp/nonexistent_cc_test_file_xyz.txt"}))
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(result.content.as_str().unwrap().contains("not found"));
    }

    #[tokio::test]
    async fn test_read_directory() {
        let tool = FileReadTool::new();
        let result = tool.call(json!({"file_path": "/tmp"})).await.unwrap();
        assert!(result.is_error);
        assert!(result.content.as_str().unwrap().contains("directory"));
    }
}
