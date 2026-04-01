use async_stream::stream;
use std::sync::Arc;
use tokio_stream::Stream;

use cc_api::types::{ApiMessage, ContentBlock, MessagesRequest, Role, SystemBlock, ThinkingConfig, ToolDefinition};
use cc_tools::registry::ToolRegistry;

use crate::orchestration::{execute_tool_calls, execute_tool_calls_with_context, PendingToolCall};
use crate::tool_execution::{ExecutionContext, PermissionCallback};

/// Events yielded by the query loop.
#[derive(Debug, Clone)]
pub enum QueryEvent {
    /// Text content from the model
    TextDelta(String),
    /// Thinking content
    ThinkingDelta(String),
    /// Tool use started
    ToolUseStart { id: String, name: String },
    /// Tool result
    ToolResult {
        id: String,
        result: serde_json::Value,
        is_error: bool,
    },
    /// Turn completed
    TurnComplete { stop_reason: String },
    /// Error occurred
    Error(String),
    /// Usage update
    UsageUpdate {
        input_tokens: u64,
        output_tokens: u64,
    },
}

/// Parameters for a query.
pub struct QueryParams {
    pub messages: Vec<ApiMessage>,
    pub system: Vec<SystemBlock>,
    pub model: String,
    pub max_tokens: u32,
    pub tools: Arc<ToolRegistry>,
    pub api_client: Arc<dyn cc_api::client::ApiClient>,
    pub max_turns: usize,
    pub thinking: Option<ThinkingConfig>,
    pub execution_context: Option<ExecutionContext>,
    pub permission_callback: Option<Arc<dyn PermissionCallback>>,
}

/// Run the main query loop.
///
/// This sends messages to the API, processes tool calls, and yields events.
/// The loop continues until the model stops requesting tools or max_turns is reached.
pub fn query(params: QueryParams) -> impl Stream<Item = QueryEvent> + Send {
    stream! {
        let mut messages = params.messages.clone();
        let mut turns = 0;

        loop {
            if turns >= params.max_turns {
                yield QueryEvent::TurnComplete {
                    stop_reason: "max_turns".into(),
                };
                break;
            }
            turns += 1;

            // 1. Build API request
            let request = MessagesRequest {
                model: params.model.clone(),
                messages: messages.clone(),
                system: params.system.clone(),
                max_tokens: Some(params.max_tokens),
                temperature: None,
                tools: Some(build_tool_definitions(&params.tools)),
                tool_choice: None,
                thinking: params.thinking.clone(),
                stream: false,
                metadata: None,
            };

            // 2. Send request
            let response = match params.api_client.send_messages(request).await {
                Ok(r) => r,
                Err(e) => {
                    yield QueryEvent::Error(e.to_string());
                    break;
                }
            };

            // 3. Yield text content and collect tool calls
            let mut tool_calls = Vec::new();
            for block in &response.content {
                match block {
                    ContentBlock::Text { text } => {
                        yield QueryEvent::TextDelta(text.clone());
                    }
                    ContentBlock::Thinking { thinking, .. } => {
                        yield QueryEvent::ThinkingDelta(thinking.clone());
                    }
                    ContentBlock::ToolUse { id, name, input } => {
                        yield QueryEvent::ToolUseStart {
                            id: id.clone(),
                            name: name.clone(),
                        };
                        tool_calls.push(PendingToolCall {
                            id: id.clone(),
                            name: name.clone(),
                            input: input.clone(),
                        });
                    }
                    _ => {}
                }
            }

            // 4. Yield usage
            yield QueryEvent::UsageUpdate {
                input_tokens: response.usage.input_tokens,
                output_tokens: response.usage.output_tokens,
            };

            // 5. If no tool calls, we're done
            if tool_calls.is_empty() {
                yield QueryEvent::TurnComplete {
                    stop_reason: response.stop_reason.unwrap_or_else(|| "end_turn".into()),
                };
                break;
            }

            // 6. Execute tool calls
            let results = if let (Some(exec_ctx), Some(perm_cb)) =
                (&params.execution_context, &params.permission_callback)
            {
                execute_tool_calls_with_context(
                    tool_calls,
                    &params.tools,
                    exec_ctx,
                    perm_cb.as_ref(),
                )
                .await
            } else {
                execute_tool_calls(tool_calls, &params.tools).await
            };

            // 7. Build assistant message
            let assistant_msg = ApiMessage {
                role: Role::Assistant,
                content: response.content.clone(),
            };
            messages.push(assistant_msg);

            // 8. Yield tool results and build tool result content blocks
            let mut tool_result_blocks: Vec<ContentBlock> = Vec::new();
            for r in &results {
                let (content, is_error) = match &r.result {
                    Ok(tr) => (tr.content.clone(), tr.is_error),
                    Err(e) => (serde_json::Value::String(e.to_string()), true),
                };
                yield QueryEvent::ToolResult {
                    id: r.tool_use_id.clone(),
                    result: content.clone(),
                    is_error,
                };
                tool_result_blocks.push(ContentBlock::ToolResult {
                    tool_use_id: r.tool_use_id.clone(),
                    content,
                    is_error: Some(is_error),
                });
            }

            messages.push(ApiMessage {
                role: Role::User,
                content: tool_result_blocks,
            });
        }
    }
}

