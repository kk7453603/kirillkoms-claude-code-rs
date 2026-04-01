//! OpenAI-compatible API provider.
//!
//! Implements `ApiClient` by translating between Anthropic Messages API types
//! (used internally) and OpenAI Chat Completions API format. This enables
//! the agent to work with any OpenAI-compatible provider: OpenAI, OpenRouter,
//! LM Studio, Ollama, vLLM, Together AI, etc.

use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;
use tokio_util::sync::CancellationToken;

use crate::client::ApiClient;
use crate::errors::ApiError;
use crate::providers::openai_translate;
use crate::providers::openai_types::ChatCompletionResponse;
use crate::streaming::parse_sse_line;
use crate::types::{MessagesRequest, MessagesResponse, StreamEvent};

pub struct OpenAiCompatibleClient {
    http: reqwest::Client,
    base_url: String,
}

impl OpenAiCompatibleClient {
    pub fn new(api_key: String, base_url: String) -> Result<Self, ApiError> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "content-type",
            "application/json"
                .parse()
                .map_err(|_| ApiError::ConnectionError {
                    message: "Invalid header value for content-type".into(),
                })?,
        );
        headers.insert(
            "authorization",
            format!("Bearer {}", api_key)
                .parse()
                .map_err(|_| ApiError::AuthError {
                    message: "Invalid API key format".into(),
                })?,
        );

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(std::time::Duration::from_secs(600))
            .build()
            .map_err(|e| ApiError::ConnectionError {
                message: e.to_string(),
            })?;

        // Normalize base URL: strip trailing slash
        let base_url = base_url.trim_end_matches('/').to_string();

        Ok(Self { http, base_url })
    }

    fn completions_url(&self) -> String {
        format!("{}/v1/chat/completions", self.base_url)
    }
}

