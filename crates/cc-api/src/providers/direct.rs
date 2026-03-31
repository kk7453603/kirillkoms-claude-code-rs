use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;
use tokio_util::sync::CancellationToken;

use crate::auth::{ApiConfig, AuthMethod};
use crate::client::ApiClient;
use crate::errors::ApiError;
use crate::streaming::parse_sse_line;
use crate::types::{MessagesRequest, MessagesResponse, StreamEvent};

pub struct DirectApiClient {
    http: reqwest::Client,
    config: ApiConfig,
}

impl DirectApiClient {
    pub fn new(config: ApiConfig) -> Result<Self, ApiError> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "anthropic-version",
            "2023-06-01"
                .parse()
                .map_err(|_| ApiError::ConnectionError {
                    message: "Invalid header value for anthropic-version".into(),
                })?,
        );
        headers.insert(
            "content-type",
            "application/json"
                .parse()
                .map_err(|_| ApiError::ConnectionError {
                    message: "Invalid header value for content-type".into(),
                })?,
        );

        match &config.auth {
            AuthMethod::ApiKey(key) => {
                headers.insert(
                    "x-api-key",
                    key.parse().map_err(|_| ApiError::AuthError {
                        message: "Invalid API key format".into(),
                    })?,
                );
            }
            AuthMethod::OAuthToken(token) => {
                headers.insert(
                    "authorization",
                    format!("Bearer {}", token)
                        .parse()
                        .map_err(|_| ApiError::AuthError {
                            message: "Invalid token format".into(),
                        })?,
                );
            }
        }

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| ApiError::ConnectionError {
                message: e.to_string(),
            })?;

        Ok(Self { http, config })
    }

    fn messages_url(&self) -> String {
        format!("{}/v1/messages", self.config.base_url)
    }
}

#[async_trait]
impl ApiClient for DirectApiClient {
    async fn stream_messages(
        &self,
        mut request: MessagesRequest,
        cancel: CancellationToken,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent, ApiError>> + Send>>, ApiError> {
        request.stream = true;

        let url = self.messages_url();
        let body = serde_json::to_string(&request).map_err(|e| ApiError::InvalidRequest {
            message: format!("Failed to serialize request: {}", e),
        })?;

        let response = tokio::select! {
            result = self.http.post(&url).body(body).send() => {
                result.map_err(|e| {
                    if e.is_timeout() {
                        ApiError::Timeout
                    } else if e.is_connect() {
                        ApiError::ConnectionError { message: e.to_string() }
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
            return Err(ApiError::from_status(status, &body_text));
        }

        let byte_stream = response.bytes_stream();
        let cancel_clone = cancel.clone();

        let stream = async_stream::stream! {
            use futures::StreamExt;

            let mut byte_stream = Box::pin(byte_stream);
            let mut buffer = String::new();

            loop {
                tokio::select! {
                    chunk = byte_stream.next() => {
                        match chunk {
                            Some(Ok(bytes)) => {
                                let text = String::from_utf8_lossy(&bytes);
                                buffer.push_str(&text);

                                // Process complete lines
                                while let Some(newline_pos) = buffer.find('\n') {
                                    let line = buffer[..newline_pos].to_string();
                                    buffer = buffer[newline_pos + 1..].to_string();

                                    if let Some(sse_event) = parse_sse_line(&line) {
                                        if !sse_event.data.is_empty() && sse_event.data != "[DONE]" {
                                            match serde_json::from_str::<StreamEvent>(&sse_event.data) {
                                                Ok(event) => yield Ok(event),
                                                Err(e) => {
                                                    tracing::warn!(
                                                        error = %e,
                                                        data = %sse_event.data,
                                                        "Failed to parse SSE event"
                                                    );
                                                    // Skip unparseable events rather than failing the stream
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Some(Err(e)) => {
                                yield Err(ApiError::StreamError { message: e.to_string() });
                                break;
                            }
                            None => {
                                // Stream ended
                                break;
                            }
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

    async fn send_messages(
        &self,
        mut request: MessagesRequest,
    ) -> Result<MessagesResponse, ApiError> {
        request.stream = false;

        let url = self.messages_url();
        let body = serde_json::to_string(&request).map_err(|e| ApiError::InvalidRequest {
            message: format!("Failed to serialize request: {}", e),
        })?;

        let response = self
            .http
            .post(&url)
            .body(body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    ApiError::Timeout
                } else if e.is_connect() {
                    ApiError::ConnectionError {
                        message: e.to_string(),
                    }
                } else {
                    ApiError::ConnectionError {
                        message: e.to_string(),
                    }
                }
            })?;

        let status = response.status().as_u16();
        let body_text = response.text().await.map_err(|e| ApiError::ConnectionError {
            message: format!("Failed to read response body: {}", e),
        })?;

        if status != 200 {
            return Err(ApiError::from_status(status, &body_text));
        }

        serde_json::from_str::<MessagesResponse>(&body_text).map_err(|e| {
            ApiError::InvalidRequest {
                message: format!("Failed to parse response: {}", e),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    #[test]
    fn new_with_api_key() {
        let config = ApiConfig::with_api_key("sk-test-key".to_string());
        let client = DirectApiClient::new(config);
        assert!(client.is_ok());
    }

    #[test]
    fn new_with_oauth_token() {
        let config = ApiConfig {
            auth: AuthMethod::OAuthToken("oauth-token".to_string()),
            base_url: "https://api.anthropic.com".to_string(),
            max_retries: 3,
            timeout_secs: 600,
        };
        let client = DirectApiClient::new(config);
        assert!(client.is_ok());
    }

    #[test]
    fn messages_url_default() {
        let config = ApiConfig::with_api_key("sk-test".to_string());
        let client = DirectApiClient::new(config).unwrap();
        assert_eq!(
            client.messages_url(),
            "https://api.anthropic.com/v1/messages"
        );
    }

    #[test]
    fn messages_url_custom_base() {
        let config = ApiConfig {
            auth: AuthMethod::ApiKey("sk-test".to_string()),
            base_url: "https://custom.api.com".to_string(),
            max_retries: 3,
            timeout_secs: 600,
        };
        let client = DirectApiClient::new(config).unwrap();
        assert_eq!(
            client.messages_url(),
            "https://custom.api.com/v1/messages"
        );
    }

    #[tokio::test]
    async fn send_messages_connection_error() {
        let config = ApiConfig {
            auth: AuthMethod::ApiKey("sk-test".to_string()),
            base_url: "http://localhost:1".to_string(), // unreachable port
            max_retries: 0,
            timeout_secs: 2,
        };
        let client = DirectApiClient::new(config).unwrap();

        let request = MessagesRequest {
            model: "claude-sonnet-4-20250514".to_string(),
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
        let err = result.unwrap_err();
        assert!(
            matches!(err, ApiError::ConnectionError { .. } | ApiError::Timeout),
            "Expected ConnectionError or Timeout, got: {:?}",
            err
        );
    }

    #[tokio::test]
    async fn stream_messages_cancellation() {
        let config = ApiConfig {
            auth: AuthMethod::ApiKey("sk-test".to_string()),
            base_url: "http://localhost:1".to_string(),
            max_retries: 0,
            timeout_secs: 30,
        };
        let client = DirectApiClient::new(config).unwrap();
        let cancel = CancellationToken::new();
        cancel.cancel(); // Cancel immediately

        let request = MessagesRequest {
            model: "claude-sonnet-4-20250514".to_string(),
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
        // Either Cancelled (if cancel won the race) or ConnectionError (if connect failed first)
        assert!(result.is_err());
    }
}
