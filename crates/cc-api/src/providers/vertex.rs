use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;
use tokio_util::sync::CancellationToken;

use crate::client::ApiClient;
use crate::errors::ApiError;
use crate::streaming::parse_sse_line;
use crate::types::{MessagesRequest, MessagesResponse, StreamEvent};

pub struct VertexApiClient {
    http: reqwest::Client,
    project_id: String,
    region: String,
    model_id: String,
}

impl VertexApiClient {
    pub fn new(
        project_id: String,
        region: String,
        model_id: String,
    ) -> Result<Self, ApiError> {
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
            project_id,
            region,
            model_id,
        })
    }

    fn endpoint_url(&self) -> String {
        format!(
            "https://{}-aiplatform.googleapis.com/v1/projects/{}/locations/{}/publishers/anthropic/models/{}:streamRawPredict",
            self.region, self.project_id, self.region, self.model_id
        )
    }

    /// Stub for Google Application Default Credentials.
    /// In production, this would read from the well-known credential file,
    /// use the metadata server, or use a service account key to obtain
    /// an OAuth2 access token.
    fn get_access_token(&self) -> Result<String, ApiError> {
        // Check for env var first as a simple override
        if let Ok(token) = std::env::var("GOOGLE_ACCESS_TOKEN") {
            return Ok(token);
        }
        // Stub: return a placeholder token
        Ok("ya29.stub-access-token".to_string())
    }
}

#[async_trait]
impl ApiClient for VertexApiClient {
    async fn stream_messages(
        &self,
        mut request: MessagesRequest,
        cancel: CancellationToken,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent, ApiError>> + Send>>, ApiError> {
        request.stream = true;

        let url = self.endpoint_url();
        let token = self.get_access_token()?;
        let body = serde_json::to_string(&request).map_err(|e| ApiError::InvalidRequest {
            message: format!("Failed to serialize request: {}", e),
        })?;

        let response = tokio::select! {
            result = self.http
                .post(&url)
                .header("Authorization", format!("Bearer {}", token))
                .body(body)
                .send() => {
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

        let url = self.endpoint_url();
        let token = self.get_access_token()?;
        let body = serde_json::to_string(&request).map_err(|e| ApiError::InvalidRequest {
            message: format!("Failed to serialize request: {}", e),
        })?;

        let response = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .body(body)
            .send()
            .await
            .map_err(|e| {
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

    #[test]
    fn create_vertex_client() {
        let client = VertexApiClient::new(
            "my-project-123".to_string(),
            "us-central1".to_string(),
            "claude-3-sonnet@20240229".to_string(),
        );
        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.project_id, "my-project-123");
        assert_eq!(client.region, "us-central1");
        assert_eq!(client.model_id, "claude-3-sonnet@20240229");
    }

    #[test]
    fn endpoint_url_format() {
        let client = VertexApiClient::new(
            "my-project".to_string(),
            "europe-west1".to_string(),
            "claude-3-haiku@20240307".to_string(),
        )
        .unwrap();
        let url = client.endpoint_url();
        assert_eq!(
            url,
            "https://europe-west1-aiplatform.googleapis.com/v1/projects/my-project/locations/europe-west1/publishers/anthropic/models/claude-3-haiku@20240307:streamRawPredict"
        );
    }

    #[test]
    fn endpoint_url_us_central() {
        let client = VertexApiClient::new(
            "test-proj".to_string(),
            "us-central1".to_string(),
            "claude-3-opus@20240229".to_string(),
        )
        .unwrap();
        let url = client.endpoint_url();
        assert!(url.starts_with("https://us-central1-aiplatform.googleapis.com"));
        assert!(url.contains("projects/test-proj"));
        assert!(url.contains("locations/us-central1"));
        assert!(url.ends_with(":streamRawPredict"));
    }

    #[test]
    fn get_access_token_returns_stub() {
        let client = VertexApiClient::new(
            "proj".to_string(),
            "us-east1".to_string(),
            "model".to_string(),
        )
        .unwrap();
        let token = client.get_access_token();
        assert!(token.is_ok());
        // Either env var or stub
        assert!(!token.unwrap().is_empty());
    }

    #[tokio::test]
    async fn send_messages_connection_error() {
        let client = VertexApiClient::new(
            "fake-project".to_string(),
            "us-central1".to_string(),
            "claude-3-sonnet@20240229".to_string(),
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
        // This will fail because we don't have real credentials
        let result = client.send_messages(request).await;
        assert!(result.is_err());
    }
}
