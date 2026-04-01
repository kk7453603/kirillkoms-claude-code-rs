use cc_tools::registry::ToolRegistry;
use cc_tools::trait_def::{ToolError, ToolResult};

use crate::tool_execution::execute_single_tool;

/// A pending tool call from the model.
#[derive(Debug, Clone)]
pub struct PendingToolCall {
    pub id: String,
    pub name: String,
    pub input: serde_json::Value,
}

/// Result of executing tool calls.
#[derive(Debug)]
pub struct ToolCallResult {
    pub tool_use_id: String,
    pub tool_name: String,
    pub result: Result<ToolResult, ToolError>,
    pub duration_ms: u64,
}

/// Partition tool calls into concurrent (read-only) and serial (write) batches.
///
/// Returns (concurrent_batch, serial_batch).
/// All read-only + concurrency-safe calls go to concurrent.
/// Everything else goes to serial.
pub fn partition_tool_calls(
    calls: &[PendingToolCall],
    tools: &ToolRegistry,
) -> (Vec<PendingToolCall>, Vec<PendingToolCall>) {
    let mut concurrent = Vec::new();
    let mut serial = Vec::new();

    for call in calls {
        let is_concurrent = tools
            .get(&call.name)
            .map(|t| t.is_read_only(&call.input) && t.is_concurrency_safe(&call.input))
            .unwrap_or(false);

        if is_concurrent {
            concurrent.push(call.clone());
        } else {
            serial.push(call.clone());
        }
    }

    (concurrent, serial)
}

/// Execute a batch of tool calls concurrently.
pub async fn run_tools_concurrently(
    calls: Vec<PendingToolCall>,
    tools: &ToolRegistry,
) -> Vec<ToolCallResult> {
    let futures: Vec<_> = calls
        .into_iter()
        .map(|call| {
            let tool = tools.get(&call.name);
            async move {
                match tool {
                    Some(t) => execute_single_tool(t, call.input, &call.id).await,
                    None => ToolCallResult {
                        tool_use_id: call.id,
                        tool_name: call.name,
                        result: Err(ToolError::ExecutionFailed {
                            message: "Tool not found".to_string(),
                        }),
                        duration_ms: 0,
                    },
                }
            }
        })
        .collect();

    futures::future::join_all(futures).await
}

/// Execute tool calls serially.
pub async fn run_tools_serially(
    calls: Vec<PendingToolCall>,
    tools: &ToolRegistry,
) -> Vec<ToolCallResult> {
    let mut results = Vec::with_capacity(calls.len());

    for call in calls {
        let result = match tools.get(&call.name) {
            Some(t) => execute_single_tool(t, call.input, &call.id).await,
            None => ToolCallResult {
                tool_use_id: call.id,
                tool_name: call.name,
                result: Err(ToolError::ExecutionFailed {
                    message: "Tool not found".to_string(),
                }),
                duration_ms: 0,
            },
        };
        results.push(result);
    }

    results
}

