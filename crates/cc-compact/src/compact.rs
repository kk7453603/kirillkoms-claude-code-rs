use cc_api::client::ApiClient;
use cc_api::types::{ApiMessage, ContentBlock, MessagesRequest, Role, SystemBlock};

/// Compact old messages by summarizing them via the API
pub async fn compact_messages(
    api_client: &dyn ApiClient,
    messages_to_compact: &[ApiMessage],
    model: &str,
) -> Result<String, CompactError> {
    let messages_text = messages_to_compact
        .iter()
        .flat_map(|m| m.content.iter())
        .filter_map(|c| match c {
            ContentBlock::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");

    let prompt = build_compaction_prompt(&messages_text);

    let request = MessagesRequest {
        model: model.to_string(),
        messages: vec![ApiMessage {
            role: Role::User,
            content: vec![ContentBlock::Text { text: prompt }],
        }],
        system: vec![SystemBlock::Text {
            text: "You are a conversation summarizer. Summarize the key points concisely.".into(),
            cache_control: None,
        }],
        max_tokens: Some(2048),
        temperature: None,
        tools: None,
        tool_choice: None,
        thinking: None,
        stream: false,
        metadata: None,
    };

    let response = api_client
        .send_messages(request)
        .await
        .map_err(|e| CompactError::Api(e.to_string()))?;

    let summary = response
        .content
        .iter()
        .filter_map(|c| match c {
            ContentBlock::Text { text } => Some(text.clone()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");

    if summary.is_empty() {
        return Err(CompactError::NoSummary);
    }

    Ok(parse_compaction_response(&summary))
}

#[derive(Debug, thiserror::Error)]
pub enum CompactError {
    #[error("API error: {0}")]
    Api(String),
    #[error("No summary generated")]
    NoSummary,
}

/// Build compaction prompt for the LLM.
///
/// Returns a system-style prompt that asks the LLM to summarize the given
/// conversation messages into a compact form.
pub fn build_compaction_prompt(messages_text: &str) -> String {
    format!(
        "Please provide a concise summary of the following conversation. \
         Preserve key decisions, code changes, file paths, and important context. \
         Omit redundant details and verbose tool output.\n\n\
         --- CONVERSATION ---\n\
         {}\n\
         --- END CONVERSATION ---\n\n\
         Provide a compact summary:",
        messages_text
    )
}

/// Parse compacted summary from LLM response.
///
/// Trims whitespace and returns the cleaned summary text.
pub fn parse_compaction_response(response: &str) -> String {
    response.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockCompactClient {
        response_text: String,
    }

    impl std::fmt::Debug for MockCompactClient {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("MockCompactClient").finish()
        }
    }

    #[async_trait::async_trait]
    impl ApiClient for MockCompactClient {
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
            unimplemented!()
        }

        async fn send_messages(
            &self,
            _request: MessagesRequest,
        ) -> Result<cc_api::types::MessagesResponse, cc_api::errors::ApiError> {
            Ok(cc_api::types::MessagesResponse {
                id: "msg_1".to_string(),
                model: "test".to_string(),
                role: Role::Assistant,
                content: vec![ContentBlock::Text {
                    text: self.response_text.clone(),
                }],
                stop_reason: Some("end_turn".to_string()),
                usage: cc_api::types::Usage::default(),
            })
        }
    }

    struct MockErrorClient;

    impl std::fmt::Debug for MockErrorClient {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("MockErrorClient").finish()
        }
    }

    #[async_trait::async_trait]
    impl ApiClient for MockErrorClient {
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
            unimplemented!()
        }

        async fn send_messages(
            &self,
            _request: MessagesRequest,
        ) -> Result<cc_api::types::MessagesResponse, cc_api::errors::ApiError> {
            Err(cc_api::errors::ApiError::ConnectionError {
                message: "test error".to_string(),
            })
        }
    }

    struct MockEmptyClient;

    impl std::fmt::Debug for MockEmptyClient {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("MockEmptyClient").finish()
        }
    }

    #[async_trait::async_trait]
    impl ApiClient for MockEmptyClient {
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
            unimplemented!()
        }

        async fn send_messages(
            &self,
            _request: MessagesRequest,
        ) -> Result<cc_api::types::MessagesResponse, cc_api::errors::ApiError> {
            Ok(cc_api::types::MessagesResponse {
                id: "msg_1".to_string(),
                model: "test".to_string(),
                role: Role::Assistant,
                content: vec![],
                stop_reason: Some("end_turn".to_string()),
                usage: cc_api::types::Usage::default(),
            })
        }
    }

    #[tokio::test]
    async fn test_compact_messages_success() {
        let client = MockCompactClient {
            response_text: "  Summary of conversation.  ".to_string(),
        };
        let messages = vec![
            ApiMessage {
                role: Role::User,
                content: vec![ContentBlock::Text {
                    text: "Hello".to_string(),
                }],
            },
            ApiMessage {
                role: Role::Assistant,
                content: vec![ContentBlock::Text {
                    text: "Hi there!".to_string(),
                }],
            },
        ];
        let result = compact_messages(&client, &messages, "test-model").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Summary of conversation.");
    }

    #[tokio::test]
    async fn test_compact_messages_api_error() {
        let client = MockErrorClient;
        let messages = vec![ApiMessage {
            role: Role::User,
            content: vec![ContentBlock::Text {
                text: "Hello".to_string(),
            }],
        }];
        let result = compact_messages(&client, &messages, "test-model").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CompactError::Api(_)));
    }

    #[tokio::test]
    async fn test_compact_messages_empty_response() {
        let client = MockEmptyClient;
        let messages = vec![ApiMessage {
            role: Role::User,
            content: vec![ContentBlock::Text {
                text: "Hello".to_string(),
            }],
        }];
        let result = compact_messages(&client, &messages, "test-model").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CompactError::NoSummary));
    }

    #[test]
    fn test_compact_error_display() {
        let err = CompactError::Api("timeout".to_string());
        assert_eq!(err.to_string(), "API error: timeout");

        let err = CompactError::NoSummary;
        assert_eq!(err.to_string(), "No summary generated");
    }

    #[test]
    fn test_build_compaction_prompt_contains_messages() {
        let prompt = build_compaction_prompt("User asked about Rust lifetimes.");
        assert!(prompt.contains("User asked about Rust lifetimes."));
        assert!(prompt.contains("CONVERSATION"));
        assert!(prompt.contains("summary"));
    }

    #[test]
    fn test_build_compaction_prompt_empty_messages() {
        let prompt = build_compaction_prompt("");
        assert!(prompt.contains("--- CONVERSATION ---"));
        assert!(prompt.contains("--- END CONVERSATION ---"));
    }

    #[test]
    fn test_parse_compaction_response_trims() {
        let result = parse_compaction_response("  summary text  \n");
        assert_eq!(result, "summary text");
    }

    #[test]
    fn test_parse_compaction_response_preserves_content() {
        let input = "The user modified main.rs to add error handling.";
        let result = parse_compaction_response(input);
        assert_eq!(result, input);
    }

    #[test]
    fn test_parse_compaction_response_empty() {
        let result = parse_compaction_response("");
        assert_eq!(result, "");
    }

    #[test]
    fn test_parse_compaction_response_multiline() {
        let input = "Line 1\nLine 2\nLine 3";
        let result = parse_compaction_response(input);
        assert_eq!(result, input);
    }
}
