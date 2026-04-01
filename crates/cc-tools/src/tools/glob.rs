use async_trait::async_trait;
use serde_json::{json, Value};

use crate::trait_def::{
    RenderedContent, SearchReadInfo, Tool, ToolError, ToolResult, ValidationResult,
};

pub struct GlobTool;

impl GlobTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GlobTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &str {
        "Glob"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "The glob pattern to match files against (e.g., '**/*.rs', 'src/**/*.ts')"
                },
                "path": {
                    "type": "string",
                    "description": "The directory to search in. Defaults to the current working directory."
                }
            },
            "required": ["pattern"]
        })
    }

    fn description(&self) -> String {
        "Fast file pattern matching tool that works with any codebase size. Returns matching file paths sorted by modification time.".to_string()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        true
    }

    fn is_concurrency_safe(&self, _input: &Value) -> bool {
        true
    }

    fn search_read_info(&self, _input: &Value) -> SearchReadInfo {
        SearchReadInfo {
            is_search: true,
            is_read: false,
            is_list: true,
        }
    }

    fn validate_input(&self, input: &Value) -> ValidationResult {
        match input.get("pattern").and_then(|v| v.as_str()) {
            Some(p) if !p.is_empty() => ValidationResult::Ok,
            _ => ValidationResult::Error {
                message: "Missing or empty 'pattern' parameter".to_string(),
            },
        }
    }

    fn render_tool_use(&self, input: &Value) -> RenderedContent {
        let pattern = input
            .get("pattern")
            .and_then(|v| v.as_str())
            .unwrap_or("<unknown>");
        RenderedContent::Text(format!("Glob: {}", pattern))
    }

    async fn call(&self, input: Value) -> Result<ToolResult, ToolError> {
        let pattern = input
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or(ToolError::ValidationFailed {
                message: "Missing 'pattern' parameter".into(),
            })?;

        let base_path = input
            .get("path")
            .and_then(|v| v.as_str())
            .map(|s| std::path::PathBuf::from(s))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        let full_pattern = if pattern.starts_with('/') {
            pattern.to_string()
        } else {
            format!("{}/{}", base_path.display(), pattern)
        };

        let paths = glob::glob(&full_pattern).map_err(|e| ToolError::ExecutionFailed {
            message: format!("Invalid glob pattern '{}': {}", pattern, e),
        })?;

        let mut entries: Vec<(std::path::PathBuf, std::time::SystemTime)> = Vec::new();
        for entry in paths {
            match entry {
                Ok(path) => {
                    let mtime = path
                        .metadata()
                        .and_then(|m| m.modified())
                        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                    entries.push((path, mtime));
                }
                Err(_) => continue,
            }
        }

        // Sort by modification time, most recent first
        entries.sort_by(|a, b| b.1.cmp(&a.1));

        if entries.is_empty() {
            return Ok(ToolResult::text("No files matched the pattern."));
        }

        let result: Vec<String> = entries
            .iter()
            .take(1000)
            .map(|(p, _)| p.display().to_string())
            .collect();

        let mut output = result.join("\n");
        if entries.len() > 1000 {
            output.push_str(&format!(
                "\n\n... and {} more files",
                entries.len() - 1000
            ));
        }

        Ok(ToolResult::text(&output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_name() {
        let tool = GlobTool::new();
        assert_eq!(tool.name(), "Glob");
    }

    #[test]
    fn test_schema() {
        let tool = GlobTool::new();
        let schema = tool.input_schema();
        assert!(schema["properties"]["pattern"].is_object());
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("pattern")));
    }

    #[test]
    fn test_is_read_only() {
        let tool = GlobTool::new();
        assert!(tool.is_read_only(&json!({})));
    }

    #[tokio::test]
    async fn test_glob_finds_files() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("foo.txt"), "hello").unwrap();
        fs::write(dir.path().join("bar.txt"), "world").unwrap();
        fs::write(dir.path().join("baz.rs"), "fn main() {}").unwrap();

        let tool = GlobTool::new();
        let result = tool
            .call(json!({
                "pattern": "*.txt",
                "path": dir.path().to_str().unwrap()
            }))
            .await
            .unwrap();
        assert!(!result.is_error);
        let text = result.content.as_str().unwrap();
        assert!(text.contains("foo.txt"));
        assert!(text.contains("bar.txt"));
        assert!(!text.contains("baz.rs"));
    }

    #[tokio::test]
    async fn test_glob_no_matches() {
        let dir = TempDir::new().unwrap();

        let tool = GlobTool::new();
        let result = tool
            .call(json!({
                "pattern": "*.xyz",
                "path": dir.path().to_str().unwrap()
            }))
            .await
            .unwrap();
        assert!(!result.is_error);
        assert!(result.content.as_str().unwrap().contains("No files"));
    }

    #[tokio::test]
    async fn test_glob_recursive() {
        let dir = TempDir::new().unwrap();
        let sub = dir.path().join("sub");
        fs::create_dir(&sub).unwrap();
        fs::write(dir.path().join("a.txt"), "").unwrap();
        fs::write(sub.join("b.txt"), "").unwrap();

        let tool = GlobTool::new();
        let result = tool
            .call(json!({
                "pattern": "**/*.txt",
                "path": dir.path().to_str().unwrap()
            }))
            .await
            .unwrap();
        assert!(!result.is_error);
        let text = result.content.as_str().unwrap();
        assert!(text.contains("a.txt"));
        assert!(text.contains("b.txt"));
    }
}