/// Build tool definitions for API request.
pub fn build_tool_definitions(registry: &ToolRegistry) -> Vec<ToolDefinition> {
    registry
        .enabled_tools()
        .iter()
        .map(|t| ToolDefinition {
            name: t.name().to_string(),
            description: t.description(),
            input_schema: t.input_schema(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_event_variants() {
        // Just verify all variants can be constructed
        let _e1 = QueryEvent::TextDelta("hello".to_string());
        let _e2 = QueryEvent::ThinkingDelta("thinking".to_string());
        let _e3 = QueryEvent::ToolUseStart {
            id: "tu_1".to_string(),
            name: "Bash".to_string(),
        };
        let _e4 = QueryEvent::ToolResult {
            id: "tu_1".to_string(),
            result: serde_json::json!("done"),
            is_error: false,
        };
        let _e5 = QueryEvent::TurnComplete {
            stop_reason: "end_turn".to_string(),
        };
        let _e6 = QueryEvent::Error("something went wrong".to_string());
        let _e7 = QueryEvent::UsageUpdate {
            input_tokens: 100,
            output_tokens: 50,
        };
    }

    #[test]
    fn test_query_event_debug() {
        let event = QueryEvent::TextDelta("hi".to_string());
        let debug = format!("{:?}", event);
        assert!(debug.contains("TextDelta"));
    }

    #[test]
    fn test_query_event_clone() {
        let event = QueryEvent::ToolUseStart {
            id: "tu_1".to_string(),
            name: "Read".to_string(),
        };
        let cloned = event.clone();
        match cloned {
            QueryEvent::ToolUseStart { id, name } => {
                assert_eq!(id, "tu_1");
                assert_eq!(name, "Read");
            }
            _ => panic!("expected ToolUseStart"),
        }
    }

    #[test]
    fn test_build_tool_definitions_empty_registry() {
        let reg = ToolRegistry::new();
        let defs = build_tool_definitions(&reg);
        assert!(defs.is_empty());
    }

    #[test]
    fn test_build_tool_definitions_with_tools() {
        use async_trait::async_trait;
        use cc_tools::trait_def::{Tool, ToolError, ToolResult as TResult};
        use serde_json::Value;

        struct DummyTool;

        #[async_trait]
        impl Tool for DummyTool {
            fn name(&self) -> &str {
                "dummy"
            }
            fn input_schema(&self) -> Value {
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "arg": {"type": "string"}
                    }
                })
            }
            async fn call(&self, _input: Value) -> Result<TResult, ToolError> {
                Ok(TResult::text("ok"))
            }
            fn description(&self) -> String {
                "A dummy tool".to_string()
            }
            fn is_read_only(&self, _input: &Value) -> bool {
                true
            }
            fn is_concurrency_safe(&self, _input: &Value) -> bool {
                true
            }
        }

        let mut reg = ToolRegistry::new();
        reg.register(Arc::new(DummyTool));
        let defs = build_tool_definitions(&reg);
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].name, "dummy");
        assert_eq!(defs[0].description, "A dummy tool");
        assert!(defs[0].input_schema.is_object());
    }

    #[tokio::test]
    async fn test_query_max_turns_zero() {
        use tokio_stream::StreamExt;

        let reg = Arc::new(ToolRegistry::new());
        let api_client = Arc::new(MockApiClient::new(vec![]));

        let params = QueryParams {
            messages: vec![],
            system: vec![],
            model: "test".to_string(),
            max_tokens: 1024,
            tools: reg,
            api_client,
            max_turns: 0,
            thinking: None,
            execution_context: None,
            permission_callback: None,
        };

        let mut stream = std::pin::pin!(query(params));
        let event = stream.next().await.unwrap();
        match event {
            QueryEvent::TurnComplete { stop_reason } => {
                assert_eq!(stop_reason, "max_turns");
            }
            _ => panic!("expected TurnComplete, got {:?}", event),
        }
    }

    #[tokio::test]
    async fn test_query_simple_text_response() {
        use tokio_stream::StreamExt;

        let response = cc_api::types::MessagesResponse {
            id: "msg_1".to_string(),
            model: "test".to_string(),
            role: cc_api::types::Role::Assistant,
            content: vec![ContentBlock::Text {
                text: "Hello!".to_string(),
            }],
            stop_reason: Some("end_turn".to_string()),
            usage: cc_api::types::Usage {
                input_tokens: 10,
                output_tokens: 5,
                ..Default::default()
            },
        };

        let api_client = Arc::new(MockApiClient::new(vec![Ok(response)]));
        let reg = Arc::new(ToolRegistry::new());

        let params = QueryParams {
            messages: vec![ApiMessage {
                role: Role::User,
                content: vec![ContentBlock::Text {
                    text: "Hi".to_string(),
                }],
            }],
            system: vec![],
            model: "test".to_string(),
            max_tokens: 1024,
            tools: reg,
            api_client,
            max_turns: 5,
            thinking: None,
            execution_context: None,
            permission_callback: None,
        };

        let mut stream = std::pin::pin!(query(params));
        let mut events = Vec::new();
        while let Some(event) = stream.next().await {
            events.push(event);
        }

        assert!(events.len() >= 3); // TextDelta, UsageUpdate, TurnComplete
        assert!(matches!(&events[0], QueryEvent::TextDelta(t) if t == "Hello!"));
        assert!(matches!(
            &events[1],
            QueryEvent::UsageUpdate {
                input_tokens: 10,
                output_tokens: 5
            }
        ));
        assert!(matches!(
            &events[2],
            QueryEvent::TurnComplete { stop_reason } if stop_reason == "end_turn"
        ));
    }

    #[tokio::test]
    async fn test_query_api_error() {
        use tokio_stream::StreamExt;

        let api_client = Arc::new(MockApiClient::new(vec![Err(
            cc_api::errors::ApiError::ConnectionError {
                message: "connection refused".to_string(),
            },
        )]));
        let reg = Arc::new(ToolRegistry::new());

        let params = QueryParams {
            messages: vec![],
            system: vec![],
            model: "test".to_string(),
            max_tokens: 1024,
            tools: reg,
            api_client,
            max_turns: 5,
            thinking: None,
            execution_context: None,
            permission_callback: None,
        };

        let mut stream = std::pin::pin!(query(params));
        let event = stream.next().await.unwrap();
        assert!(matches!(event, QueryEvent::Error(msg) if msg.contains("connection refused")));
    }

    // --- Mock API client for tests ---

    struct MockApiClient {
        responses:
            std::sync::Mutex<Vec<Result<cc_api::types::MessagesResponse, cc_api::errors::ApiError>>>,
    }

    impl MockApiClient {
        fn new(
            responses: Vec<
                Result<cc_api::types::MessagesResponse, cc_api::errors::ApiError>,
            >,
        ) -> Self {
            // Reverse so we can pop from the end
            let mut responses = responses;
            responses.reverse();
            Self {
                responses: std::sync::Mutex::new(responses),
            }
        }
    }

    impl std::fmt::Debug for MockApiClient {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("MockApiClient").finish()
        }
    }

    #[async_trait::async_trait]
    impl cc_api::client::ApiClient for MockApiClient {
        async fn stream_messages(
            &self,
            _request: MessagesRequest,
            _cancel: tokio_util::sync::CancellationToken,
        ) -> Result<
            std::pin::Pin<
                Box<
                    dyn futures::Stream<
                            Item = Result<cc_api::types::StreamEvent, cc_api::errors::ApiError>,
                        > + Send,
                >,
            >,
            cc_api::errors::ApiError,
        > {
            Err(cc_api::errors::ApiError::InvalidRequest {
                message: "streaming not implemented in mock".to_string(),
            })
        }

        async fn send_messages(
            &self,
            _request: MessagesRequest,
        ) -> Result<cc_api::types::MessagesResponse, cc_api::errors::ApiError> {
            let mut responses = self.responses.lock().unwrap();
            responses.pop().unwrap_or(Err(cc_api::errors::ApiError::InvalidRequest {
                message: "no more mock responses".to_string(),
            }))
        }
    }
}