/// Execute all tool calls with proper partitioning.
///
/// Concurrent (read-only) tools run first in parallel,
/// then serial (write) tools run one by one.
pub async fn execute_tool_calls(
    calls: Vec<PendingToolCall>,
    tools: &ToolRegistry,
) -> Vec<ToolCallResult> {
    let (concurrent, serial) = partition_tool_calls(&calls, tools);

    let mut results = Vec::with_capacity(calls.len());

    if !concurrent.is_empty() {
        let concurrent_results = run_tools_concurrently(concurrent, tools).await;
        results.extend(concurrent_results);
    }

    if !serial.is_empty() {
        let serial_results = run_tools_serially(serial, tools).await;
        results.extend(serial_results);
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use cc_tools::trait_def::Tool;
    use serde_json::Value;
    use std::sync::Arc;

    struct ReadOnlyTool {
        name: String,
    }

    #[async_trait]
    impl Tool for ReadOnlyTool {
        fn name(&self) -> &str {
            &self.name
        }
        fn input_schema(&self) -> Value {
            serde_json::json!({"type": "object"})
        }
        async fn call(&self, _input: Value) -> Result<ToolResult, ToolError> {
            Ok(ToolResult::text(&format!("{} result", self.name)))
        }
        fn description(&self) -> String {
            format!("{} description", self.name)
        }
        fn is_read_only(&self, _input: &Value) -> bool {
            true
        }
        fn is_concurrency_safe(&self, _input: &Value) -> bool {
            true
        }
    }

    struct WriteTool {
        name: String,
    }

    #[async_trait]
    impl Tool for WriteTool {
        fn name(&self) -> &str {
            &self.name
        }
        fn input_schema(&self) -> Value {
            serde_json::json!({"type": "object"})
        }
        async fn call(&self, _input: Value) -> Result<ToolResult, ToolError> {
            Ok(ToolResult::text(&format!("{} result", self.name)))
        }
        fn description(&self) -> String {
            format!("{} description", self.name)
        }
        fn is_read_only(&self, _input: &Value) -> bool {
            false
        }
        fn is_concurrency_safe(&self, _input: &Value) -> bool {
            false
        }
    }

    fn make_registry() -> ToolRegistry {
        let mut reg = ToolRegistry::new();
        reg.register(Arc::new(ReadOnlyTool {
            name: "Read".to_string(),
        }));
        reg.register(Arc::new(ReadOnlyTool {
            name: "Grep".to_string(),
        }));
        reg.register(Arc::new(WriteTool {
            name: "Edit".to_string(),
        }));
        reg.register(Arc::new(WriteTool {
            name: "Bash".to_string(),
        }));
        reg
    }

    #[test]
    fn test_partition_all_read_only() {
        let reg = make_registry();
        let calls = vec![
            PendingToolCall {
                id: "1".into(),
                name: "Read".into(),
                input: serde_json::json!({}),
            },
            PendingToolCall {
                id: "2".into(),
                name: "Grep".into(),
                input: serde_json::json!({}),
            },
        ];
        let (concurrent, serial) = partition_tool_calls(&calls, &reg);
        assert_eq!(concurrent.len(), 2);
        assert_eq!(serial.len(), 0);
    }

    #[test]
    fn test_partition_all_write() {
        let reg = make_registry();
        let calls = vec![
            PendingToolCall {
                id: "1".into(),
                name: "Edit".into(),
                input: serde_json::json!({}),
            },
            PendingToolCall {
                id: "2".into(),
                name: "Bash".into(),
                input: serde_json::json!({}),
            },
        ];
        let (concurrent, serial) = partition_tool_calls(&calls, &reg);
        assert_eq!(concurrent.len(), 0);
        assert_eq!(serial.len(), 2);
    }

    #[test]
    fn test_partition_mixed() {
        let reg = make_registry();
        let calls = vec![
            PendingToolCall {
                id: "1".into(),
                name: "Read".into(),
                input: serde_json::json!({}),
            },
            PendingToolCall {
                id: "2".into(),
                name: "Edit".into(),
                input: serde_json::json!({}),
            },
            PendingToolCall {
                id: "3".into(),
                name: "Grep".into(),
                input: serde_json::json!({}),
            },
        ];
        let (concurrent, serial) = partition_tool_calls(&calls, &reg);
        assert_eq!(concurrent.len(), 2);
        assert_eq!(serial.len(), 1);
        assert_eq!(serial[0].name, "Edit");
    }

    #[test]
    fn test_partition_unknown_tool_goes_serial() {
        let reg = make_registry();
        let calls = vec![PendingToolCall {
            id: "1".into(),
            name: "Unknown".into(),
            input: serde_json::json!({}),
        }];
        let (concurrent, serial) = partition_tool_calls(&calls, &reg);
        assert_eq!(concurrent.len(), 0);
        assert_eq!(serial.len(), 1);
    }

    #[tokio::test]
    async fn test_run_tools_concurrently() {
        let reg = make_registry();
        let calls = vec![
            PendingToolCall {
                id: "1".into(),
                name: "Read".into(),
                input: serde_json::json!({}),
            },
            PendingToolCall {
                id: "2".into(),
                name: "Grep".into(),
                input: serde_json::json!({}),
            },
        ];
        let results = run_tools_concurrently(calls, &reg).await;
        assert_eq!(results.len(), 2);
        for r in &results {
            assert!(r.result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_run_tools_serially() {
        let reg = make_registry();
        let calls = vec![
            PendingToolCall {
                id: "1".into(),
                name: "Edit".into(),
                input: serde_json::json!({}),
            },
            PendingToolCall {
                id: "2".into(),
                name: "Bash".into(),
                input: serde_json::json!({}),
            },
        ];
        let results = run_tools_serially(calls, &reg).await;
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].tool_use_id, "1");
        assert_eq!(results[1].tool_use_id, "2");
    }

    #[tokio::test]
    async fn test_run_tools_unknown_tool() {
        let reg = make_registry();
        let calls = vec![PendingToolCall {
            id: "1".into(),
            name: "Nonexistent".into(),
            input: serde_json::json!({}),
        }];
        let results = run_tools_serially(calls, &reg).await;
        assert_eq!(results.len(), 1);
        assert!(results[0].result.is_err());
    }

    #[tokio::test]
    async fn test_execute_tool_calls_mixed() {
        let reg = make_registry();
        let calls = vec![
            PendingToolCall {
                id: "1".into(),
                name: "Read".into(),
                input: serde_json::json!({}),
            },
            PendingToolCall {
                id: "2".into(),
                name: "Edit".into(),
                input: serde_json::json!({}),
            },
        ];
        let results = execute_tool_calls(calls, &reg).await;
        assert_eq!(results.len(), 2);
        // Both should succeed
        for r in &results {
            assert!(r.result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_execute_tool_calls_empty() {
        let reg = make_registry();
        let results = execute_tool_calls(vec![], &reg).await;
        assert!(results.is_empty());
    }
}
