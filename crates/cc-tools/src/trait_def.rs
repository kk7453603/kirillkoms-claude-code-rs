use async_trait::async_trait;
use serde_json::Value;

/// Result of tool execution
#[derive(Debug, Clone)]
pub struct ToolResult {
    pub content: Value,
    pub is_error: bool,
}

impl ToolResult {
    pub fn success(content: Value) -> Self {
        Self {
            content,
            is_error: false,
        }
    }

    pub fn error(message: &str) -> Self {
        Self {
            content: Value::String(message.to_string()),
            is_error: true,
        }
    }

    pub fn text(text: &str) -> Self {
        Self::success(Value::String(text.to_string()))
    }
}

/// Tool validation result
#[derive(Debug, Clone)]
pub enum ValidationResult {
    Ok,
    Error { message: String },
}

/// Permission decision for a tool
#[derive(Debug, Clone)]
pub enum PermissionCheckResult {
    Allow { message: Option<String> },
    Deny { message: String },
    Ask { message: String },
    Passthrough,
}

/// Interrupt behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptBehavior {
    Cancel,
    Block,
}

/// Info about whether a tool call is read/search
#[derive(Debug, Clone, Default)]
pub struct SearchReadInfo {
    pub is_search: bool,
    pub is_read: bool,
    pub is_list: bool,
}

/// Rendered content for display (replaces React nodes)
#[derive(Debug, Clone)]
pub enum RenderedContent {
    Text(String),
    Styled {
        text: String,
        bold: bool,
        dim: bool,
        color: Option<String>,
    },
    Diff {
        old: String,
        new: String,
        file_path: Option<String>,
    },
    Lines(Vec<RenderedContent>),
    Empty,
}

/// Progress sender trait
pub trait ToolProgressSender: Send + Sync {
    fn send_progress(&self, data: Value);
}

/// The core Tool trait
#[async_trait]
pub trait Tool: Send + Sync + 'static {
    /// Tool name (e.g., "Bash", "Read", "Edit")
    fn name(&self) -> &str;

    /// Alternative names
    fn aliases(&self) -> Vec<&str> {
        vec![]
    }

    /// JSON Schema for tool input
    fn input_schema(&self) -> Value;

    /// Execute the tool
    async fn call(&self, input: Value) -> Result<ToolResult, ToolError>;

    /// Tool description for API
    fn description(&self) -> String;

    /// Whether tool is read-only for given input
    fn is_read_only(&self, input: &Value) -> bool;

    /// Whether tool is safe to run concurrently
    fn is_concurrency_safe(&self, input: &Value) -> bool;

    /// Whether tool is destructive
    fn is_destructive(&self, _input: &Value) -> bool {
        false
    }

    /// Whether tool should be deferred (loaded on demand)
    fn should_defer(&self) -> bool {
        false
    }

    /// Whether tool is enabled
    fn is_enabled(&self) -> bool {
        true
    }

    /// Max result size in chars
    fn max_result_size_chars(&self) -> usize {
        30_000
    }

    /// Interrupt behavior
    fn interrupt_behavior(&self) -> InterruptBehavior {
        InterruptBehavior::Block
    }

    /// User-facing name
    fn user_facing_name(&self) -> String {
        self.name().to_string()
    }

    /// Validate input before execution
    fn validate_input(&self, _input: &Value) -> ValidationResult {
        ValidationResult::Ok
    }

    /// Get search/read info for this tool call
    fn search_read_info(&self, _input: &Value) -> SearchReadInfo {
        SearchReadInfo::default()
    }

    /// Render tool use for display
    fn render_tool_use(&self, input: &Value) -> RenderedContent {
        RenderedContent::Text(format!("{}: {}", self.name(), input))
    }

    /// Render tool result for display
    fn render_tool_result(&self, content: &Value) -> RenderedContent {
        RenderedContent::Text(content.to_string())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("Execution failed: {message}")]
    ExecutionFailed { message: String },
    #[error("Validation failed: {message}")]
    ValidationFailed { message: String },
    #[error("Permission denied: {message}")]
    PermissionDenied { message: String },
    #[error("Timeout after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },
    #[error("Cancelled")]
    Cancelled,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_result_success() {
        let result = ToolResult::success(Value::Bool(true));
        assert!(!result.is_error);
        assert_eq!(result.content, Value::Bool(true));
    }

    #[test]
    fn test_tool_result_error() {
        let result = ToolResult::error("something went wrong");
        assert!(result.is_error);
        assert_eq!(
            result.content,
            Value::String("something went wrong".to_string())
        );
    }

    #[test]
    fn test_tool_result_text() {
        let result = ToolResult::text("hello");
        assert!(!result.is_error);
        assert_eq!(result.content, Value::String("hello".to_string()));
    }

    #[test]
    fn test_validation_result_variants() {
        let ok = ValidationResult::Ok;
        assert!(matches!(ok, ValidationResult::Ok));

        let err = ValidationResult::Error {
            message: "bad".to_string(),
        };
        assert!(matches!(err, ValidationResult::Error { .. }));
    }

    #[test]
    fn test_permission_check_result_variants() {
        let allow = PermissionCheckResult::Allow { message: None };
        assert!(matches!(allow, PermissionCheckResult::Allow { .. }));

        let deny = PermissionCheckResult::Deny {
            message: "no".to_string(),
        };
        assert!(matches!(deny, PermissionCheckResult::Deny { .. }));

        let ask = PermissionCheckResult::Ask {
            message: "?".to_string(),
        };
        assert!(matches!(ask, PermissionCheckResult::Ask { .. }));

        let pass = PermissionCheckResult::Passthrough;
        assert!(matches!(pass, PermissionCheckResult::Passthrough));
    }

    #[test]
    fn test_interrupt_behavior_eq() {
        assert_eq!(InterruptBehavior::Cancel, InterruptBehavior::Cancel);
        assert_eq!(InterruptBehavior::Block, InterruptBehavior::Block);
        assert_ne!(InterruptBehavior::Cancel, InterruptBehavior::Block);
    }

    #[test]
    fn test_search_read_info_default() {
        let info = SearchReadInfo::default();
        assert!(!info.is_search);
        assert!(!info.is_read);
        assert!(!info.is_list);
    }

    #[test]
    fn test_rendered_content_variants() {
        let text = RenderedContent::Text("hello".to_string());
        assert!(matches!(text, RenderedContent::Text(_)));

        let styled = RenderedContent::Styled {
            text: "bold".to_string(),
            bold: true,
            dim: false,
            color: Some("red".to_string()),
        };
        assert!(matches!(styled, RenderedContent::Styled { .. }));

        let diff = RenderedContent::Diff {
            old: "a".to_string(),
            new: "b".to_string(),
            file_path: None,
        };
        assert!(matches!(diff, RenderedContent::Diff { .. }));

        let lines = RenderedContent::Lines(vec![]);
        assert!(matches!(lines, RenderedContent::Lines(_)));

        let empty = RenderedContent::Empty;
        assert!(matches!(empty, RenderedContent::Empty));
    }

    #[test]
    fn test_tool_error_display() {
        let err = ToolError::ExecutionFailed {
            message: "boom".to_string(),
        };
        assert_eq!(err.to_string(), "Execution failed: boom");

        let err = ToolError::ValidationFailed {
            message: "bad input".to_string(),
        };
        assert_eq!(err.to_string(), "Validation failed: bad input");

        let err = ToolError::Timeout { timeout_ms: 5000 };
        assert_eq!(err.to_string(), "Timeout after 5000ms");

        let err = ToolError::Cancelled;
        assert_eq!(err.to_string(), "Cancelled");
    }
}
