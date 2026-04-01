use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;
use tokio_util::sync::CancellationToken;

use crate::auth::ApiConfig;
use crate::client::ApiClient;
use crate::errors::ApiError;
use crate::streaming::parse_sse_line;
use crate::types::{MessagesRequest, MessagesResponse, StreamEvent};

pub struct BedrockApiClient {
    http: reqwest::Client,
    region: String,
    model_id: String,
}

impl BedrockApiClient {
    pub fn new(region: String, model_id: String) -> Result<Self, ApiError> {
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
            region,
            model_id,
        })
    }

    fn endpoint_url(&self) -> String {
        format!(
            "https://bedrock-runtime.{}.amazonaws.com/model/{}/invoke-with-response-stream",
            self.region, self.model_id
        )
    }

    /// Stub for AWS Signature V4 signing. In a real implementation this would
    /// compute HMAC-SHA256 based signatures over the request headers and body.
    /// Returns a set of headers that would normally contain Authorization,
    /// X-Amz-Date, and optionally X-Amz-Security-Token.
    fn sign_request_v4(
        &self,
        _method: &str,
        _url: &str,
        _body: &str,
    ) -> Vec<(String, String)> {
        let now = chrono::Utc::now();
        let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
        let date_stamp = now.format("%Y%m%d").to_string();

        // Stub: in production, these would be real computed values
        let credential = format!(
            "AKIAIOSFODNN7EXAMPLE/{}/{}/bedrock/aws4_request",
            date_stamp, self.region
        );
        let authorization = format!(
            "AWS4-HMAC-SHA256 Credential={}, SignedHeaders=content-type;host;x-amz-date, Signature=stub_signature",
            credential
        );

        vec![
            ("X-Amz-Date".to_string(), amz_date),
            ("Authorization".to_string(), authorization),
        ]
    }

    fn build_request(
        &self,
        request: &MessagesRequest,
    ) -> Result<reqwest::RequestBuilder, ApiError> {
        let url = self.endpoint_url();
        let body = serde_json::to_string(request).map_err(|e| ApiError::InvalidRequest {
            message: format!("Failed to serialize request: {}", e),
        })?;

        let sign_headers = self.sign_request_v4("POST", &url, &body);

        let mut builder = self.http.post(&url).body(body);
        for (name, value) in sign_headers {
            builder = builder.header(&name, &value);
        }

        Ok(builder)
    }
}

#[async_trait]
impl ApiClient for BedrockApiClient {
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

    #[test]
    fn create_bedrock_client() {
        let client = BedrockApiClient::new(
            "us-east-1".to_string(),
            "anthropic.claude-3-sonnet-20240229-v1:0".to_string(),
        );
        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.region, "us-east-1");
        assert_eq!(
            client.model_id,
            "anthropic.claude-3-sonnet-20240229-v1:0"
        );
    }

    #[test]
    fn endpoint_url_format() {
        let client = BedrockApiClient::new(
            "us-west-2".to_string(),
            "anthropic.claude-3-haiku-20240307-v1:0".to_string(),
        )
        .unwrap();
        let url = client.endpoint_url();
        assert_eq!(
            url,
            "https://bedrock-runtime.us-west-2.amazonaws.com/model/anthropic.claude-3-haiku-20240307-v1:0/invoke-with-response-stream"
        );
    }

    #[test]
    fn sign_request_v4_returns_headers() {
        let client = BedrockApiClient::new(
            "us-east-1".to_string(),
            "anthropic.claude-3-sonnet-20240229-v1:0".to_string(),
        )
        .unwrap();
        let headers = client.sign_request_v4("POST", "https://example.com", "{}");
        assert_eq!(headers.len(), 2);
        assert_eq!(headers[0].0, "X-Amz-Date");
        assert_eq!(headers[1].0, "Authorization");
        assert!(headers[1].1.starts_with("AWS4-HMAC-SHA256"));
    }

    #[test]
    fn build_request_serializes() {
        let client = BedrockApiClient::new(
            "eu-west-1".to_string(),
            "anthropic.claude-3-sonnet-20240229-v1:0".to_string(),
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
        // BedrockApiClient will fail to connect to the real endpoint without valid AWS creds
        let client = BedrockApiClient::new(
            "us-east-1".to_string(),
            "anthropic.claude-3-sonnet-20240229-v1:0".to_string(),
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
