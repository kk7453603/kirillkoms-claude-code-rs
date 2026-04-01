use async_stream::stream;
use futures::StreamExt;
use std::sync::Arc;
use tokio_stream::Stream;

use cc_api::types::{
    ApiMessage, ContentBlock, ContentDelta, MessagesRequest, Role, StreamEvent, SystemBlock,
    ThinkingConfig, ToolDefinition,
};
use cc_tools::registry::ToolRegistry;

use crate::orchestration::{PendingToolCall, execute_tool_calls, execute_tool_calls_with_context};
use crate::streaming::StreamState;
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
    /// Session-level cache: does this model support function calling?
    /// `None` = unknown, `Some(false)` = no support (use text-based fallback).
    pub tools_supported: Option<bool>,
    /// Idle timeout per turn. If no SSE chunk arrives within this duration,
    /// the stream is cancelled and a partial response is returned.
    /// `None` = no timeout (default).
    pub turn_timeout: Option<std::time::Duration>,
}

/// Run the main query loop.
///
/// This sends messages to the API, processes tool calls, and yields events.
/// The loop continues until the model stops requesting tools or max_turns is reached.
pub fn query(mut params: QueryParams) -> impl Stream<Item = QueryEvent> + Send {
    stream! {
        let mut messages = params.messages.clone();
        let mut turns = 0;

        'outer: loop {
            if turns >= params.max_turns {
                yield QueryEvent::TurnComplete {
                    stop_reason: "max_turns".into(),
                };
                break;
            }
            turns += 1;

            // 1. Build API request — omit tools if model doesn't support them
            let use_native_tools = params.tools_supported != Some(false);
            let mut system = params.system.clone();

            let tools = if use_native_tools {
                Some(build_tool_definitions(&params.tools))
            } else {
                // Inject text-based tool descriptions into system prompt
                let tool_text = build_text_tool_descriptions(&params.tools);
                system.push(SystemBlock::Text {
                    text: tool_text,
                    cache_control: None,
                });
                None
            };

            let request = MessagesRequest {
                model: params.model.clone(),
                messages: messages.clone(),
                system,
                max_tokens: Some(params.max_tokens),
                temperature: None,
                tools,
                tool_choice: None,
                thinking: params.thinking.clone(),
                stream: true,
                metadata: None,
            };

            // 2. Open streaming connection
            let cancel = tokio_util::sync::CancellationToken::new();
            let event_stream = params.api_client.stream_messages(request, cancel.clone()).await;

            // 2a. Handle "does not support tools" error with retry
            let event_stream = match event_stream {
                Err(ref e) if use_native_tools && is_tools_not_supported_error(e) => {
                    tracing::info!("Model does not support native tools, switching to text-based fallback");
                    params.tools_supported = Some(false);

                    // Rebuild request without tools
                    let mut retry_system = params.system.clone();
                    let tool_text = build_text_tool_descriptions(&params.tools);
                    retry_system.push(SystemBlock::Text {
                        text: tool_text,
                        cache_control: None,
                    });

                    let retry_request = MessagesRequest {
                        model: params.model.clone(),
                        messages: messages.clone(),
                        system: retry_system,
                        max_tokens: Some(params.max_tokens),
                        temperature: None,
                        tools: None,
                        tool_choice: None,
                        thinking: params.thinking.clone(),
                        stream: true,
                        metadata: None,
                    };

                    let retry_cancel = tokio_util::sync::CancellationToken::new();
                    params.api_client.stream_messages(retry_request, retry_cancel).await
                }
                other => other,
            };

            let state = match event_stream {
                Ok(mut stream) => {
                    let mut state = StreamState::new();
                    let mut timed_out = false;

                    loop {
                        // Apply idle timeout if configured
                        let next_event = if let Some(timeout) = params.turn_timeout {
                            match tokio::time::timeout(timeout, stream.next()).await {
                                Ok(event) => event,
                                Err(_) => {
                                    // Idle timeout — no chunk received within duration
                                    timed_out = true;
                                    None
                                }
                            }
                        } else {
                            stream.next().await
                        };

                        match next_event {
                            Some(Ok(event)) => {
                                // Yield deltas as they arrive for real-time output
                                match &event {
                                    StreamEvent::ContentBlockDelta { delta, .. } => {
                                        match delta {
                                            ContentDelta::TextDelta { text } => {
                                                yield QueryEvent::TextDelta(text.clone());
                                            }
                                            ContentDelta::ThinkingDelta { thinking } => {
                                                yield QueryEvent::ThinkingDelta(thinking.clone());
                                            }
                                            _ => {}
                                        }
                                    }
                                    StreamEvent::ContentBlockStop { index } => {
                                        // Peek at tool_use blocks to yield ToolUseStart
                                        if let Some(ContentBlock::ToolUse { id, name, .. }) =
                                            state.content_blocks.get(*index)
                                        {
                                            yield QueryEvent::ToolUseStart {
                                                id: id.clone(),
                                                name: name.clone(),
                                            };
                                        }
                                    }
                                    StreamEvent::MessageDelta { usage, .. } => {
                                        if let Some(u) = usage {
                                            yield QueryEvent::UsageUpdate {
                                                input_tokens: state.usage.input_tokens,
                                                output_tokens: u.output_tokens,
                                            };
                                        }
                                    }
                                    _ => {}
                                }
                                state.process_event(event);
                            }
                            Some(Err(e)) => {
                                yield QueryEvent::Error(e.to_string());
                                break 'outer;
                            }
                            None => break, // Stream ended (or timed out)
                        }
                    }

                    if timed_out {
                        let secs = params.turn_timeout.map(|d| d.as_secs()).unwrap_or(0);
                        yield QueryEvent::TextDelta(format!(
                            "\n\n[Response truncated: no data received for {}s]", secs
                        ));
                        state.stop_reason = Some("timeout".to_string());
                    }

                    state
                }
                Err(e) => {
                    yield QueryEvent::Error(e.to_string());
                    break;
                }
            };

            // 3. Yield final usage from accumulated state
            yield QueryEvent::UsageUpdate {
                input_tokens: state.usage.input_tokens,
                output_tokens: state.usage.output_tokens,
            };

            // 4. Extract tool calls — from native tool_use blocks or text-based fallback
            let tool_calls: Vec<PendingToolCall> = if params.tools_supported == Some(false) {
                // Parse tool calls from text content
                let text = state.content_blocks.iter()
                    .filter_map(|b| match b {
                        ContentBlock::Text { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("");
                parse_tool_calls_from_text(&text)
            } else {
                if !state.tool_uses.is_empty() {
                    params.tools_supported = Some(true);
                }
                state.tool_uses.clone()
            };

            // 5. If no tool calls, we're done
            if tool_calls.is_empty() {
                yield QueryEvent::TurnComplete {
                    stop_reason: state.stop_reason.unwrap_or_else(|| "end_turn".into()),
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

            // 7. Build assistant message from accumulated content blocks
            let assistant_msg = ApiMessage {
                role: Role::Assistant,
                content: state.content_blocks.clone(),
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

/// Build a text-based description of tools for injection into the system prompt.
/// Used as fallback when the model does not support native function calling.
fn build_text_tool_descriptions(registry: &ToolRegistry) -> String {
    let mut desc = String::from(
        "You have access to these tools. To use a tool, respond with a JSON block:\n\n\
         ```tool_call\n\
         {\"name\": \"tool_name\", \"arguments\": {\"key\": \"value\"}}\n\
         ```\n\n\
         You may use multiple tool_call blocks in a single response.\n\n\
         Available tools:\n",
    );

    for tool in registry.enabled_tools() {
        desc.push_str(&format!("- **{}**: {}\n", tool.name(), tool.description()));
    }

    desc
}

/// Check if an API error indicates the model does not support tools.
fn is_tools_not_supported_error(err: &cc_api::errors::ApiError) -> bool {
    let msg = err.to_string().to_lowercase();
    msg.contains("does not support tools")
        || msg.contains("does not support functions")
        || msg.contains("tool_use is not supported")
        || msg.contains("tools is not supported")
}

/// Parse tool calls from text content (for models without native function calling).
///
/// Looks for ```tool_call ... ``` blocks, extracts JSON, and creates PendingToolCall entries.
fn parse_tool_calls_from_text(text: &str) -> Vec<PendingToolCall> {
    let mut calls = Vec::new();
    let mut search_from = 0;

    while let Some(start) = text[search_from..].find("```tool_call") {
        let abs_start = search_from + start;
        // Find the end of the opening fence line
        let json_start = match text[abs_start..].find('\n') {
            Some(pos) => abs_start + pos + 1,
            None => break,
        };
        // Find the closing fence
        let json_end = match text[json_start..].find("```") {
            Some(pos) => json_start + pos,
            None => break,
        };

        let json_str = text[json_start..json_end].trim();
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
            let name = parsed
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let arguments = parsed
                .get("arguments")
                .cloned()
                .unwrap_or(serde_json::json!({}));

            if !name.is_empty() {
                calls.push(PendingToolCall {
                    id: format!("text_call_{}", calls.len()),
                    name,
                    input: arguments,
                });
            }
        } else {
            tracing::warn!("Failed to parse text-based tool call: {}", json_str);
        }

        search_from = json_end + 3;
    }

    calls
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
            tools_supported: None,
            turn_timeout: None,
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
            tools_supported: None,
            turn_timeout: None,
        };

        let mut stream = std::pin::pin!(query(params));
        let mut events = Vec::new();
        while let Some(event) = stream.next().await {
            events.push(event);
        }

        // Streaming yields: TextDelta(s), UsageUpdate(s), TurnComplete
        let has_text = events
            .iter()
            .any(|e| matches!(e, QueryEvent::TextDelta(t) if t == "Hello!"));
        assert!(has_text, "expected TextDelta with 'Hello!'");
        let has_complete = events.iter().any(
            |e| matches!(e, QueryEvent::TurnComplete { stop_reason } if stop_reason == "end_turn"),
        );
        assert!(has_complete, "expected TurnComplete with 'end_turn'");
        let has_usage = events
            .iter()
            .any(|e| matches!(e, QueryEvent::UsageUpdate { .. }));
        assert!(has_usage, "expected at least one UsageUpdate");
    }

    #[tokio::test]
    async fn test_query_api_error() {
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
            tools_supported: None,
            turn_timeout: None,
        };

        let mut stream = std::pin::pin!(query(params));
        let event = stream.next().await.unwrap();
        assert!(matches!(event, QueryEvent::Error(msg) if msg.contains("connection refused")));
    }

    // --- Mock API client for tests ---

    /// Helper to convert a non-streaming response into a sequence of StreamEvents.
    fn response_to_stream_events(
        response: cc_api::types::MessagesResponse,
    ) -> Vec<cc_api::types::StreamEvent> {
        use cc_api::types::*;
        let mut events = Vec::new();

        // MessageStart
        events.push(StreamEvent::MessageStart {
            message: MessagesResponse {
                id: response.id.clone(),
                model: response.model.clone(),
                role: response.role,
                content: vec![],
                stop_reason: None,
                usage: response.usage.clone(),
            },
        });

        // Content blocks
        for (i, block) in response.content.iter().enumerate() {
            events.push(StreamEvent::ContentBlockStart {
                index: i,
                content_block: match block {
                    ContentBlock::Text { .. } => ContentBlock::Text {
                        text: String::new(),
                    },
                    other => other.clone(),
                },
            });
            match block {
                ContentBlock::Text { text } => {
                    events.push(StreamEvent::ContentBlockDelta {
                        index: i,
                        delta: ContentDelta::TextDelta { text: text.clone() },
                    });
                }
                ContentBlock::Thinking {
                    thinking,
                    signature,
                } => {
                    events.push(StreamEvent::ContentBlockDelta {
                        index: i,
                        delta: ContentDelta::ThinkingDelta {
                            thinking: thinking.clone(),
                        },
                    });
                    if let Some(sig) = signature {
                        events.push(StreamEvent::ContentBlockDelta {
                            index: i,
                            delta: ContentDelta::SignatureDelta {
                                signature: sig.clone(),
                            },
                        });
                    }
                }
                ContentBlock::ToolUse { input, .. } => {
                    let json = serde_json::to_string(input).unwrap_or_default();
                    events.push(StreamEvent::ContentBlockDelta {
                        index: i,
                        delta: ContentDelta::InputJsonDelta { partial_json: json },
                    });
                }
                _ => {}
            }
            events.push(StreamEvent::ContentBlockStop { index: i });
        }

        // MessageDelta with stop_reason
        events.push(StreamEvent::MessageDelta {
            delta: MessageDeltaBody {
                stop_reason: response.stop_reason.clone(),
            },
            usage: Some(response.usage.clone()),
        });

        events.push(StreamEvent::MessageStop);
        events
    }

    struct MockApiClient {
        responses: std::sync::Mutex<
            Vec<Result<cc_api::types::MessagesResponse, cc_api::errors::ApiError>>,
        >,
    }

    impl MockApiClient {
        fn new(
            responses: Vec<Result<cc_api::types::MessagesResponse, cc_api::errors::ApiError>>,
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
            let mut responses = self.responses.lock().unwrap();
            match responses.pop() {
                Some(Ok(response)) => {
                    let events = response_to_stream_events(response);
                    let stream = futures::stream::iter(events.into_iter().map(Ok));
                    Ok(Box::pin(stream))
                }
                Some(Err(e)) => Err(e),
                None => Err(cc_api::errors::ApiError::InvalidRequest {
                    message: "no more mock responses".to_string(),
                }),
            }
        }

        async fn send_messages(
            &self,
            _request: MessagesRequest,
        ) -> Result<cc_api::types::MessagesResponse, cc_api::errors::ApiError> {
            let mut responses = self.responses.lock().unwrap();
            responses
                .pop()
                .unwrap_or(Err(cc_api::errors::ApiError::InvalidRequest {
                    message: "no more mock responses".to_string(),
                }))
        }
    }

    // ── Tool-fallback tests ─────────────────────────────────────────

    #[test]
    fn test_is_tools_not_supported_error() {
        let err1 = cc_api::errors::ApiError::InvalidRequest {
            message: "registry.ollama.ai/library/dolphin-llama3:latest does not support tools"
                .into(),
        };
        assert!(is_tools_not_supported_error(&err1));

        let err2 = cc_api::errors::ApiError::InvalidRequest {
            message: "model does not support functions".into(),
        };
        assert!(is_tools_not_supported_error(&err2));

        let err3 = cc_api::errors::ApiError::InvalidRequest {
            message: "invalid max_tokens value".into(),
        };
        assert!(!is_tools_not_supported_error(&err3));
    }

    #[test]
    fn test_parse_tool_calls_from_text_valid() {
        let text = r#"I'll read the file for you.

```tool_call
{"name": "Read", "arguments": {"file_path": "/tmp/test.txt"}}
```

Let me also check the directory.

```tool_call
{"name": "Bash", "arguments": {"command": "ls /tmp"}}
```
"#;
        let calls = parse_tool_calls_from_text(text);
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].name, "Read");
        assert_eq!(calls[0].input["file_path"], "/tmp/test.txt");
        assert_eq!(calls[1].name, "Bash");
        assert_eq!(calls[1].input["command"], "ls /tmp");
    }

    #[test]
    fn test_parse_tool_calls_from_text_invalid_json() {
        let text = r#"```tool_call
{invalid json here}
```"#;
        let calls = parse_tool_calls_from_text(text);
        assert!(calls.is_empty());
    }

    #[test]
    fn test_parse_tool_calls_from_text_no_blocks() {
        let text = "Just a regular response with no tool calls.";
        let calls = parse_tool_calls_from_text(text);
        assert!(calls.is_empty());
    }

    #[test]
    fn test_parse_tool_calls_from_text_missing_name() {
        let text = r#"```tool_call
{"arguments": {"path": "/tmp"}}
```"#;
        let calls = parse_tool_calls_from_text(text);
        assert!(calls.is_empty()); // empty name → skipped
    }

    #[test]
    fn test_build_text_tool_descriptions() {
        let reg = ToolRegistry::new();
        let desc = build_text_tool_descriptions(&reg);
        assert!(desc.contains("tool_call"));
        assert!(desc.contains("Available tools:"));
    }
}
