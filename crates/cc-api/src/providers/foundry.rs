use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;
use tokio_util::sync::CancellationToken;

use crate::client::ApiClient;
use crate::errors::ApiError;
use crate::streaming::parse_sse_line;
use crate::types::{MessagesRequest, MessagesResponse, StreamEvent};

pub struct FoundryApiClient {
    http: reqwest::Client,
    base_url: String,
    resource: String,
}

impl FoundryApiClient {
    pub fn new(base_url: String, resource: String) -> Result<Self, ApiError> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "content-type",
            "application/json"
                .parse()
                .map_err(|_| ApiError::ConnectionError {
                    message: "Invalid header value for content-type".into(),
                })?,
        );

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(std::time::Duration::from_secs(600))
            .build()
            .map_err(|e| ApiError::ConnectionError {
                message: e.to_string(),
            })?;

        Ok(Self {
            http,
            base_url,
            resource,
        })
    }

    fn endpoint_url(&self) -> String {
        format!(
            "{}/openai/deployments/{}/messages",
            self.base_url, self.resource
        )
    }

    /// Stub for Azure AD token retrieval.
    /// In production, this would use the Azure Identity SDK to obtain
    /// a token via managed identity, service principal, or CLI credentials.
    fn get_azure_ad_token(&self) -> Result<String, ApiError> {
        if let Ok(token) = std::env::var("AZURE_AD_TOKEN") {
            return Ok(token);
        }
        // Stub: return a placeholder token
        Ok("eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzI1NiJ9.stub-azure-ad-token".to_string())
    }

    fn build_request(
        &self,
        request: &MessagesRequest,
    ) -> Result<reqwest::RequestBuilder, ApiError> {
        let url = self.endpoint_url();
        let token = self.get_azure_ad_token()?;
        let body = serde_json::to_string(request).map_err(|e| ApiError::InvalidRequest {
            message: format!("Failed to serialize request: {}", e),
        })?;

        let builder = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("api-version", "2024-06-01")
            .body(body);

        Ok(builder)
    }
}

#[async_trait]
impl ApiClient for FoundryApiClient {
    async fn stream_messages(
        &self,
        mut request: MessagesRequest,
        cancel: CancellationToken,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent, ApiError>> + Send>>, ApiError> {
        request.stream = true;

        let req_builder = self.build_request(&request)?;

        let response = tokio::select! {
            result = req_builder.send() => {
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

                                while let Some(newline_pos) = buffer.find('\n') {
                                    let line = buffer[..newline_pos].to_string();
                                    buffer = buffer[newline_pos + 1..].to_string();

                                    if let Some(sse_event) = parse_sse_line(&line)
                                        && !sse_event.data.is_empty() && sse_event.data != "[DONE]" {
                                            match serde_json::from_str::<StreamEvent>(&sse_event.data) {
                                                Ok(event) => yield Ok(event),
                                                Err(e) => {
                                                    tracing::warn!(
                                                        error = %e,
                                                        data = %sse_event.data,
                                                        "Failed to parse SSE event"
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

    async fn send_messages(
        &self,
        mut request: MessagesRequest,
    ) -> Result<MessagesResponse, ApiError> {
        request.stream = false;

        let req_builder = self.build_request(&request)?;

        let response = req_builder.send().await.map_err(|e| {
            if e.is_timeout() {
                ApiError::Timeout
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
    fn create_foundry_client() {
        let client = FoundryApiClient::new(
            "https://my-resource.openai.azure.com".to_string(),
            "claude-deployment".to_string(),
        );
        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.base_url, "https://my-resource.openai.azure.com");
        assert_eq!(client.resource, "claude-deployment");
    }

    #[test]
    fn endpoint_url_format() {
        let client = FoundryApiClient::new(
            "https://my-resource.openai.azure.com".to_string(),
            "claude-3-sonnet".to_string(),
        )
        .unwrap();
        let url = client.endpoint_url();
        assert_eq!(
            url,
            "https://my-resource.openai.azure.com/openai/deployments/claude-3-sonnet/messages"
        );
    }

    #[test]
    fn get_azure_ad_token_returns_stub() {
        let client = FoundryApiClient::new(
            "https://example.azure.com".to_string(),
            "deployment".to_string(),
        )
        .unwrap();
        let token = client.get_azure_ad_token();
        assert!(token.is_ok());
        assert!(!token.unwrap().is_empty());
    }

    #[test]
    fn build_request_serializes() {
        let client = FoundryApiClient::new(
            "https://example.azure.com".to_string(),
            "deployment".to_string(),
        )
        .unwrap();
        let request = MessagesRequest {
            model: "claude-sonnet-4-20250514".to_string(),
            messages: vec![],
            system: vec![],
            max_tokens: Some(100),
            temperature: None,
            tools: None,
            tool_choice: None,
            thinking: None,
            stream: false,
            metadata: None,
        };
        let result = client.build_request(&request);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn send_messages_connection_error() {
        let client = FoundryApiClient::new(
            "http://localhost:1".to_string(),
            "deployment".to_string(),
        )
        .unwrap();
        let request = MessagesRequest {
            model: "claude-sonnet-4-20250514".to_string(),
            messages: vec![],
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
    }
}
