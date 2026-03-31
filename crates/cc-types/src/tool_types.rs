use serde::{Deserialize, Serialize};

/// Schema definition for a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// Result of a tool execution.
#[derive(Debug, Clone)]
pub struct ToolResult {
    pub data: serde_json::Value,
    pub is_error: bool,
}

/// Result of validating tool input.
#[derive(Debug, Clone)]
pub enum ValidationResult {
    Ok,
    Error { message: String },
}

/// How tool execution handles interrupts.
#[derive(Debug, Clone, Copy)]
pub enum InterruptBehavior {
    Cancel,
    Block,
}

/// Info about the nature of a tool invocation (search/read/list).
#[derive(Debug, Clone, Default)]
pub struct SearchReadInfo {
    pub is_search: bool,
    pub is_read: bool,
    pub is_list: bool,
}

/// Rendered content for TUI display (replaces React nodes from TS).
#[derive(Debug, Clone)]
pub enum RenderedContent {
    Text(String),
    Styled(Vec<StyledSpan>),
    Diff {
        old: String,
        new: String,
        file_path: Option<String>,
    },
    Lines(Vec<RenderedContent>),
    Empty,
}

/// A styled span of text for terminal output.
#[derive(Debug, Clone)]
pub struct StyledSpan {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub dim: bool,
    pub color: Option<String>,
}

impl StyledSpan {
    /// Create a plain (unstyled) span.
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            bold: false,
            italic: false,
            dim: false,
            color: None,
        }
    }

    /// Create a bold span.
    pub fn bold(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            bold: true,
            italic: false,
            dim: false,
            color: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_schema_serde_roundtrip() {
        let schema = ToolSchema {
            name: "bash".to_string(),
            description: "Run a shell command".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {"type": "string"}
                },
                "required": ["command"]
            }),
        };
        let json = serde_json::to_string(&schema).unwrap();
        let back: ToolSchema = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "bash");
        assert_eq!(back.description, "Run a shell command");
        assert!(back.input_schema["properties"]["command"]["type"]
            .as_str()
            .unwrap()
            == "string");
    }

    #[test]
    fn tool_result_construction() {
        let result = ToolResult {
            data: serde_json::json!({"output": "hello world"}),
            is_error: false,
        };
        assert!(!result.is_error);
        assert_eq!(result.data["output"], "hello world");

        let error_result = ToolResult {
            data: serde_json::json!({"error": "command not found"}),
            is_error: true,
        };
        assert!(error_result.is_error);
    }

    #[test]
    fn validation_result_variants() {
        let ok = ValidationResult::Ok;
        assert!(matches!(ok, ValidationResult::Ok));

        let err = ValidationResult::Error {
            message: "missing required field".to_string(),
        };
        match err {
            ValidationResult::Error { message } => {
                assert_eq!(message, "missing required field");
            }
            _ => panic!("expected Error"),
        }
    }

    #[test]
    fn interrupt_behavior_copy() {
        let b = InterruptBehavior::Cancel;
        let b2 = b; // Copy
        assert!(matches!(b2, InterruptBehavior::Cancel));

        let b3 = InterruptBehavior::Block;
        assert!(matches!(b3, InterruptBehavior::Block));
    }

    #[test]
    fn search_read_info_default() {
        let info = SearchReadInfo::default();
        assert!(!info.is_search);
        assert!(!info.is_read);
        assert!(!info.is_list);
    }

    #[test]
    fn search_read_info_flags() {
        let info = SearchReadInfo {
            is_search: true,
            is_read: false,
            is_list: true,
        };
        assert!(info.is_search);
        assert!(!info.is_read);
        assert!(info.is_list);
    }

    #[test]
    fn rendered_content_text() {
        let content = RenderedContent::Text("Hello".to_string());
        match content {
            RenderedContent::Text(t) => assert_eq!(t, "Hello"),
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn rendered_content_styled() {
        let content = RenderedContent::Styled(vec![
            StyledSpan::bold("Title"),
            StyledSpan::plain(" - description"),
        ]);
        match content {
            RenderedContent::Styled(spans) => {
                assert_eq!(spans.len(), 2);
                assert!(spans[0].bold);
                assert!(!spans[1].bold);
            }
            _ => panic!("expected Styled"),
        }
    }

    #[test]
    fn rendered_content_diff() {
        let content = RenderedContent::Diff {
            old: "line1\nline2".to_string(),
            new: "line1\nline2_modified".to_string(),
            file_path: Some("/tmp/test.rs".to_string()),
        };
        match content {
            RenderedContent::Diff {
                old,
                new,
                file_path,
            } => {
                assert!(old.contains("line2"));
                assert!(new.contains("line2_modified"));
                assert_eq!(file_path, Some("/tmp/test.rs".to_string()));
            }
            _ => panic!("expected Diff"),
        }
    }

    #[test]
    fn rendered_content_lines() {
        let content = RenderedContent::Lines(vec![
            RenderedContent::Text("line 1".to_string()),
            RenderedContent::Text("line 2".to_string()),
            RenderedContent::Empty,
        ]);
        match content {
            RenderedContent::Lines(lines) => assert_eq!(lines.len(), 3),
            _ => panic!("expected Lines"),
        }
    }

    #[test]
    fn rendered_content_empty() {
        let content = RenderedContent::Empty;
        assert!(matches!(content, RenderedContent::Empty));
    }

    #[test]
    fn styled_span_plain() {
        let span = StyledSpan::plain("hello");
        assert_eq!(span.text, "hello");
        assert!(!span.bold);
        assert!(!span.italic);
        assert!(!span.dim);
        assert!(span.color.is_none());
    }

    #[test]
    fn styled_span_with_color() {
        let span = StyledSpan {
            text: "warning".to_string(),
            bold: true,
            italic: false,
            dim: false,
            color: Some("yellow".to_string()),
        };
        assert_eq!(span.color, Some("yellow".to_string()));
        assert!(span.bold);
    }
}
