use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::Path;

use crate::trait_def::{RenderedContent, Tool, ToolError, ToolResult, ValidationResult};

pub struct NotebookEditTool;

impl NotebookEditTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NotebookEditTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for NotebookEditTool {
    fn name(&self) -> &str {
        "NotebookEdit"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "notebook_path": {
                    "type": "string",
                    "description": "The absolute path to the Jupyter notebook (.ipynb) file"
                },
                "cell_id": {
                    "type": "string",
                    "description": "The ID of the cell to edit. For insert mode, the new cell is inserted after this cell. For replace/delete, this is the target cell."
                },
                "new_source": {
                    "type": "string",
                    "description": "The new source content for the cell"
                },
                "cell_type": {
                    "type": "string",
                    "description": "The cell type (code or markdown). Defaults to code.",
                    "enum": ["code", "markdown"]
                },
                "edit_mode": {
                    "type": "string",
                    "description": "The edit operation: replace (default), insert (after cell_id), or delete.",
                    "enum": ["replace", "insert", "delete"]
                }
            },
            "required": ["notebook_path", "new_source"]
        })
    }

    fn description(&self) -> String {
        "Edit Jupyter notebook (.ipynb) files. Supports replacing, inserting, and deleting cells."
            .to_string()
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
        let path = input.get("notebook_path").and_then(|v| v.as_str());
        if path.is_none() || path == Some("") {
            return ValidationResult::Error {
                message: "Missing or empty 'notebook_path' parameter".to_string(),
            };
        }
        if input.get("new_source").is_none() {
            return ValidationResult::Error {
                message: "Missing 'new_source' parameter".to_string(),
            };
        }
        ValidationResult::Ok
    }

    fn render_tool_use(&self, input: &Value) -> RenderedContent {
        let path = input
            .get("notebook_path")
            .and_then(|v| v.as_str())
            .unwrap_or("<unknown>");
        let mode = input
            .get("edit_mode")
            .and_then(|v| v.as_str())
            .unwrap_or("replace");
        RenderedContent::Styled {
            text: format!("NotebookEdit ({}): {}", mode, path),
            bold: true,
            dim: false,
            color: Some("yellow".to_string()),
        }
    }

    async fn call(&self, input: Value) -> Result<ToolResult, ToolError> {
        let notebook_path = input
            .get("notebook_path")
            .and_then(|v| v.as_str())
            .ok_or(ToolError::ValidationFailed {
                message: "Missing 'notebook_path' parameter".into(),
            })?;

        let new_source = input
            .get("new_source")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let cell_id = input.get("cell_id").and_then(|v| v.as_str());
        let cell_type = input
            .get("cell_type")
            .and_then(|v| v.as_str())
            .unwrap_or("code");
        let edit_mode = input
            .get("edit_mode")
            .and_then(|v| v.as_str())
            .unwrap_or("replace");

        let path = Path::new(notebook_path);
        if !path.exists() {
            return Ok(ToolResult::error(&format!(
                "Notebook not found: {}",
                notebook_path
            )));
        }

        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                message: format!("Failed to read notebook '{}': {}", notebook_path, e),
            })?;

        let mut notebook: Value =
            serde_json::from_str(&content).map_err(|e| ToolError::ExecutionFailed {
                message: format!("Failed to parse notebook JSON: {}", e),
            })?;

        let cells = notebook
            .get_mut("cells")
            .and_then(|c| c.as_array_mut())
            .ok_or(ToolError::ExecutionFailed {
                message: "Notebook has no 'cells' array".into(),
            })?;

        // Split new_source into lines for the notebook format
        let source_lines: Vec<Value> = new_source
            .lines()
            .enumerate()
            .map(|(i, line)| {
                let total = new_source.lines().count();
                if i < total - 1 {
                    Value::String(format!("{}\n", line))
                } else {
                    Value::String(line.to_string())
                }
            })
            .collect();

        let new_cell = json!({
            "cell_type": cell_type,
            "metadata": {},
            "source": source_lines,
            "outputs": if cell_type == "code" { json!([]) } else { json!(null) },
            "id": uuid::Uuid::new_v4().to_string()
        });

        match edit_mode {
            "insert" => {
                if let Some(target_id) = cell_id {
                    let idx = find_cell_index(cells, target_id);
                    match idx {
                        Some(i) => {
                            cells.insert(i + 1, new_cell);
                        }
                        None => {
                            return Ok(ToolResult::error(&format!(
                                "Cell with id '{}' not found in notebook",
                                target_id
                            )));
                        }
                    }
                } else {
                    // Insert at end
                    cells.push(new_cell);
                }
            }
            "delete" => {
                if let Some(target_id) = cell_id {
                    let idx = find_cell_index(cells, target_id);
                    match idx {
                        Some(i) => {
                            cells.remove(i);
                        }
                        None => {
                            return Ok(ToolResult::error(&format!(
                                "Cell with id '{}' not found in notebook",
                                target_id
                            )));
                        }
                    }
                } else {
                    return Ok(ToolResult::error(
                        "cell_id is required for delete mode",
                    ));
                }
            }
            _ => {
                // replace mode
                if let Some(target_id) = cell_id {
                    let idx = find_cell_index(cells, target_id);
                    match idx {
                        Some(i) => {
                            if let Some(cell) = cells.get_mut(i) {
                                cell["source"] = json!(source_lines);
                                if cell_type != "code" || input.get("cell_type").is_some() {
                                    cell["cell_type"] = json!(cell_type);
                                }
                            }
                        }
                        None => {
                            return Ok(ToolResult::error(&format!(
                                "Cell with id '{}' not found in notebook",
                                target_id
                            )));
                        }
                    }
                } else if !cells.is_empty() {
                    // Replace first cell if no cell_id specified
                    cells[0]["source"] = json!(source_lines);
                    if input.get("cell_type").is_some() {
                        cells[0]["cell_type"] = json!(cell_type);
                    }
                } else {
                    cells.push(new_cell);
                }
            }
        }

        let output =
            serde_json::to_string_pretty(&notebook).map_err(|e| ToolError::ExecutionFailed {
                message: format!("Failed to serialize notebook: {}", e),
            })?;

        tokio::fs::write(path, output)
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                message: format!("Failed to write notebook '{}': {}", notebook_path, e),
            })?;

        Ok(ToolResult::text(&format!(
            "Successfully {} cell in {}",
            match edit_mode {
                "insert" => "inserted",
                "delete" => "deleted",
                _ => "replaced",
            },
            notebook_path
        )))
    }
}

