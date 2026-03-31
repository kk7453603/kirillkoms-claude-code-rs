use std::collections::HashMap;

use crate::types::{
    ContentBlock, ContentDelta, MessagesResponse, Role, StreamEvent, Usage,
};

/// A parsed SSE event.
#[derive(Debug, Clone, PartialEq)]
pub struct SseEvent {
    pub event_type: Option<String>,
    pub data: String,
}

/// Parse a raw SSE line into an SseEvent.
///
/// SSE protocol:
/// - Lines starting with "event:" set the event type
/// - Lines starting with "data:" contain the data payload
/// - Empty lines dispatch the event
///
/// This function handles individual field lines. For a complete SSE parser,
/// you would accumulate lines and dispatch on empty lines.
pub fn parse_sse_line(line: &str) -> Option<SseEvent> {
    let line = line.trim_end_matches('\n').trim_end_matches('\r');

    if line.is_empty() {
        return None;
    }

    if let Some(data) = line.strip_prefix("data: ") {
        Some(SseEvent {
            event_type: None,
            data: data.to_string(),
        })
    } else if let Some(data) = line.strip_prefix("data:") {
        Some(SseEvent {
            event_type: None,
            data: data.to_string(),
        })
    } else if let Some(event_type) = line.strip_prefix("event: ") {
        Some(SseEvent {
            event_type: Some(event_type.to_string()),
            data: String::new(),
        })
    } else if let Some(event_type) = line.strip_prefix("event:") {
        Some(SseEvent {
            event_type: Some(event_type.to_string()),
            data: String::new(),
        })
    } else {
        // Comment lines (starting with ':') or unknown fields
        None
    }
}

/// Accumulates streaming events into a complete response.
#[derive(Debug, Default)]
pub struct StreamAccumulator {
    pub response: Option<MessagesResponse>,
    pub content_blocks: Vec<ContentBlock>,
    pub text_buffer: String,
    pub thinking_buffer: String,
    pub input_json_buffers: HashMap<usize, String>,
    pub usage: Usage,
    pub stop_reason: Option<String>,
}

