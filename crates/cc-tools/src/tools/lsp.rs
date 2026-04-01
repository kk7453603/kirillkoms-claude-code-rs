use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use cc_utils::lsp::{
    detect_language, default_server_for_language, LspClient, Location, Position, StdioLspClient,
};
use crate::trait_def::{SearchReadInfo, Tool, ToolError, ToolResult, ValidationResult};

pub struct LspTool {
    clients: Arc<Mutex<HashMap<String, Arc<StdioLspClient>>>>,
}

impl LspTool {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Get or create a client for the given language.
    async fn get_client(&self, file_path: &str) -> Result<Arc<StdioLspClient>, ToolError> {
        let lang = detect_language(file_path);
        let (cmd, args) = default_server_for_language(lang).ok_or_else(|| {
            ToolError::ExecutionFailed {
                message: format!("No LSP server configured for language '{}'", lang),
            }
        })?;

        let mut clients = self.clients.lock().await;
        if let Some(client) = clients.get(lang) {
            return Ok(Arc::clone(client));
        }

        // Derive root_uri from file_path: use the parent directory.
        let root = std::path::Path::new(file_path)
            .parent()
            .unwrap_or(std::path::Path::new("/"))
            .to_string_lossy()
            .to_string();
        let root_uri = format!("file://{}", root);

        let client = Arc::new(StdioLspClient::new(
            cmd.to_string(),
            args.iter().map(|s| s.to_string()).collect(),
            root_uri,
        ));

        client.connect().await.map_err(|e| ToolError::ExecutionFailed {
            message: format!("Failed to connect to {} LSP server: {}", lang, e),
        })?;

        clients.insert(lang.to_string(), Arc::clone(&client));
        Ok(client)
    }
}

impl Default for LspTool {
    fn default() -> Self {
        Self::new()
    }
}

fn format_locations(locations: &[Location]) -> String {
    if locations.is_empty() {
        return "No results found.".to_string();
    }
    locations
        .iter()
        .map(|loc| {
            format!(
                "{}:{}:{}",
                loc.uri
                    .strip_prefix("file://")
                    .unwrap_or(&loc.uri),
                loc.range.start.line + 1,
                loc.range.start.character + 1,
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
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
        let file_path = input
            .get("filePath")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let line = input
            .get("line")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        let character = input
            .get("character")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;

        let client = match self.get_client(file_path).await {
            Ok(c) => c,
            Err(e) => {
                return Ok(ToolResult::error(&format!(
                    "LSP operation '{}' failed: {}",
                    operation, e
                )));
            }
        };

        let pos = Position { line, character };

        match operation {
            "goToDefinition" => {
                match client.go_to_definition(file_path, pos).await {
                    Ok(locs) => Ok(ToolResult::text(&format_locations(&locs))),
                    Err(e) => Ok(ToolResult::error(&format!("goToDefinition failed: {}", e))),
                }
            }
            "findReferences" => {
                match client.find_references(file_path, pos).await {
                    Ok(locs) => Ok(ToolResult::text(&format_locations(&locs))),
                    Err(e) => Ok(ToolResult::error(&format!("findReferences failed: {}", e))),
                }
            }
            "hover" => match client.hover(file_path, pos).await {
                Ok(Some(hover)) => Ok(ToolResult::text(&hover.contents)),
                Ok(None) => Ok(ToolResult::text("No hover information available.")),
                Err(e) => Ok(ToolResult::error(&format!("hover failed: {}", e))),
            },
            "documentSymbols" => match client.document_symbols(file_path).await {
                Ok(symbols) => {
                    if symbols.is_empty() {
                        Ok(ToolResult::text("No symbols found."))
                    } else {
                        let text = symbols
                            .iter()
                            .map(|s| {
                                format!(
                                    "{} ({}) at {}:{}",
                                    s.name,
                                    s.kind,
                                    s.location.range.start.line + 1,
                                    s.location.range.start.character + 1,
                                )
                            })
                            .collect::<Vec<_>>()
                            .join("\n");
                        Ok(ToolResult::text(&text))
                    }
                }
                Err(e) => Ok(ToolResult::error(&format!(
                    "documentSymbols failed: {}",
                    e
                ))),
            },
            "workspaceSymbols" => {
                let query = input
                    .get("query")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                match client.workspace_symbols(query).await {
                    Ok(symbols) => {
                        if symbols.is_empty() {
                            Ok(ToolResult::text("No symbols found."))
                        } else {
                            let text = symbols
                                .iter()
                                .map(|s| {
                                    format!(
                                        "{} ({}) in {}:{}",
                                        s.name,
                                        s.kind,
                                        s.location
                                            .uri
                                            .strip_prefix("file://")
                                            .unwrap_or(&s.location.uri),
                                        s.location.range.start.line + 1,
                                    )
                                })
                                .collect::<Vec<_>>()
                                .join("\n");
                            Ok(ToolResult::text(&text))
                        }
                    }
                    Err(e) => Ok(ToolResult::error(&format!(
                        "workspaceSymbols failed: {}",
                        e
                    ))),
                }
            }
            other => Ok(ToolResult::error(&format!(
                "LSP operation '{}' is not yet implemented.",
                other
            ))),
        }
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

        let ops = schema["properties"]["operation"]["enum"]
            .as_array()
            .unwrap();
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
    async fn test_call_unsupported_language() {
        let tool = LspTool::new();
        let result = tool
            .call(json!({"operation": "hover", "filePath": "/tmp/t.txt", "line": 1, "character": 5}))
            .await
            .unwrap();
        assert!(result.is_error);
        // plaintext has no LSP server
        assert!(result.content.as_str().unwrap().contains("failed"));
    }

    #[tokio::test]
    async fn test_call_unimplemented_operation() {
        // This will fail at the connect step (no real server), so we can test
        // the error path. We use a file with a known language to trigger the
        // server lookup.
        let tool = LspTool::new();
        let result = tool
            .call(json!({
                "operation": "formatting",
                "filePath": "/tmp/test.rs"
            }))
            .await
            .unwrap();
        // Should error because rust-analyzer is not available in test env
        assert!(result.is_error);
    }

    #[test]
    fn test_should_defer() {
        let tool = LspTool::new();
        assert!(tool.should_defer());
    }

    #[test]
    fn test_format_locations_empty() {
        assert_eq!(format_locations(&[]), "No results found.");
    }

    #[test]
    fn test_format_locations() {
        let locs = vec![
            Location {
                uri: "file:///src/main.rs".to_string(),
                range: cc_utils::lsp::Range {
                    start: Position {
                        line: 9,
                        character: 4,
                    },
                    end: Position {
                        line: 9,
                        character: 10,
                    },
                },
            },
            Location {
                uri: "file:///src/lib.rs".to_string(),
                range: cc_utils::lsp::Range {
                    start: Position {
                        line: 0,
                        character: 0,
                    },
                    end: Position {
                        line: 0,
                        character: 5,
                    },
                },
            },
        ];
        let text = format_locations(&locs);
        assert!(text.contains("/src/main.rs:10:5"));
        assert!(text.contains("/src/lib.rs:1:1"));
    }

    #[test]
    fn test_default_creates_empty_clients() {
        let tool = LspTool::default();
        assert_eq!(tool.name(), "LSP");
    }
}