fn find_cell_index(cells: &[Value], cell_id: &str) -> Option<usize> {
    cells
        .iter()
        .position(|c| c.get("id").and_then(|id| id.as_str()) == Some(cell_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_notebook() -> Value {
        json!({
            "nbformat": 4,
            "nbformat_minor": 5,
            "metadata": {
                "kernelspec": {
                    "display_name": "Python 3",
                    "language": "python",
                    "name": "python3"
                }
            },
            "cells": [
                {
                    "id": "cell-001",
                    "cell_type": "code",
                    "metadata": {},
                    "source": ["print('hello')\n"],
                    "outputs": []
                },
                {
                    "id": "cell-002",
                    "cell_type": "markdown",
                    "metadata": {},
                    "source": ["# Title\n"],
                    "outputs": null
                }
            ]
        })
    }

    #[test]
    fn test_name_and_schema() {
        let tool = NotebookEditTool::new();
        assert_eq!(tool.name(), "NotebookEdit");
        let schema = tool.input_schema();
        assert!(schema["properties"]["notebook_path"].is_object());
        assert!(schema["properties"]["edit_mode"].is_object());
    }

    #[test]
    fn test_validate_input() {
        let tool = NotebookEditTool::new();
        assert!(matches!(
            tool.validate_input(&json!({
                "notebook_path": "/tmp/test.ipynb",
                "new_source": "print('hi')"
            })),
            ValidationResult::Ok
        ));
        assert!(matches!(
            tool.validate_input(&json!({})),
            ValidationResult::Error { .. }
        ));
    }

    #[tokio::test]
    async fn test_replace_cell() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.ipynb");
        let nb = sample_notebook();
        std::fs::write(&path, serde_json::to_string_pretty(&nb).unwrap()).unwrap();

        let tool = NotebookEditTool::new();
        let result = tool
            .call(json!({
                "notebook_path": path.to_str().unwrap(),
                "cell_id": "cell-001",
                "new_source": "print('updated')",
                "edit_mode": "replace"
            }))
            .await
            .unwrap();
        assert!(!result.is_error);

        let updated: Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let source = updated["cells"][0]["source"][0].as_str().unwrap();
        assert!(source.contains("updated"));
    }

    #[tokio::test]
    async fn test_insert_cell() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.ipynb");
        let nb = sample_notebook();
        std::fs::write(&path, serde_json::to_string_pretty(&nb).unwrap()).unwrap();

        let tool = NotebookEditTool::new();
        let result = tool
            .call(json!({
                "notebook_path": path.to_str().unwrap(),
                "cell_id": "cell-001",
                "new_source": "x = 42",
                "edit_mode": "insert"
            }))
            .await
            .unwrap();
        assert!(!result.is_error);

        let updated: Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let cells = updated["cells"].as_array().unwrap();
        assert_eq!(cells.len(), 3);
    }

    #[tokio::test]
    async fn test_delete_cell() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.ipynb");
        let nb = sample_notebook();
        std::fs::write(&path, serde_json::to_string_pretty(&nb).unwrap()).unwrap();

        let tool = NotebookEditTool::new();
        let result = tool
            .call(json!({
                "notebook_path": path.to_str().unwrap(),
                "cell_id": "cell-002",
                "new_source": "",
                "edit_mode": "delete"
            }))
            .await
            .unwrap();
        assert!(!result.is_error);

        let updated: Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let cells = updated["cells"].as_array().unwrap();
        assert_eq!(cells.len(), 1);
    }

    #[tokio::test]
    async fn test_cell_not_found() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.ipynb");
        let nb = sample_notebook();
        std::fs::write(&path, serde_json::to_string_pretty(&nb).unwrap()).unwrap();

        let tool = NotebookEditTool::new();
        let result = tool
            .call(json!({
                "notebook_path": path.to_str().unwrap(),
                "cell_id": "nonexistent",
                "new_source": "x = 1",
                "edit_mode": "replace"
            }))
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(result.content.as_str().unwrap().contains("not found"));
    }
}
