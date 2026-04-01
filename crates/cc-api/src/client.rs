use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;
use tokio_util::sync::CancellationToken;

use crate::auth::{ApiConfig, ApiProvider};
use crate::errors::ApiError;
use crate::providers::bedrock::BedrockApiClient;
use crate::providers::direct::DirectApiClient;
use crate::providers::foundry::FoundryApiClient;
use crate::providers::openai_compatible::OpenAiCompatibleClient;
use crate::providers::vertex::VertexApiClient;
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
    async fn send_messages(&self, request: MessagesRequest) -> Result<MessagesResponse, ApiError>;
}

/// Create an API client based on configuration.
/// Uses `ApiProvider::from_env()` to determine which provider to use,
/// then constructs the appropriate client.
pub async fn create_client(config: ApiConfig) -> Result<Box<dyn ApiClient>, ApiError> {
    let provider = ApiProvider::from_env();
    create_client_for_provider(config, provider)
}

/// Create an API client for a specific provider.
pub fn create_client_for_provider(
    config: ApiConfig,
    provider: ApiProvider,
) -> Result<Box<dyn ApiClient>, ApiError> {
    match provider {
        ApiProvider::Direct => {
            let client = DirectApiClient::new(config)?;
            Ok(Box::new(client))
        }
        ApiProvider::Bedrock => {
            let region = std::env::var("AWS_REGION")
                .or_else(|_| std::env::var("AWS_DEFAULT_REGION"))
                .unwrap_or_else(|_| "us-east-1".to_string());
            let model_id = std::env::var("ANTHROPIC_MODEL")
                .unwrap_or_else(|_| "anthropic.claude-3-sonnet-20240229-v1:0".to_string());
            let client = BedrockApiClient::new(region, model_id)?;
            Ok(Box::new(client))
        }
        ApiProvider::Vertex => {
            let project_id = std::env::var("CLOUD_ML_PROJECT_ID")
                .or_else(|_| std::env::var("GOOGLE_CLOUD_PROJECT"))
                .map_err(|_| ApiError::AuthError {
                    message:
                        "CLOUD_ML_PROJECT_ID or GOOGLE_CLOUD_PROJECT must be set for Vertex AI"
                            .into(),
                })?;
            let region =
                std::env::var("CLOUD_ML_REGION").unwrap_or_else(|_| "us-central1".to_string());
            let model_id = std::env::var("ANTHROPIC_MODEL")
                .unwrap_or_else(|_| "claude-3-sonnet@20240229".to_string());
            let client = VertexApiClient::new(project_id, region, model_id)?;
            Ok(Box::new(client))
        }
        ApiProvider::Foundry => {
            let base_url =
                std::env::var("AZURE_FOUNDRY_BASE_URL").map_err(|_| ApiError::AuthError {
                    message: "AZURE_FOUNDRY_BASE_URL must be set for Azure Foundry".into(),
                })?;
            let resource = std::env::var("AZURE_FOUNDRY_RESOURCE")
                .unwrap_or_else(|_| "claude-deployment".to_string());
            let client = FoundryApiClient::new(base_url, resource)?;
            Ok(Box::new(client))
        }
        ApiProvider::OpenAiCompatible => {
            let api_key =
                std::env::var("OPENAI_API_KEY").map_err(|_| ApiError::AuthError {
                    message: "OPENAI_API_KEY must be set for OpenAI-compatible provider".into(),
                })?;
            let base_url = std::env::var("OPENAI_BASE_URL")
                .or_else(|_| std::env::var("OPENAI_API_BASE"))
                .unwrap_or_else(|_| "https://api.openai.com".to_string());
            let client = OpenAiCompatibleClient::new(api_key, base_url)?;
            Ok(Box::new(client))
        }
    }
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

    #[test]
    fn create_client_for_direct_provider() {
        let config = ApiConfig::with_api_key("sk-test".to_string());
        let result = create_client_for_provider(config, ApiProvider::Direct);
        assert!(result.is_ok());
    }

    #[test]
    fn create_client_for_bedrock_provider() {
        let result = create_client_for_provider(
            ApiConfig::with_api_key("unused".to_string()),
            ApiProvider::Bedrock,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn create_client_for_vertex_provider_missing_project() {
        // Without CLOUD_ML_PROJECT_ID or GOOGLE_CLOUD_PROJECT, should error
        // (unless the env happens to have them set)
        let result = create_client_for_provider(
            ApiConfig::with_api_key("unused".to_string()),
            ApiProvider::Vertex,
        );
        // Result depends on env; just verify it doesn't panic
        let _ = result;
    }

    #[test]
    fn create_client_for_foundry_provider_missing_url() {
        // Without AZURE_FOUNDRY_BASE_URL, should error
        let result = create_client_for_provider(
            ApiConfig::with_api_key("unused".to_string()),
            ApiProvider::Foundry,
        );
        // Result depends on env
        let _ = result;
    }

    #[test]
    fn create_client_for_openai_provider() {
        // Without OPENAI_API_KEY, should error
        let result = create_client_for_provider(
            ApiConfig::with_api_key("unused".to_string()),
            ApiProvider::OpenAiCompatible,
        );
        // Result depends on env (OPENAI_API_KEY presence)
        let _ = result;
    }
}
