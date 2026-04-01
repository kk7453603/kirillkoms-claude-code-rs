use async_trait::async_trait;
use serde_json::{json, Value};

use crate::trait_def::{SearchReadInfo, Tool, ToolError, ToolResult, ValidationResult};

pub struct LspTool;

impl LspTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LspTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for LspTool {
    fn name(&self) -> &str {
        "LSP"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "description": "The LSP operation to perform",
                    "enum": [
                        "goToDefinition",
                        "findReferences",
                        "hover",
                        "documentSymbols",
                        "workspaceSymbols",
                        "completion",
                        "signatureHelp",
                        "diagnostics",
                        "codeAction",
                        "rename",
                        "formatting",
                        "rangeFormatting"
                    ]
                },
                "filePath": {
                    "type": "string",
                    "description": "The absolute path to the file"
                },
                "line": {
                    "type": "number",
                    "description": "The line number (0-based)"
                },
                "character": {
                    "type": "number",
                    "description": "The character offset within the line (0-based)"
                },
                "query": {
                    "type": "string",
                    "description": "Query string for workspace symbol search"
                },
                "newName": {
                    "type": "string",
                    "description": "New name for rename operations"
                }
            },
            "required": ["operation", "filePath"]
        })
    }

    fn description(&self) -> String {
        "Interact with Language Server Protocol servers for code intelligence operations like go-to-definition, find-references, hover, and more.".to_string()
    }

    fn is_read_only(&self, input: &Value) -> bool {
        let op = input
            .get("operation")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        matches!(
            op,
            "goToDefinition"
                | "findReferences"
                | "hover"
                | "documentSymbols"
                | "workspaceSymbols"
                | "completion"
                | "signatureHelp"
                | "diagnostics"
        )
    }

    fn is_concurrency_safe(&self, _input: &Value) -> bool {
        true
    }

    fn should_defer(&self) -> bool {
        true
    }

    fn search_read_info(&self, input: &Value) -> SearchReadInfo {
        let op = input
            .get("operation")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        SearchReadInfo {
            is_search: matches!(op, "findReferences" | "workspaceSymbols"),
            is_read: matches!(op, "goToDefinition" | "hover" | "documentSymbols"),
            is_list: matches!(op, "documentSymbols" | "workspaceSymbols" | "diagnostics"),
        }
    }

    fn validate_input(&self, input: &Value) -> ValidationResult {
        let op = input.get("operation").and_then(|v| v.as_str());
        let path = input.get("filePath").and_then(|v| v.as_str());

        if op.is_none() || op == Some("") {
            return ValidationResult::Error {
                message: "Missing or empty 'operation' parameter".to_string(),
            };
        }
        if path.is_none() || path == Some("") {
            return ValidationResult::Error {
                message: "Missing or empty 'filePath' parameter".to_string(),
            };
        }
        ValidationResult::Ok
    }

    async fn call(&self, input: Value) -> Result<ToolResult, ToolError> {
        let operation = input
            .get("operation")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        Ok(ToolResult::error(&format!(
            "LSP operation '{}' is not yet available. The LSP tool requires a language server to be running and connected. Start a language server for your project's language and configure it in the settings.",
            operation
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name_and_schema() {
        let tool = LspTool::new();
        assert_eq!(tool.name(), "LSP");
        let schema = tool.input_schema();
        assert!(schema["properties"]["operation"].is_object());
        assert!(schema["properties"]["filePath"].is_object());
        assert!(schema["properties"]["line"].is_object());

        let ops = schema["properties"]["operation"]["enum"].as_array().unwrap();
        assert!(ops.contains(&json!("goToDefinition")));
        assert!(ops.contains(&json!("findReferences")));
        assert!(ops.contains(&json!("hover")));
    }

    #[test]
    fn test_validate_input() {
        let tool = LspTool::new();
        assert!(matches!(
            tool.validate_input(&json!({"operation": "hover", "filePath": "/tmp/test.rs"})),
            ValidationResult::Ok
        ));
        assert!(matches!(
            tool.validate_input(&json!({"operation": "hover"})),
            ValidationResult::Error { .. }
        ));
    }

    #[test]
    fn test_is_read_only() {
        let tool = LspTool::new();
        assert!(tool.is_read_only(&json!({"operation": "hover"})));
        assert!(tool.is_read_only(&json!({"operation": "goToDefinition"})));
        assert!(!tool.is_read_only(&json!({"operation": "rename"})));
    }

    #[tokio::test]
    async fn test_call_returns_stub() {
        let tool = LspTool::new();
        let result = tool
            .call(json!({"operation": "hover", "filePath": "/tmp/t.rs", "line": 1, "character": 5}))
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(result.content.as_str().unwrap().contains("hover"));
    }

    #[test]
    fn test_should_defer() {
        let tool = LspTool::new();
        assert!(tool.should_defer());
    }
}
