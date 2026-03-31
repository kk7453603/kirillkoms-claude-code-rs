use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;
use tokio_util::sync::CancellationToken;

use crate::auth::ApiConfig;
use crate::errors::ApiError;
use crate::providers::direct::DirectApiClient;
use crate::types::{MessagesRequest, MessagesResponse, StreamEvent};

#[async_trait]
pub trait ApiClient: Send + Sync {
    /// Send a streaming messages request.
    /// Returns a stream of `StreamEvent`s that can be consumed incrementally.
    /// Use the `cancel` token to abort the request.
    async fn stream_messages(
        &self,
        request: MessagesRequest,
        cancel: CancellationToken,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent, ApiError>> + Send>>, ApiError>;

    /// Send a non-streaming messages request.
    /// Returns the complete response.
    async fn send_messages(
        &self,
        request: MessagesRequest,
    ) -> Result<MessagesResponse, ApiError>;
}

/// Create an API client based on configuration.
pub async fn create_client(config: ApiConfig) -> Result<Box<dyn ApiClient>, ApiError> {
    let client = DirectApiClient::new(config)?;
    Ok(Box::new(client))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_client_returns_box_dyn() {
        // Verify the function signature compiles - the actual client creation
        // is tested via DirectApiClient tests
        let config = ApiConfig::with_api_key("sk-test".to_string());
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(create_client(config));
        assert!(result.is_ok());
    }
}
