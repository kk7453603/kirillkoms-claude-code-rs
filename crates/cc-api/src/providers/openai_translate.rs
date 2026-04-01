//! Translation layer between Anthropic Messages API types and OpenAI Chat Completions API types.
//!
//! All translation happens at the provider boundary — the rest of the codebase
//! only sees Anthropic-format types.

use std::collections::HashMap;

use crate::providers::openai_types::*;
use crate::types::{
    ApiMessage, ContentBlock, ContentDelta, ImageSource, MessageDeltaBody, MessagesRequest,
    MessagesResponse, Role, StreamEvent, SystemBlock, ToolChoice, ToolDefinition, Usage,
};

// ── Request translation ─────────────────────────────────────────────────

/// Translate an Anthropic `MessagesRequest` into an OpenAI `ChatCompletionRequest`.
pub(crate) fn translate_request(req: &MessagesRequest) -> ChatCompletionRequest {
    let mut messages = Vec::new();

    // System blocks → single system message
    let system_text: String = req
        .system
        .iter()
        .map(|s| match s {
            SystemBlock::Text { text, .. } => text.as_str(),
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    if !system_text.is_empty() {
        messages.push(ChatMessage {
            role: "system".to_string(),
            content: Some(serde_json::json!(system_text)),
            tool_calls: None,
            tool_call_id: None,
        });
    }

    // Convert messages
    for msg in &req.messages {
        translate_message(msg, &mut messages);
    }

    // Convert tools
    let tools = req.tools.as_ref().map(|tools| {
        tools
            .iter()
            .map(translate_tool_definition)
            .collect()
    });

    // Convert tool_choice
    let tool_choice = req.tool_choice.as_ref().map(|tc| match tc {
        ToolChoice::Auto => serde_json::json!("auto"),
        ToolChoice::Any => serde_json::json!("required"),
        ToolChoice::Tool { name } => serde_json::json!({
            "type": "function",
            "function": { "name": name }
        }),
    });

    // Stream options: request usage info when streaming
    let stream_options = if req.stream {
        Some(StreamOptions {
            include_usage: true,
        })
    } else {
        None
    };

    ChatCompletionRequest {
        model: req.model.clone(),
        messages,
        max_tokens: req.max_tokens,
        temperature: req.temperature,
        tools,
        tool_choice,
        stream: req.stream,
        stream_options,
    }
}

/// Translate a single Anthropic message into one or more OpenAI messages.
///
/// Anthropic puts ToolResult blocks inside User messages, but OpenAI requires
/// separate messages with `role: "tool"`. This function may emit multiple
/// OpenAI messages from a single Anthropic message.
fn translate_message(msg: &ApiMessage, out: &mut Vec<ChatMessage>) {
    match msg.role {
        Role::User => translate_user_message(msg, out),
        Role::Assistant => translate_assistant_message(msg, out),
    }
}

fn translate_user_message(msg: &ApiMessage, out: &mut Vec<ChatMessage>) {
    let mut tool_results = Vec::new();
    let mut content_parts = Vec::new();

    for block in &msg.content {
        match block {
            ContentBlock::ToolResult {
                tool_use_id,
                content,
                is_error,
            } => {
                // Convert content to string for OpenAI
                let text = match content {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                };

                let mut tool_msg = ChatMessage {
                    role: "tool".to_string(),
                    content: Some(serde_json::json!(text)),
                    tool_calls: None,
                    tool_call_id: Some(tool_use_id.clone()),
                };

                // If error, prefix the content
                if is_error == &Some(true) {
                    tool_msg.content =
                        Some(serde_json::json!(format!("[ERROR] {}", text)));
                }

                tool_results.push(tool_msg);
            }
            ContentBlock::Text { text } => {
                content_parts.push(serde_json::json!({"type": "text", "text": text}));
            }
            ContentBlock::Image { source } => {
                content_parts.push(translate_image(source));
            }
            ContentBlock::Thinking { .. } => {
                // No OpenAI equivalent; skip
            }
            ContentBlock::ToolUse { .. } => {
                // ToolUse in a user message is unusual; skip
            }
        }
    }

    // Emit tool result messages first (order matters for the API)
    out.extend(tool_results);

    // Then emit the user content if any
    if !content_parts.is_empty() {
        let content = if content_parts.len() == 1 {
            // Single text block: use plain string for better compatibility
            if let Some(text) = content_parts[0].get("text") {
                text.clone()
            } else {
                serde_json::json!(content_parts)
            }
        } else {
            serde_json::json!(content_parts)
        };

        out.push(ChatMessage {
            role: "user".to_string(),
            content: Some(content),
            tool_calls: None,
            tool_call_id: None,
        });
    }
}

fn translate_assistant_message(msg: &ApiMessage, out: &mut Vec<ChatMessage>) {
    let mut text_parts = Vec::new();
    let mut tool_calls = Vec::new();

    for block in &msg.content {
        match block {
            ContentBlock::Text { text } => {
                text_parts.push(text.clone());
            }
            ContentBlock::ToolUse { id, name, input } => {
                tool_calls.push(ToolCall {
                    id: id.clone(),
                    call_type: "function".to_string(),
                    function: FunctionCall {
                        name: name.clone(),
                        arguments: input.to_string(),
                    },
                });
            }
            ContentBlock::Thinking { .. } | ContentBlock::Image { .. } | ContentBlock::ToolResult { .. } => {
                // Skip non-applicable blocks
            }
        }
    }

    let content = if text_parts.is_empty() {
        None
    } else {
        Some(serde_json::json!(text_parts.join("")))
    };

    let tool_calls_opt = if tool_calls.is_empty() {
        None
    } else {
        Some(tool_calls)
    };

    out.push(ChatMessage {
        role: "assistant".to_string(),
        content,
        tool_calls: tool_calls_opt,
        tool_call_id: None,
    });
}

fn translate_image(source: &ImageSource) -> serde_json::Value {
    let data_uri = format!("data:{};base64,{}", source.media_type, source.data);
    serde_json::json!({
        "type": "image_url",
        "image_url": { "url": data_uri }
    })
}

fn translate_tool_definition(tool: &ToolDefinition) -> ChatTool {
    ChatTool {
        tool_type: "function".to_string(),
        function: FunctionDefinition {
            name: tool.name.clone(),
            description: tool.description.clone(),
            parameters: tool.input_schema.clone(),
        },
    }
}

// ── Response translation (non-streaming) ────────────────────────────────

/// Translate an OpenAI `ChatCompletionResponse` into an Anthropic `MessagesResponse`.
pub(crate) fn translate_response(resp: ChatCompletionResponse) -> MessagesResponse {
    let choice = resp.choices.into_iter().next();

    let mut content = Vec::new();
    let mut stop_reason = None;

    if let Some(c) = &choice {
        if let Some(ref msg) = c.message {
            // Text content
            if let Some(ref text) = msg.content
                && !text.is_empty()
            {
                content.push(ContentBlock::Text {
                    text: text.clone(),
                });
            }

            // Tool calls
            if let Some(ref tool_calls) = msg.tool_calls {
                for tc in tool_calls {
                    let input = serde_json::from_str(&tc.function.arguments)
                        .unwrap_or(serde_json::json!({}));
                    content.push(ContentBlock::ToolUse {
                        id: tc.id.clone(),
                        name: tc.function.name.clone(),
                        input,
                    });
                }
            }
        }

        stop_reason = c.finish_reason.as_deref().map(translate_finish_reason);
    }

    let usage = resp.usage.map(translate_usage).unwrap_or_default();

    MessagesResponse {
        id: resp.id,
        model: resp.model,
        role: Role::Assistant,
        content,
        stop_reason,
        usage,
    }
}

fn translate_finish_reason(reason: &str) -> String {
    match reason {
        "stop" => "end_turn".to_string(),
        "tool_calls" => "tool_use".to_string(),
        "length" => "max_tokens".to_string(),
        other => other.to_string(),
    }
}

fn translate_usage(usage: ChatUsage) -> Usage {
    Usage {
        input_tokens: usage.prompt_tokens,
        output_tokens: usage.completion_tokens,
        cache_read_input_tokens: 0,
        cache_creation_input_tokens: 0,
    }
}

// ── Streaming translation ───────────────────────────────────────────────

/// State machine for translating OpenAI streaming chunks into Anthropic StreamEvents.
pub(crate) struct StreamTranslationState {
    message_started: bool,
    current_content_index: usize,
    text_block_started: bool,
    /// Tracks active tool calls by their OpenAI-side index.
    active_tool_calls: HashMap<usize, ToolCallAccumulator>,
    model: String,
    accumulated_usage: Option<ChatUsage>,
}

struct ToolCallAccumulator {
    name: String,
    content_block_index: usize,
    started: bool,
}

impl StreamTranslationState {
    pub fn new() -> Self {
        Self {
            message_started: false,
            current_content_index: 0,
            text_block_started: false,
            active_tool_calls: HashMap::new(),
            model: String::new(),
            accumulated_usage: None,
        }
    }
}

/// Translate a single OpenAI streaming chunk into zero or more Anthropic StreamEvents.
pub(crate) fn translate_stream_chunk(
    chunk: ChatCompletionResponse,
    state: &mut StreamTranslationState,
) -> Vec<StreamEvent> {
    let mut events = Vec::new();

    // Emit MessageStart on first chunk
    if !state.message_started {
        state.message_started = true;
        if !chunk.model.is_empty() {
            state.model = chunk.model.clone();
        }
        events.push(StreamEvent::MessageStart {
            message: MessagesResponse {
                id: chunk.id.clone(),
                model: state.model.clone(),
                role: Role::Assistant,
                content: vec![],
                stop_reason: None,
                usage: Usage::default(),
            },
        });
    }

    // Save usage if present (often arrives with the final chunk)
    if let Some(usage) = chunk.usage {
        state.accumulated_usage = Some(usage);
    }

    let choice = match chunk.choices.into_iter().next() {
        Some(c) => c,
        None => return events,
    };

    if let Some(delta) = &choice.delta {
        // Handle text content
        if let Some(ref text) = delta.content
            && !text.is_empty()
        {
            if !state.text_block_started {
                state.text_block_started = true;
                events.push(StreamEvent::ContentBlockStart {
                    index: state.current_content_index,
                    content_block: ContentBlock::Text {
                        text: String::new(),
                    },
                });
            }
            events.push(StreamEvent::ContentBlockDelta {
                index: state.current_content_index,
                delta: ContentDelta::TextDelta {
                    text: text.clone(),
                },
            });
        }

        // Handle tool calls
        if let Some(ref tool_calls) = delta.tool_calls {
            for tc_delta in tool_calls {
                let tc_index = tc_delta.index;

                if !state.active_tool_calls.contains_key(&tc_index) {
                    // New tool call starting — close text block if open
                    if state.text_block_started {
                        events.push(StreamEvent::ContentBlockStop {
                            index: state.current_content_index,
                        });
                        state.current_content_index += 1;
                        state.text_block_started = false;
                    }

                    // Close previous tool call block if any
                    // (in case of multiple sequential tool calls)
                    // Previous tool calls would already be in the map,
                    // but their blocks are started, so no need to close here —
                    // we close them when a new one starts at a different index.

                    let id = tc_delta.id.clone().unwrap_or_default();
                    let name = tc_delta
                        .function
                        .as_ref()
                        .and_then(|f| f.name.clone())
                        .unwrap_or_default();

                    let content_idx = state.current_content_index;

                    state.active_tool_calls.insert(
                        tc_index,
                        ToolCallAccumulator {
                            name: name.clone(),
                            content_block_index: content_idx,
                            started: true,
                        },
                    );

                    events.push(StreamEvent::ContentBlockStart {
                        index: content_idx,
                        content_block: ContentBlock::ToolUse {
                            id,
                            name,
                            input: serde_json::json!({}),
                        },
                    });

                    state.current_content_index += 1;
                }

                // Emit argument deltas
                if let Some(ref func) = tc_delta.function {
                    if let Some(ref args) = func.arguments
                        && !args.is_empty()
                    {
                        let acc = &state.active_tool_calls[&tc_index];
                        events.push(StreamEvent::ContentBlockDelta {
                            index: acc.content_block_index,
                            delta: ContentDelta::InputJsonDelta {
                                partial_json: args.clone(),
                            },
                        });
                    }

                    // Update name if it arrives in a later delta
                    if let Some(ref name) = func.name
                        && let Some(acc) = state.active_tool_calls.get_mut(&tc_index)
                        && acc.name.is_empty()
                    {
                        acc.name = name.clone();
                    }
                }
            }
        }
    }

    // Handle finish
    if let Some(ref finish_reason) = choice.finish_reason {
        // Close any open text block
        if state.text_block_started {
            events.push(StreamEvent::ContentBlockStop {
                index: state.current_content_index,
            });
            state.text_block_started = false;
        }

        // Close all open tool call blocks
        let mut tool_indices: Vec<usize> = state.active_tool_calls.keys().copied().collect();
        tool_indices.sort();
        for idx in tool_indices {
            let acc = &state.active_tool_calls[&idx];
            if acc.started {
                events.push(StreamEvent::ContentBlockStop {
                    index: acc.content_block_index,
                });
            }
        }

        // Build usage from accumulated data
        let usage = state.accumulated_usage.take().map(|u| Usage {
            input_tokens: u.prompt_tokens,
            output_tokens: u.completion_tokens,
            cache_read_input_tokens: 0,
            cache_creation_input_tokens: 0,
        });

        events.push(StreamEvent::MessageDelta {
            delta: MessageDeltaBody {
                stop_reason: Some(translate_finish_reason(finish_reason)),
            },
            usage,
        });

        events.push(StreamEvent::MessageStop);
    }

    events
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    // ── Request translation tests ───────────────────────────────────────

    #[test]
    fn translate_simple_text_request() {
        let req = MessagesRequest {
            model: "gpt-4o".to_string(),
            messages: vec![ApiMessage {
                role: Role::User,
                content: vec![ContentBlock::Text {
                    text: "Hello".to_string(),
                }],
            }],
            system: vec![SystemBlock::Text {
                text: "Be helpful.".to_string(),
                cache_control: None,
            }],
            max_tokens: Some(1024),
            temperature: None,
            tools: None,
            tool_choice: None,
            thinking: None,
            stream: false,
            metadata: None,
        };

        let result = translate_request(&req);
        assert_eq!(result.model, "gpt-4o");
        assert_eq!(result.messages.len(), 2); // system + user
        assert_eq!(result.messages[0].role, "system");
        assert_eq!(result.messages[0].content, Some(serde_json::json!("Be helpful.")));
        assert_eq!(result.messages[1].role, "user");
        assert_eq!(result.messages[1].content, Some(serde_json::json!("Hello")));
        assert_eq!(result.max_tokens, Some(1024));
        assert!(!result.stream);
    }

    #[test]
    fn translate_request_with_tools() {
        let req = MessagesRequest {
            model: "gpt-4o".to_string(),
            messages: vec![],
            system: vec![],
            max_tokens: Some(4096),
            temperature: Some(0.7),
            tools: Some(vec![ToolDefinition {
                name: "bash".to_string(),
                description: "Run a command".to_string(),
                input_schema: serde_json::json!({"type": "object", "properties": {"cmd": {"type": "string"}}}),
            }]),
            tool_choice: Some(ToolChoice::Auto),
            thinking: None,
            stream: true,
            metadata: None,
        };

        let result = translate_request(&req);
        assert!(result.tools.is_some());
        let tools = result.tools.unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].tool_type, "function");
        assert_eq!(tools[0].function.name, "bash");
        assert_eq!(result.tool_choice, Some(serde_json::json!("auto")));
        assert!(result.stream_options.is_some());
    }

    #[test]
    fn translate_tool_choice_variants() {
        // Any → required
        let req = MessagesRequest {
            model: "m".into(), messages: vec![], system: vec![],
            max_tokens: None, temperature: None, tools: None,
            tool_choice: Some(ToolChoice::Any),
            thinking: None, stream: false, metadata: None,
        };
        let result = translate_request(&req);
        assert_eq!(result.tool_choice, Some(serde_json::json!("required")));

        // Tool { name } → object
        let req2 = MessagesRequest {
            model: "m".into(), messages: vec![], system: vec![],
            max_tokens: None, temperature: None, tools: None,
            tool_choice: Some(ToolChoice::Tool { name: "bash".to_string() }),
            thinking: None, stream: false, metadata: None,
        };
        let result2 = translate_request(&req2);
        assert_eq!(
            result2.tool_choice,
            Some(serde_json::json!({"type": "function", "function": {"name": "bash"}}))
        );
    }

    #[test]
    fn translate_user_message_with_tool_results() {
        let msg = ApiMessage {
            role: Role::User,
            content: vec![
                ContentBlock::ToolResult {
                    tool_use_id: "call_1".to_string(),
                    content: serde_json::json!("file contents"),
                    is_error: Some(false),
                },
                ContentBlock::Text {
                    text: "Now analyze this".to_string(),
                },
            ],
        };

        let mut out = Vec::new();
        translate_message(&msg, &mut out);

        // Should produce: tool message, then user message
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].role, "tool");
        assert_eq!(out[0].tool_call_id, Some("call_1".to_string()));
        assert_eq!(out[1].role, "user");
        assert_eq!(out[1].content, Some(serde_json::json!("Now analyze this")));
    }

    #[test]
    fn translate_user_message_with_error_tool_result() {
        let msg = ApiMessage {
            role: Role::User,
            content: vec![ContentBlock::ToolResult {
                tool_use_id: "call_2".to_string(),
                content: serde_json::json!("permission denied"),
                is_error: Some(true),
            }],
        };

        let mut out = Vec::new();
        translate_message(&msg, &mut out);

        assert_eq!(out.len(), 1);
        assert_eq!(
            out[0].content,
            Some(serde_json::json!("[ERROR] permission denied"))
        );
    }

    #[test]
    fn translate_assistant_message_with_tool_use() {
        let msg = ApiMessage {
            role: Role::Assistant,
            content: vec![
                ContentBlock::Text {
                    text: "Let me run that.".to_string(),
                },
                ContentBlock::ToolUse {
                    id: "call_3".to_string(),
                    name: "bash".to_string(),
                    input: serde_json::json!({"command": "ls"}),
                },
            ],
        };

        let mut out = Vec::new();
        translate_message(&msg, &mut out);

        assert_eq!(out.len(), 1);
        let m = &out[0];
        assert_eq!(m.role, "assistant");
        assert_eq!(m.content, Some(serde_json::json!("Let me run that.")));
        assert!(m.tool_calls.is_some());
        let tc = &m.tool_calls.as_ref().unwrap()[0];
        assert_eq!(tc.id, "call_3");
        assert_eq!(tc.function.name, "bash");
    }

    #[test]
    fn translate_thinking_blocks_skipped() {
        let msg = ApiMessage {
            role: Role::Assistant,
            content: vec![
                ContentBlock::Thinking {
                    thinking: "hmm...".to_string(),
                    signature: None,
                },
                ContentBlock::Text {
                    text: "Result".to_string(),
                },
            ],
        };

        let mut out = Vec::new();
        translate_message(&msg, &mut out);

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].content, Some(serde_json::json!("Result")));
        assert!(out[0].tool_calls.is_none());
    }

    #[test]
    fn translate_image_content() {
        let msg = ApiMessage {
            role: Role::User,
            content: vec![
                ContentBlock::Text {
                    text: "What's this?".to_string(),
                },
                ContentBlock::Image {
                    source: ImageSource {
                        source_type: "base64".to_string(),
                        media_type: "image/png".to_string(),
                        data: "abc123".to_string(),
                    },
                },
            ],
        };

        let mut out = Vec::new();
        translate_message(&msg, &mut out);

        assert_eq!(out.len(), 1);
        // Should be array content (multipart)
        let content = out[0].content.as_ref().unwrap();
        assert!(content.is_array());
        let parts = content.as_array().unwrap();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[1]["type"], "image_url");
        assert!(parts[1]["image_url"]["url"]
            .as_str()
            .unwrap()
            .starts_with("data:image/png;base64,"));
    }

    #[test]
    fn translate_multiple_system_blocks() {
        let req = MessagesRequest {
            model: "m".into(),
            messages: vec![],
            system: vec![
                SystemBlock::Text { text: "First.".into(), cache_control: None },
                SystemBlock::Text { text: "Second.".into(), cache_control: None },
            ],
            max_tokens: None, temperature: None, tools: None,
            tool_choice: None, thinking: None, stream: false, metadata: None,
        };

        let result = translate_request(&req);
        assert_eq!(result.messages.len(), 1);
        assert_eq!(
            result.messages[0].content,
            Some(serde_json::json!("First.\n\nSecond."))
        );
    }

    // ── Response translation tests ──────────────────────────────────────

    #[test]
    fn translate_simple_response() {
        let resp = ChatCompletionResponse {
            id: "chatcmpl-123".to_string(),
            model: "gpt-4o".to_string(),
            choices: vec![Choice {
                index: 0,
                message: Some(ResponseMessage {
                    role: Some("assistant".to_string()),
                    content: Some("Hello!".to_string()),
                    tool_calls: None,
                }),
                delta: None,
                finish_reason: Some("stop".to_string()),
            }],
            usage: Some(ChatUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
            }),
        };

        let result = translate_response(resp);
        assert_eq!(result.id, "chatcmpl-123");
        assert_eq!(result.model, "gpt-4o");
        assert_eq!(result.role, Role::Assistant);
        assert_eq!(result.content.len(), 1);
        match &result.content[0] {
            ContentBlock::Text { text } => assert_eq!(text, "Hello!"),
            _ => panic!("expected Text"),
        }
        assert_eq!(result.stop_reason, Some("end_turn".to_string()));
        assert_eq!(result.usage.input_tokens, 10);
        assert_eq!(result.usage.output_tokens, 5);
    }

    #[test]
    fn translate_response_with_tool_calls() {
        let resp = ChatCompletionResponse {
            id: "chatcmpl-456".to_string(),
            model: "gpt-4o".to_string(),
            choices: vec![Choice {
                index: 0,
                message: Some(ResponseMessage {
                    role: Some("assistant".to_string()),
                    content: None,
                    tool_calls: Some(vec![ToolCall {
                        id: "call_abc".to_string(),
                        call_type: "function".to_string(),
                        function: FunctionCall {
                            name: "bash".to_string(),
                            arguments: r#"{"command":"ls"}"#.to_string(),
                        },
                    }]),
                }),
                delta: None,
                finish_reason: Some("tool_calls".to_string()),
            }],
            usage: Some(ChatUsage {
                prompt_tokens: 20,
                completion_tokens: 15,
            }),
        };

        let result = translate_response(resp);
        assert_eq!(result.content.len(), 1);
        match &result.content[0] {
            ContentBlock::ToolUse { id, name, input } => {
                assert_eq!(id, "call_abc");
                assert_eq!(name, "bash");
                assert_eq!(input["command"], "ls");
            }
            _ => panic!("expected ToolUse"),
        }
        assert_eq!(result.stop_reason, Some("tool_use".to_string()));
    }

    #[test]
    fn translate_finish_reasons() {
        assert_eq!(translate_finish_reason("stop"), "end_turn");
        assert_eq!(translate_finish_reason("tool_calls"), "tool_use");
        assert_eq!(translate_finish_reason("length"), "max_tokens");
        assert_eq!(translate_finish_reason("content_filter"), "content_filter");
    }

    #[test]
    fn translate_empty_response() {
        let resp = ChatCompletionResponse {
            id: String::new(),
            model: String::new(),
            choices: vec![],
            usage: None,
        };
        let result = translate_response(resp);
        assert!(result.content.is_empty());
        assert!(result.stop_reason.is_none());
    }

    // ── Streaming translation tests ─────────────────────────────────────

    #[test]
    fn stream_simple_text() {
        let mut state = StreamTranslationState::new();

        // First chunk: role delta
        let chunk1 = ChatCompletionResponse {
            id: "chatcmpl-1".to_string(),
            model: "gpt-4o".to_string(),
            choices: vec![Choice {
                index: 0,
                message: None,
                delta: Some(ResponseDelta {
                    role: Some("assistant".to_string()),
                    content: None,
                    tool_calls: None,
                }),
                finish_reason: None,
            }],
            usage: None,
        };
        let events1 = translate_stream_chunk(chunk1, &mut state);
        assert_eq!(events1.len(), 1);
        assert!(matches!(events1[0], StreamEvent::MessageStart { .. }));

        // Second chunk: text
        let chunk2 = ChatCompletionResponse {
            id: "chatcmpl-1".to_string(),
            model: "gpt-4o".to_string(),
            choices: vec![Choice {
                index: 0,
                message: None,
                delta: Some(ResponseDelta {
                    role: None,
                    content: Some("Hello".to_string()),
                    tool_calls: None,
                }),
                finish_reason: None,
            }],
            usage: None,
        };
        let events2 = translate_stream_chunk(chunk2, &mut state);
        assert_eq!(events2.len(), 2); // ContentBlockStart + ContentBlockDelta
        assert!(matches!(events2[0], StreamEvent::ContentBlockStart { .. }));
        assert!(matches!(events2[1], StreamEvent::ContentBlockDelta { .. }));

        // Third chunk: more text
        let chunk3 = ChatCompletionResponse {
            id: "chatcmpl-1".to_string(),
            model: "gpt-4o".to_string(),
            choices: vec![Choice {
                index: 0,
                message: None,
                delta: Some(ResponseDelta {
                    role: None,
                    content: Some(" world".to_string()),
                    tool_calls: None,
                }),
                finish_reason: None,
            }],
            usage: None,
        };
        let events3 = translate_stream_chunk(chunk3, &mut state);
        assert_eq!(events3.len(), 1); // just ContentBlockDelta
        assert!(matches!(events3[0], StreamEvent::ContentBlockDelta { .. }));

        // Final chunk: finish
        let chunk4 = ChatCompletionResponse {
            id: "chatcmpl-1".to_string(),
            model: "gpt-4o".to_string(),
            choices: vec![Choice {
                index: 0,
                message: None,
                delta: Some(ResponseDelta {
                    role: None,
                    content: None,
                    tool_calls: None,
                }),
                finish_reason: Some("stop".to_string()),
            }],
            usage: Some(ChatUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
            }),
        };
        let events4 = translate_stream_chunk(chunk4, &mut state);
        // ContentBlockStop + MessageDelta + MessageStop
        assert_eq!(events4.len(), 3);
        assert!(matches!(events4[0], StreamEvent::ContentBlockStop { .. }));
        assert!(matches!(events4[1], StreamEvent::MessageDelta { .. }));
        assert!(matches!(events4[2], StreamEvent::MessageStop));

        // Verify usage in MessageDelta
        if let StreamEvent::MessageDelta { usage, delta } = &events4[1] {
            assert_eq!(delta.stop_reason, Some("end_turn".to_string()));
            let u = usage.as_ref().unwrap();
            assert_eq!(u.input_tokens, 10);
            assert_eq!(u.output_tokens, 5);
        }
    }

    #[test]
    fn stream_tool_call() {
        let mut state = StreamTranslationState::new();

        // First chunk with role
        let chunk1 = ChatCompletionResponse {
            id: "c-1".into(),
            model: "gpt-4o".into(),
            choices: vec![Choice {
                index: 0, message: None,
                delta: Some(ResponseDelta {
                    role: Some("assistant".into()),
                    content: None,
                    tool_calls: None,
                }),
                finish_reason: None,
            }],
            usage: None,
        };
        translate_stream_chunk(chunk1, &mut state);

        // Tool call start
        let chunk2 = ChatCompletionResponse {
            id: "c-1".into(),
            model: "gpt-4o".into(),
            choices: vec![Choice {
                index: 0, message: None,
                delta: Some(ResponseDelta {
                    role: None,
                    content: None,
                    tool_calls: Some(vec![ToolCallDelta {
                        index: 0,
                        id: Some("call_1".into()),
                        call_type: Some("function".into()),
                        function: Some(FunctionCallDelta {
                            name: Some("bash".into()),
                            arguments: Some(r#"{"com"#.into()),
                        }),
                    }]),
                }),
                finish_reason: None,
            }],
            usage: None,
        };
        let events2 = translate_stream_chunk(chunk2, &mut state);
        assert!(events2.iter().any(|e| matches!(e, StreamEvent::ContentBlockStart {
            content_block: ContentBlock::ToolUse { name, .. }, ..
        } if name == "bash")));
        assert!(events2.iter().any(|e| matches!(e, StreamEvent::ContentBlockDelta {
            delta: ContentDelta::InputJsonDelta { .. }, ..
        })));

        // Tool call argument continuation
        let chunk3 = ChatCompletionResponse {
            id: "c-1".into(),
            model: "gpt-4o".into(),
            choices: vec![Choice {
                index: 0, message: None,
                delta: Some(ResponseDelta {
                    role: None,
                    content: None,
                    tool_calls: Some(vec![ToolCallDelta {
                        index: 0,
                        id: None,
                        call_type: None,
                        function: Some(FunctionCallDelta {
                            name: None,
                            arguments: Some(r#"mand":"ls"}"#.into()),
                        }),
                    }]),
                }),
                finish_reason: None,
            }],
            usage: None,
        };
        let events3 = translate_stream_chunk(chunk3, &mut state);
        assert_eq!(events3.len(), 1);
        assert!(matches!(&events3[0], StreamEvent::ContentBlockDelta {
            delta: ContentDelta::InputJsonDelta { partial_json }, ..
        } if partial_json == r#"mand":"ls"}"#));

        // Finish
        let chunk4 = ChatCompletionResponse {
            id: "c-1".into(),
            model: "gpt-4o".into(),
            choices: vec![Choice {
                index: 0, message: None,
                delta: Some(ResponseDelta {
                    role: None, content: None, tool_calls: None,
                }),
                finish_reason: Some("tool_calls".into()),
            }],
            usage: None,
        };
        let events4 = translate_stream_chunk(chunk4, &mut state);
        // ContentBlockStop + MessageDelta + MessageStop
        assert!(events4.iter().any(|e| matches!(e, StreamEvent::ContentBlockStop { .. })));
        assert!(events4.iter().any(|e| matches!(e, StreamEvent::MessageDelta {
            delta: MessageDeltaBody { stop_reason: Some(r) }, ..
        } if r == "tool_use")));
        assert!(events4.iter().any(|e| matches!(e, StreamEvent::MessageStop)));
    }

    #[test]
    fn stream_text_then_tool_call() {
        let mut state = StreamTranslationState::new();

        // MessageStart
        let chunk1 = ChatCompletionResponse {
            id: "c-2".into(), model: "m".into(),
            choices: vec![Choice {
                index: 0, message: None,
                delta: Some(ResponseDelta {
                    role: Some("assistant".into()), content: Some("Let me check.".into()),
                    tool_calls: None,
                }),
                finish_reason: None,
            }],
            usage: None,
        };
        let events1 = translate_stream_chunk(chunk1, &mut state);
        // MessageStart + ContentBlockStart + ContentBlockDelta
        assert_eq!(events1.len(), 3);

        // Then tool call starts — should close text block
        let chunk2 = ChatCompletionResponse {
            id: "c-2".into(), model: "m".into(),
            choices: vec![Choice {
                index: 0, message: None,
                delta: Some(ResponseDelta {
                    role: None, content: None,
                    tool_calls: Some(vec![ToolCallDelta {
                        index: 0,
                        id: Some("call_x".into()),
                        call_type: Some("function".into()),
                        function: Some(FunctionCallDelta {
                            name: Some("read".into()),
                            arguments: Some("{}".into()),
                        }),
                    }]),
                }),
                finish_reason: None,
            }],
            usage: None,
        };
        let events2 = translate_stream_chunk(chunk2, &mut state);
        // Should contain: ContentBlockStop (for text), ContentBlockStart (for tool), ContentBlockDelta (args)
        assert!(events2.iter().any(|e| matches!(e, StreamEvent::ContentBlockStop { index: 0 })));
        assert!(events2.iter().any(|e| matches!(e, StreamEvent::ContentBlockStart { index: 1, .. })));
    }
}
