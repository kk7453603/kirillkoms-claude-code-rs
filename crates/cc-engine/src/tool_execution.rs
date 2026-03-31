use cc_api::types::ContentBlock;
use cc_tools::trait_def::{Tool, ToolError, ToolResult};
use serde_json::Value;
use std::sync::Arc;

use crate::orchestration::ToolCallResult;

/// Execute a single tool call.
pub async fn execute_single_tool(
    tool: Arc<dyn Tool>,
    input: Value,
    tool_use_id: &str,
) -> ToolCallResult {
    let start = std::time::Instant::now();
    let tool_name = tool.name().to_string();

    let result = tool.call(input).await;
    let duration_ms = start.elapsed().as_millis() as u64;

    ToolCallResult {
        tool_use_id: tool_use_id.to_string(),
        tool_name,
        result,
        duration_ms,
    }
}

/// Convert tool results to API content blocks.
pub fn tool_result_to_content_block(
    tool_use_id: &str,
    result: &Result<ToolResult, ToolError>,
) -> ContentBlock {
    match result {
        Ok(tr) => ContentBlock::ToolResult {
            tool_use_id: tool_use_id.to_string(),
            content: tr.content.clone(),
            is_error: if tr.is_error { Some(true) } else { Some(false) },
        },
        Err(e) => ContentBlock::ToolResult {
            tool_use_id: tool_use_id.to_string(),
            content: Value::String(e.to_string()),
            is_error: Some(true),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct MockTool {
        name: String,
        read_only: bool,
    }

    #[async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &str {
            &self.name
        }

        fn input_schema(&self) -> Value {
            serde_json::json!({
                "type": "object",
                "properties": {}
            })
        }

        async fn call(&self, _input: Value) -> Result<ToolResult, ToolError> {
            Ok(ToolResult::text("mock result"))
        }

        fn description(&self) -> String {
            "A mock tool".to_string()
        }

        fn is_read_only(&self, _input: &Value) -> bool {
            self.read_only
        }

        fn is_concurrency_safe(&self, _input: &Value) -> bool {
            self.read_only
        }
    }

    struct FailingTool;

    #[async_trait]
    impl Tool for FailingTool {
        fn name(&self) -> &str {
            "failing"
        }

        fn input_schema(&self) -> Value {
            serde_json::json!({"type": "object"})
        }

        async fn call(&self, _input: Value) -> Result<ToolResult, ToolError> {
            Err(ToolError::ExecutionFailed {
                message: "something broke".to_string(),
            })
        }

        fn description(&self) -> String {
            "A failing tool".to_string()
        }

        fn is_read_only(&self, _input: &Value) -> bool {
            true
        }

        fn is_concurrency_safe(&self, _input: &Value) -> bool {
            true
        }
    }

    #[tokio::test]
    async fn test_execute_single_tool_success() {
        let tool = Arc::new(MockTool {
            name: "test_tool".to_string(),
            read_only: true,
        });
        let result =
            execute_single_tool(tool, serde_json::json!({}), "tu_1").await;
        assert_eq!(result.tool_use_id, "tu_1");
        assert_eq!(result.tool_name, "test_tool");
        assert!(result.result.is_ok());
        let tr = result.result.unwrap();
        assert!(!tr.is_error);
        assert_eq!(tr.content, Value::String("mock result".to_string()));
    }

    #[tokio::test]
    async fn test_execute_single_tool_failure() {
        let tool: Arc<dyn Tool> = Arc::new(FailingTool);
        let result =
            execute_single_tool(tool, serde_json::json!({}), "tu_2").await;
        assert_eq!(result.tool_use_id, "tu_2");
        assert!(result.result.is_err());
    }

    #[tokio::test]
    async fn test_execute_single_tool_records_duration() {
        let tool = Arc::new(MockTool {
            name: "fast".to_string(),
            read_only: true,
        });
        let result =
            execute_single_tool(tool, serde_json::json!({}), "tu_3").await;
        // Duration should be non-negative (might be 0 for very fast calls)
        assert!(result.duration_ms < 10_000);
    }

    #[test]
    fn test_tool_result_to_content_block_success() {
        let tr = Ok(ToolResult::text("hello"));
        let block = tool_result_to_content_block("tu_1", &tr);
        match block {
            ContentBlock::ToolResult {
                tool_use_id,
                content,
                is_error,
            } => {
                assert_eq!(tool_use_id, "tu_1");
                assert_eq!(content, Value::String("hello".to_string()));
                assert_eq!(is_error, Some(false));
            }
            _ => panic!("expected ToolResult"),
        }
    }

    #[test]
    fn test_tool_result_to_content_block_tool_error_flag() {
        let tr = Ok(ToolResult::error("bad input"));
        let block = tool_result_to_content_block("tu_2", &tr);
        match block {
            ContentBlock::ToolResult { is_error, .. } => {
                assert_eq!(is_error, Some(true));
            }
            _ => panic!("expected ToolResult"),
        }
    }

    #[test]
    fn test_tool_result_to_content_block_error() {
        let tr: Result<ToolResult, ToolError> = Err(ToolError::ExecutionFailed {
            message: "boom".to_string(),
        });
        let block = tool_result_to_content_block("tu_3", &tr);
        match block {
            ContentBlock::ToolResult {
                tool_use_id,
                content,
                is_error,
            } => {
                assert_eq!(tool_use_id, "tu_3");
                assert!(content.as_str().unwrap().contains("boom"));
                assert_eq!(is_error, Some(true));
            }
            _ => panic!("expected ToolResult"),
        }
    }
}