#[async_trait]
impl ApiClient for OpenAiCompatibleClient {
    async fn stream_messages(
        &self,
        mut request: MessagesRequest,
        cancel: CancellationToken,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent, ApiError>> + Send>>, ApiError> {
        request.stream = true;

        let openai_request = openai_translate::translate_request(&request);
        let url = self.completions_url();
        let body = serde_json::to_string(&openai_request).map_err(|e| ApiError::InvalidRequest {
            message: format!("Failed to serialize request: {}", e),
        })?;

        let response = tokio::select! {
            result = self.http.post(&url).body(body).send() => {
                result.map_err(|e| {
                    if e.is_timeout() {
                        ApiError::Timeout
                    } else {
                        ApiError::ConnectionError { message: e.to_string() }
                    }
                })?
            }
            _ = cancel.cancelled() => {
                return Err(ApiError::Cancelled);
            }
        };

        let status = response.status().as_u16();
        if status != 200 {
            let body_text = response.text().await.unwrap_or_default();
            return Err(parse_openai_error(status, &body_text));
        }

        let byte_stream = response.bytes_stream();
        let cancel_clone = cancel.clone();

        let stream = async_stream::stream! {
            use futures::StreamExt;

            let mut byte_stream = Box::pin(byte_stream);
            let mut buffer = String::new();
            let mut state = openai_translate::StreamTranslationState::new();

            loop {
                tokio::select! {
                    chunk = byte_stream.next() => {
                        match chunk {
                            Some(Ok(bytes)) => {
                                let text = String::from_utf8_lossy(&bytes);
                                buffer.push_str(&text);

                                while let Some(newline_pos) = buffer.find('\n') {
                                    let line = buffer[..newline_pos].to_string();
                                    buffer = buffer[newline_pos + 1..].to_string();

                                    if let Some(sse_event) = parse_sse_line(&line)
                                        && !sse_event.data.is_empty() && sse_event.data != "[DONE]" {
                                            match serde_json::from_str::<ChatCompletionResponse>(&sse_event.data) {
                                                Ok(openai_chunk) => {
                                                    let events = openai_translate::translate_stream_chunk(
                                                        openai_chunk,
                                                        &mut state,
                                                    );
                                                    for event in events {
                                                        yield Ok(event);
                                                    }
                                                }
                                                Err(e) => {
                                                    tracing::warn!(
                                                        error = %e,
                                                        data = %sse_event.data,
                                                        "Failed to parse OpenAI SSE event"
                                                    );
                                                }
                                            }
                                        }
                                }
                            }
                            Some(Err(e)) => {
                                yield Err(ApiError::StreamError { message: e.to_string() });
                                break;
                            }
                            None => break,
                        }
                    }
                    _ = cancel_clone.cancelled() => {
                        yield Err(ApiError::Cancelled);
                        break;
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }

    async fn send_messages(&self, mut request: MessagesRequest) -> Result<MessagesResponse, ApiError> {
        request.stream = false;

        let openai_request = openai_translate::translate_request(&request);
        let url = self.completions_url();
        let body = serde_json::to_string(&openai_request).map_err(|e| ApiError::InvalidRequest {
            message: format!("Failed to serialize request: {}", e),
        })?;

        let response = self.http.post(&url).body(body).send().await.map_err(|e| {
            if e.is_timeout() {
                ApiError::Timeout
            } else {
                ApiError::ConnectionError {
                    message: e.to_string(),
                }
            }
        })?;

        let status = response.status().as_u16();
        let body_text = response
            .text()
            .await
            .map_err(|e| ApiError::ConnectionError {
                message: format!("Failed to read response body: {}", e),
            })?;

        if status != 200 {
            return Err(parse_openai_error(status, &body_text));
        }

        let openai_response =
            serde_json::from_str::<ChatCompletionResponse>(&body_text).map_err(|e| {
                ApiError::InvalidRequest {
                    message: format!("Failed to parse OpenAI response: {}", e),
                }
            })?;

        Ok(openai_translate::translate_response(openai_response))
    }
}

/// Parse an OpenAI error response into an `ApiError`.
///
/// OpenAI errors use: `{"error":{"message":"...","type":"...","code":"..."}}`
fn parse_openai_error(status: u16, body: &str) -> ApiError {
    #[derive(serde::Deserialize)]
    struct ErrorEnvelope {
        error: ErrorInner,
    }
    #[derive(serde::Deserialize)]
    struct ErrorInner {
        message: String,
    }

    let message = match serde_json::from_str::<ErrorEnvelope>(body) {
        Ok(envelope) => envelope.error.message,
        Err(_) => body.to_string(),
    };

    match status {
        401 => ApiError::AuthError { message },
        429 => ApiError::RateLimited {
            message,
            retry_after_ms: None,
        },
        400 => ApiError::InvalidRequest { message },
        413 => ApiError::PromptTooLong { message },
        s if s >= 500 => ApiError::ServerError {
            status: s,
            message,
        },
        _ => ApiError::InvalidRequest { message },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    #[test]
    fn new_client_with_api_key() {
        let client = OpenAiCompatibleClient::new(
            "sk-test-key".to_string(),
            "https://api.openai.com".to_string(),
        );
        assert!(client.is_ok());
    }

    #[test]
    fn completions_url_default() {
        let client = OpenAiCompatibleClient::new(
            "sk-test".to_string(),
            "https://api.openai.com".to_string(),
        )
        .unwrap();
        assert_eq!(
            client.completions_url(),
            "https://api.openai.com/v1/chat/completions"
        );
    }

    #[test]
    fn completions_url_custom_base() {
        let client = OpenAiCompatibleClient::new(
            "sk-test".to_string(),
            "http://localhost:11434".to_string(),
        )
        .unwrap();
        assert_eq!(
            client.completions_url(),
            "http://localhost:11434/v1/chat/completions"
        );
    }

    #[test]
    fn completions_url_trailing_slash_stripped() {
        let client = OpenAiCompatibleClient::new(
            "sk-test".to_string(),
            "http://localhost:11434/".to_string(),
        )
        .unwrap();
        assert_eq!(
            client.completions_url(),
            "http://localhost:11434/v1/chat/completions"
        );
    }

    #[tokio::test]
    async fn send_messages_connection_error() {
        let client = OpenAiCompatibleClient::new(
            "sk-test".to_string(),
            "http://localhost:1".to_string(), // unreachable
        )
        .unwrap();

        let request = MessagesRequest {
            model: "gpt-4o".to_string(),
            messages: vec![ApiMessage {
                role: Role::User,
                content: vec![ContentBlock::Text {
                    text: "Hello".to_string(),
                }],
            }],
            system: vec![],
            max_tokens: Some(100),
            temperature: None,
            tools: None,
            tool_choice: None,
            thinking: None,
            stream: false,
            metadata: None,
        };

        let result = client.send_messages(request).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ApiError::ConnectionError { .. } | ApiError::Timeout
        ));
    }

    #[tokio::test]
    async fn stream_messages_cancellation() {
        let client = OpenAiCompatibleClient::new(
            "sk-test".to_string(),
            "http://localhost:1".to_string(),
        )
        .unwrap();
        let cancel = CancellationToken::new();
        cancel.cancel();

        let request = MessagesRequest {
            model: "gpt-4o".to_string(),
            messages: vec![],
            system: vec![],
            max_tokens: Some(100),
            temperature: None,
            tools: None,
            tool_choice: None,
            thinking: None,
            stream: true,
            metadata: None,
        };

        let result = client.stream_messages(request, cancel).await;
        assert!(result.is_err());
    }

    #[test]
    fn parse_openai_error_401() {
        let err = parse_openai_error(
            401,
            r#"{"error":{"message":"Invalid API key","type":"invalid_request_error","code":"invalid_api_key"}}"#,
        );
        assert!(matches!(err, ApiError::AuthError { .. }));
        assert!(err.to_string().contains("Invalid API key"));
    }

    #[test]
    fn parse_openai_error_429() {
        let err = parse_openai_error(
            429,
            r#"{"error":{"message":"Rate limit exceeded","type":"rate_limit_error"}}"#,
        );
        assert!(matches!(err, ApiError::RateLimited { .. }));
    }

    #[test]
    fn parse_openai_error_500() {
        let err = parse_openai_error(500, r#"{"error":{"message":"Internal error"}}"#);
        assert!(matches!(err, ApiError::ServerError { status: 500, .. }));
    }

    #[test]
    fn parse_openai_error_non_json() {
        let err = parse_openai_error(502, "Bad Gateway");
        assert!(matches!(err, ApiError::ServerError { status: 502, .. }));
        assert!(err.to_string().contains("Bad Gateway"));
    }
}