impl StreamAccumulator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Process a single stream event and update internal state.
    pub fn process_event(&mut self, event: &StreamEvent) {
        match event {
            StreamEvent::MessageStart { message } => {
                self.usage.input_tokens = message.usage.input_tokens;
                self.usage.output_tokens = message.usage.output_tokens;
                self.usage.cache_read_input_tokens = message.usage.cache_read_input_tokens;
                self.usage.cache_creation_input_tokens =
                    message.usage.cache_creation_input_tokens;
                self.response = Some(message.clone());
            }
            StreamEvent::ContentBlockStart {
                index,
                content_block,
            } => {
                // Ensure content_blocks vec is large enough
                while self.content_blocks.len() <= *index {
                    self.content_blocks.push(ContentBlock::Text {
                        text: String::new(),
                    });
                }
                self.content_blocks[*index] = content_block.clone();
            }
            StreamEvent::ContentBlockDelta { index, delta } => {
                match delta {
                    ContentDelta::TextDelta { text } => {
                        self.text_buffer.push_str(text);
                        // Update the content block in place
                        if let Some(ContentBlock::Text { text: t }) =
                            self.content_blocks.get_mut(*index)
                        {
                            t.push_str(text);
                        }
                    }
                    ContentDelta::InputJsonDelta { partial_json } => {
                        let buf = self.input_json_buffers.entry(*index).or_default();
                        buf.push_str(partial_json);
                        // Update ToolUse content block
                        if let Some(ContentBlock::ToolUse { input, .. }) =
                            self.content_blocks.get_mut(*index)
                        {
                            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(
                                &self.input_json_buffers[index],
                            ) {
                                *input = parsed;
                            }
                        }
                    }
                    ContentDelta::ThinkingDelta { thinking } => {
                        self.thinking_buffer.push_str(thinking);
                        if let Some(ContentBlock::Thinking {
                            thinking: t,
                            ..
                        }) = self.content_blocks.get_mut(*index)
                        {
                            t.push_str(thinking);
                        }
                    }
                    ContentDelta::SignatureDelta { signature } => {
                        if let Some(ContentBlock::Thinking {
                            signature: s,
                            ..
                        }) = self.content_blocks.get_mut(*index)
                        {
                            let sig = s.get_or_insert_with(String::new);
                            sig.push_str(signature);
                        }
                    }
                }
            }
            StreamEvent::ContentBlockStop { .. } => {
                // Block is complete, nothing special to do
            }
            StreamEvent::MessageDelta { delta, usage } => {
                self.stop_reason = delta.stop_reason.clone();
                if let Some(u) = usage {
                    self.usage.output_tokens = u.output_tokens;
                }
            }
            StreamEvent::MessageStop => {
                // Stream complete
            }
            StreamEvent::Ping => {
                // Keep-alive, ignore
            }
            StreamEvent::Error { .. } => {
                // Error events are typically handled by the caller
            }
        }
    }

    /// Finalize the accumulator into a complete MessagesResponse.
    pub fn finalize(self) -> MessagesResponse {
        if let Some(mut response) = self.response {
            response.content = self.content_blocks;
            response.stop_reason = self.stop_reason;
            response.usage = self.usage;
            response
        } else {
            // Fallback: construct a minimal response
            MessagesResponse {
                id: String::new(),
                model: String::new(),
                role: Role::Assistant,
                content: self.content_blocks,
                stop_reason: self.stop_reason,
                usage: self.usage,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    #[test]
    fn parse_sse_line_data() {
        let result = parse_sse_line("data: {\"type\":\"ping\"}");
        assert_eq!(
            result,
            Some(SseEvent {
                event_type: None,
                data: "{\"type\":\"ping\"}".to_string(),
            })
        );
    }

    #[test]
    fn parse_sse_line_data_no_space() {
        let result = parse_sse_line("data:{\"type\":\"ping\"}");
        assert_eq!(
            result,
            Some(SseEvent {
                event_type: None,
                data: "{\"type\":\"ping\"}".to_string(),
            })
        );
    }

    #[test]
    fn parse_sse_line_event() {
        let result = parse_sse_line("event: message_start");
        assert_eq!(
            result,
            Some(SseEvent {
                event_type: Some("message_start".to_string()),
                data: String::new(),
            })
        );
    }

    #[test]
    fn parse_sse_line_event_no_space() {
        let result = parse_sse_line("event:message_start");
        assert_eq!(
            result,
            Some(SseEvent {
                event_type: Some("message_start".to_string()),
                data: String::new(),
            })
        );
    }

    #[test]
    fn parse_sse_line_empty() {
        assert_eq!(parse_sse_line(""), None);
    }

    #[test]
    fn parse_sse_line_comment() {
        assert_eq!(parse_sse_line(": this is a comment"), None);
    }

    #[test]
    fn parse_sse_line_unknown_field() {
        assert_eq!(parse_sse_line("id: 123"), None);
    }

    #[test]
    fn parse_sse_line_with_trailing_newline() {
        let result = parse_sse_line("data: hello\n");
        assert_eq!(
            result,
            Some(SseEvent {
                event_type: None,
                data: "hello".to_string(),
            })
        );
    }

    #[test]
    fn accumulator_message_start() {
        let mut acc = StreamAccumulator::new();
        let event = StreamEvent::MessageStart {
            message: MessagesResponse {
                id: "msg_1".to_string(),
                model: "claude-sonnet-4-20250514".to_string(),
                role: Role::Assistant,
                content: vec![],
                stop_reason: None,
                usage: Usage {
                    input_tokens: 100,
                    output_tokens: 0,
                    cache_read_input_tokens: 50,
                    cache_creation_input_tokens: 0,
                },
            },
        };
        acc.process_event(&event);
        assert!(acc.response.is_some());
        assert_eq!(acc.usage.input_tokens, 100);
        assert_eq!(acc.usage.cache_read_input_tokens, 50);
    }

    #[test]
    fn accumulator_text_streaming() {
        let mut acc = StreamAccumulator::new();

        // message_start
        acc.process_event(&StreamEvent::MessageStart {
            message: MessagesResponse {
                id: "msg_1".to_string(),
                model: "claude-sonnet-4-20250514".to_string(),
                role: Role::Assistant,
                content: vec![],
                stop_reason: None,
                usage: Usage {
                    input_tokens: 10,
                    output_tokens: 0,
                    ..Default::default()
                },
            },
        });

        // content_block_start
        acc.process_event(&StreamEvent::ContentBlockStart {
            index: 0,
            content_block: ContentBlock::Text {
                text: String::new(),
            },
        });

        // text deltas
        acc.process_event(&StreamEvent::ContentBlockDelta {
            index: 0,
            delta: ContentDelta::TextDelta {
                text: "Hello".to_string(),
            },
        });
        acc.process_event(&StreamEvent::ContentBlockDelta {
            index: 0,
            delta: ContentDelta::TextDelta {
                text: " world".to_string(),
            },
        });

        // content_block_stop
        acc.process_event(&StreamEvent::ContentBlockStop { index: 0 });

        // message_delta
        acc.process_event(&StreamEvent::MessageDelta {
            delta: MessageDeltaBody {
                stop_reason: Some("end_turn".to_string()),
            },
            usage: Some(Usage {
                output_tokens: 5,
                ..Default::default()
            }),
        });

        // message_stop
        acc.process_event(&StreamEvent::MessageStop);

        assert_eq!(acc.text_buffer, "Hello world");
        assert_eq!(acc.stop_reason, Some("end_turn".to_string()));
        assert_eq!(acc.usage.output_tokens, 5);

        let response = acc.finalize();
        assert_eq!(response.id, "msg_1");
        assert_eq!(response.content.len(), 1);
        match &response.content[0] {
            ContentBlock::Text { text } => assert_eq!(text, "Hello world"),
            _ => panic!("expected Text"),
        }
        assert_eq!(response.stop_reason, Some("end_turn".to_string()));
    }

    #[test]
    fn accumulator_thinking_streaming() {
        let mut acc = StreamAccumulator::new();

        acc.process_event(&StreamEvent::MessageStart {
            message: MessagesResponse {
                id: "msg_2".to_string(),
                model: "claude-sonnet-4-20250514".to_string(),
                role: Role::Assistant,
                content: vec![],
                stop_reason: None,
                usage: Usage::default(),
            },
        });

        acc.process_event(&StreamEvent::ContentBlockStart {
            index: 0,
            content_block: ContentBlock::Thinking {
                thinking: String::new(),
                signature: None,
            },
        });

        acc.process_event(&StreamEvent::ContentBlockDelta {
            index: 0,
            delta: ContentDelta::ThinkingDelta {
                thinking: "Let me think".to_string(),
            },
        });

        acc.process_event(&StreamEvent::ContentBlockDelta {
            index: 0,
            delta: ContentDelta::SignatureDelta {
                signature: "sig123".to_string(),
            },
        });

        acc.process_event(&StreamEvent::ContentBlockStop { index: 0 });

        assert_eq!(acc.thinking_buffer, "Let me think");

        let response = acc.finalize();
        match &response.content[0] {
            ContentBlock::Thinking {
                thinking,
                signature,
            } => {
                assert_eq!(thinking, "Let me think");
                assert_eq!(signature.as_deref(), Some("sig123"));
            }
            _ => panic!("expected Thinking"),
        }
    }

    #[test]
    fn accumulator_tool_use_streaming() {
        let mut acc = StreamAccumulator::new();

        acc.process_event(&StreamEvent::MessageStart {
            message: MessagesResponse {
                id: "msg_3".to_string(),
                model: "claude-sonnet-4-20250514".to_string(),
                role: Role::Assistant,
                content: vec![],
                stop_reason: None,
                usage: Usage::default(),
            },
        });

        acc.process_event(&StreamEvent::ContentBlockStart {
            index: 0,
            content_block: ContentBlock::ToolUse {
                id: "tu_1".to_string(),
                name: "bash".to_string(),
                input: serde_json::json!({}),
            },
        });

        acc.process_event(&StreamEvent::ContentBlockDelta {
            index: 0,
            delta: ContentDelta::InputJsonDelta {
                partial_json: r#"{"command":"#.to_string(),
            },
        });

        acc.process_event(&StreamEvent::ContentBlockDelta {
            index: 0,
            delta: ContentDelta::InputJsonDelta {
                partial_json: r#""ls"}"#.to_string(),
            },
        });

        acc.process_event(&StreamEvent::ContentBlockStop { index: 0 });

        let response = acc.finalize();
        match &response.content[0] {
            ContentBlock::ToolUse { id, name, input } => {
                assert_eq!(id, "tu_1");
                assert_eq!(name, "bash");
                assert_eq!(input["command"], "ls");
            }
            _ => panic!("expected ToolUse"),
        }
    }

    #[test]
    fn accumulator_ping_ignored() {
        let mut acc = StreamAccumulator::new();
        acc.process_event(&StreamEvent::Ping);
        assert!(acc.response.is_none());
        assert!(acc.content_blocks.is_empty());
    }

    #[test]
    fn accumulator_finalize_without_message_start() {
        let mut acc = StreamAccumulator::new();
        acc.process_event(&StreamEvent::ContentBlockStart {
            index: 0,
            content_block: ContentBlock::Text {
                text: "test".to_string(),
            },
        });
        let response = acc.finalize();
        assert_eq!(response.id, "");
        assert_eq!(response.content.len(), 1);
    }

    #[test]
    fn accumulator_multiple_content_blocks() {
        let mut acc = StreamAccumulator::new();

        acc.process_event(&StreamEvent::MessageStart {
            message: MessagesResponse {
                id: "msg_4".to_string(),
                model: "claude-sonnet-4-20250514".to_string(),
                role: Role::Assistant,
                content: vec![],
                stop_reason: None,
                usage: Usage::default(),
            },
        });

        // First block: text
        acc.process_event(&StreamEvent::ContentBlockStart {
            index: 0,
            content_block: ContentBlock::Text {
                text: String::new(),
            },
        });
        acc.process_event(&StreamEvent::ContentBlockDelta {
            index: 0,
            delta: ContentDelta::TextDelta {
                text: "I'll run a command.".to_string(),
            },
        });
        acc.process_event(&StreamEvent::ContentBlockStop { index: 0 });

        // Second block: tool_use
        acc.process_event(&StreamEvent::ContentBlockStart {
            index: 1,
            content_block: ContentBlock::ToolUse {
                id: "tu_1".to_string(),
                name: "bash".to_string(),
                input: serde_json::json!({}),
            },
        });
        acc.process_event(&StreamEvent::ContentBlockDelta {
            index: 1,
            delta: ContentDelta::InputJsonDelta {
                partial_json: r#"{"command":"ls"}"#.to_string(),
            },
        });
        acc.process_event(&StreamEvent::ContentBlockStop { index: 1 });

        acc.process_event(&StreamEvent::MessageDelta {
            delta: MessageDeltaBody {
                stop_reason: Some("tool_use".to_string()),
            },
            usage: Some(Usage {
                output_tokens: 20,
                ..Default::default()
            }),
        });

        let response = acc.finalize();
        assert_eq!(response.content.len(), 2);
        assert!(matches!(&response.content[0], ContentBlock::Text { .. }));
        assert!(matches!(&response.content[1], ContentBlock::ToolUse { .. }));
        assert_eq!(response.stop_reason, Some("tool_use".to_string()));
    }
}
