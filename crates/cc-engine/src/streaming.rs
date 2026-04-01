use cc_api::types::{ContentBlock, ContentDelta, StreamEvent, Usage};

use crate::orchestration::PendingToolCall;

/// Accumulated state from streaming events.
#[derive(Debug, Default)]
pub struct StreamState {
    pub content_blocks: Vec<ContentBlock>,
    pub text_parts: Vec<String>,
    pub tool_uses: Vec<PendingToolCall>,
    pub stop_reason: Option<String>,
    pub model: Option<String>,
    pub usage: Usage,
    /// Accumulated JSON fragments for tool inputs, keyed by block index.
    tool_input_buffers: Vec<(usize, String)>,
}

impl StreamState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Process a single stream event.
    pub fn process_event(&mut self, event: StreamEvent) {
        match event {
            StreamEvent::MessageStart { message } => {
                self.model = Some(message.model);
                self.usage = message.usage;
                // Initialize content blocks from the message (usually empty)
                for block in message.content {
                    self.content_blocks.push(block);
                }
            }
            StreamEvent::ContentBlockStart {
                index,
                content_block,
            } => {
                // Ensure content_blocks is large enough
                while self.content_blocks.len() <= index {
                    self.content_blocks.push(ContentBlock::Text {
                        text: String::new(),
                    });
                }
                // If this is a tool_use block, start tracking the JSON input buffer
                if let ContentBlock::ToolUse { id, name, .. } = &content_block {
                    self.tool_input_buffers.push((index, String::new()));
                    // We don't add to tool_uses yet; we do that at ContentBlockStop
                    let _ = (id, name); // avoid unused warnings
                }
                self.content_blocks[index] = content_block;
            }
            StreamEvent::ContentBlockDelta { index, delta } => {
                match delta {
                    ContentDelta::TextDelta { text } => {
                        self.text_parts.push(text.clone());
                        // Also update the content block in place
                        if let Some(ContentBlock::Text { text: existing }) =
                            self.content_blocks.get_mut(index)
                        {
                            existing.push_str(&text);
                        }
                    }
                    ContentDelta::ThinkingDelta { thinking } => {
                        if let Some(ContentBlock::Thinking {
                            thinking: existing, ..
                        }) = self.content_blocks.get_mut(index)
                        {
                            existing.push_str(&thinking);
                        }
                    }
                    ContentDelta::InputJsonDelta { partial_json } => {
                        // Accumulate JSON for the tool use at this index
                        for (buf_idx, buf) in &mut self.tool_input_buffers {
                            if *buf_idx == index {
                                buf.push_str(&partial_json);
                                break;
                            }
                        }
                    }
                    ContentDelta::SignatureDelta { signature } => {
                        if let Some(ContentBlock::Thinking {
                            signature: existing,
                            ..
                        }) = self.content_blocks.get_mut(index)
                        {
                            let sig = existing.get_or_insert_with(String::new);
                            sig.push_str(&signature);
                        }
                    }
                }
            }
            StreamEvent::ContentBlockStop { index } => {
                // If this was a tool_use block, finalize it
                if let Some(ContentBlock::ToolUse { id, name, .. }) = self.content_blocks.get(index)
                {
                    let id = id.clone();
                    let name = name.clone();
                    // Find and parse the accumulated JSON
                    let input_json = self
                        .tool_input_buffers
                        .iter()
                        .find(|(buf_idx, _)| *buf_idx == index)
                        .map(|(_, buf)| buf.clone())
                        .unwrap_or_default();

                    let input: serde_json::Value = serde_json::from_str(&input_json)
                        .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

                    // Update the content block with parsed input
                    if let Some(ContentBlock::ToolUse {
                        input: existing_input,
                        ..
                    }) = self.content_blocks.get_mut(index)
                    {
                        *existing_input = input.clone();
                    }

                    self.tool_uses.push(PendingToolCall { id, name, input });
                }
            }
            StreamEvent::MessageDelta { delta, usage } => {
                if let Some(reason) = delta.stop_reason {
                    self.stop_reason = Some(reason);
                }
                if let Some(u) = usage {
                    self.usage.output_tokens = u.output_tokens;
                }
            }
            StreamEvent::MessageStop => {
                // Nothing extra to do
            }
            StreamEvent::Ping => {
                // Ignore
            }
            StreamEvent::Error { error } => {
                // Store as stop reason for error reporting
                self.stop_reason = Some(format!("error: {}", error.message));
            }
        }
    }

    /// Get accumulated text.
    pub fn full_text(&self) -> String {
        self.text_parts.join("")
    }

    /// Check if there are pending tool calls.
    pub fn has_tool_calls(&self) -> bool {
        !self.tool_uses.is_empty()
    }

    /// Extract pending tool calls (consumes them).
    pub fn take_tool_calls(&mut self) -> Vec<PendingToolCall> {
        std::mem::take(&mut self.tool_uses)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cc_api::types::*;

    fn make_message_start(model: &str) -> StreamEvent {
        StreamEvent::MessageStart {
            message: MessagesResponse {
                id: "msg_1".to_string(),
                model: model.to_string(),
                role: Role::Assistant,
                content: vec![],
                stop_reason: None,
                usage: Usage {
                    input_tokens: 100,
                    output_tokens: 0,
                    cache_read_input_tokens: 0,
                    cache_creation_input_tokens: 0,
                },
            },
        }
    }

    #[test]
    fn test_new_state_is_empty() {
        let state = StreamState::new();
        assert!(state.content_blocks.is_empty());
        assert!(state.text_parts.is_empty());
        assert!(state.tool_uses.is_empty());
        assert!(state.stop_reason.is_none());
        assert!(state.model.is_none());
        assert_eq!(state.full_text(), "");
        assert!(!state.has_tool_calls());
    }

    #[test]
    fn test_message_start() {
        let mut state = StreamState::new();
        state.process_event(make_message_start("claude-sonnet-4-20250514"));
        assert_eq!(state.model, Some("claude-sonnet-4-20250514".to_string()));
        assert_eq!(state.usage.input_tokens, 100);
    }

    #[test]
    fn test_text_accumulation() {
        let mut state = StreamState::new();
        state.process_event(make_message_start("model"));
        state.process_event(StreamEvent::ContentBlockStart {
            index: 0,
            content_block: ContentBlock::Text {
                text: String::new(),
            },
        });
        state.process_event(StreamEvent::ContentBlockDelta {
            index: 0,
            delta: ContentDelta::TextDelta {
                text: "Hello".to_string(),
            },
        });
        state.process_event(StreamEvent::ContentBlockDelta {
            index: 0,
            delta: ContentDelta::TextDelta {
                text: ", world!".to_string(),
            },
        });
        state.process_event(StreamEvent::ContentBlockStop { index: 0 });

        assert_eq!(state.full_text(), "Hello, world!");
        assert_eq!(state.content_blocks.len(), 1);
        match &state.content_blocks[0] {
            ContentBlock::Text { text } => assert_eq!(text, "Hello, world!"),
            _ => panic!("expected Text block"),
        }
    }

    #[test]
    fn test_tool_use_accumulation() {
        let mut state = StreamState::new();
        state.process_event(make_message_start("model"));
        state.process_event(StreamEvent::ContentBlockStart {
            index: 0,
            content_block: ContentBlock::ToolUse {
                id: "tu_1".to_string(),
                name: "Bash".to_string(),
                input: serde_json::json!({}),
            },
        });
        state.process_event(StreamEvent::ContentBlockDelta {
            index: 0,
            delta: ContentDelta::InputJsonDelta {
                partial_json: r#"{"command""#.to_string(),
            },
        });
        state.process_event(StreamEvent::ContentBlockDelta {
            index: 0,
            delta: ContentDelta::InputJsonDelta {
                partial_json: r#": "ls"}"#.to_string(),
            },
        });
        state.process_event(StreamEvent::ContentBlockStop { index: 0 });

        assert!(state.has_tool_calls());
        assert_eq!(state.tool_uses.len(), 1);
        assert_eq!(state.tool_uses[0].id, "tu_1");
        assert_eq!(state.tool_uses[0].name, "Bash");
        assert_eq!(state.tool_uses[0].input["command"], "ls");
    }

    #[test]
    fn test_take_tool_calls() {
        let mut state = StreamState::new();
        state.tool_uses.push(PendingToolCall {
            id: "tu_1".to_string(),
            name: "Read".to_string(),
            input: serde_json::json!({}),
        });
        assert!(state.has_tool_calls());

        let calls = state.take_tool_calls();
        assert_eq!(calls.len(), 1);
        assert!(!state.has_tool_calls());
    }

    #[test]
    fn test_message_delta_stop_reason() {
        let mut state = StreamState::new();
        state.process_event(StreamEvent::MessageDelta {
            delta: MessageDeltaBody {
                stop_reason: Some("end_turn".to_string()),
            },
            usage: Some(Usage {
                input_tokens: 0,
                output_tokens: 42,
                cache_read_input_tokens: 0,
                cache_creation_input_tokens: 0,
            }),
        });
        assert_eq!(state.stop_reason, Some("end_turn".to_string()));
        assert_eq!(state.usage.output_tokens, 42);
    }

    #[test]
    fn test_thinking_accumulation() {
        let mut state = StreamState::new();
        state.process_event(make_message_start("model"));
        state.process_event(StreamEvent::ContentBlockStart {
            index: 0,
            content_block: ContentBlock::Thinking {
                thinking: String::new(),
                signature: None,
            },
        });
        state.process_event(StreamEvent::ContentBlockDelta {
            index: 0,
            delta: ContentDelta::ThinkingDelta {
                thinking: "Let me ".to_string(),
            },
        });
        state.process_event(StreamEvent::ContentBlockDelta {
            index: 0,
            delta: ContentDelta::ThinkingDelta {
                thinking: "think...".to_string(),
            },
        });
        state.process_event(StreamEvent::ContentBlockStop { index: 0 });

        match &state.content_blocks[0] {
            ContentBlock::Thinking { thinking, .. } => {
                assert_eq!(thinking, "Let me think...");
            }
            _ => panic!("expected Thinking"),
        }
    }

    #[test]
    fn test_ping_and_stop_are_ignored() {
        let mut state = StreamState::new();
        state.process_event(StreamEvent::Ping);
        state.process_event(StreamEvent::MessageStop);
        // State should still be empty/default
        assert!(state.content_blocks.is_empty());
        assert!(state.stop_reason.is_none());
    }

    #[test]
    fn test_error_event() {
        let mut state = StreamState::new();
        state.process_event(StreamEvent::Error {
            error: ApiErrorBody {
                error_type: "overloaded_error".to_string(),
                message: "Server overloaded".to_string(),
            },
        });
        assert!(
            state
                .stop_reason
                .as_ref()
                .unwrap()
                .contains("Server overloaded")
        );
    }

    #[test]
    fn test_multiple_content_blocks() {
        let mut state = StreamState::new();
        state.process_event(make_message_start("model"));

        // Text block at index 0
        state.process_event(StreamEvent::ContentBlockStart {
            index: 0,
            content_block: ContentBlock::Text {
                text: String::new(),
            },
        });
        state.process_event(StreamEvent::ContentBlockDelta {
            index: 0,
            delta: ContentDelta::TextDelta {
                text: "Before tool".to_string(),
            },
        });
        state.process_event(StreamEvent::ContentBlockStop { index: 0 });

        // Tool use at index 1
        state.process_event(StreamEvent::ContentBlockStart {
            index: 1,
            content_block: ContentBlock::ToolUse {
                id: "tu_1".to_string(),
                name: "Read".to_string(),
                input: serde_json::json!({}),
            },
        });
        state.process_event(StreamEvent::ContentBlockDelta {
            index: 1,
            delta: ContentDelta::InputJsonDelta {
                partial_json: r#"{"path": "/tmp"}"#.to_string(),
            },
        });
        state.process_event(StreamEvent::ContentBlockStop { index: 1 });

        assert_eq!(state.content_blocks.len(), 2);
        assert_eq!(state.full_text(), "Before tool");
        assert!(state.has_tool_calls());
        assert_eq!(state.tool_uses[0].name, "Read");
    }

    #[test]
    fn test_signature_delta() {
        let mut state = StreamState::new();
        state.process_event(make_message_start("model"));
        state.process_event(StreamEvent::ContentBlockStart {
            index: 0,
            content_block: ContentBlock::Thinking {
                thinking: "thought".to_string(),
                signature: None,
            },
        });
        state.process_event(StreamEvent::ContentBlockDelta {
            index: 0,
            delta: ContentDelta::SignatureDelta {
                signature: "sig123".to_string(),
            },
        });
        state.process_event(StreamEvent::ContentBlockStop { index: 0 });

        match &state.content_blocks[0] {
            ContentBlock::Thinking { signature, .. } => {
                assert_eq!(signature.as_deref(), Some("sig123"));
            }
            _ => panic!("expected Thinking"),
        }
    }
}
