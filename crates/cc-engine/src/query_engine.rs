use std::path::PathBuf;
use std::sync::Arc;
use tokio_stream::{Stream, StreamExt};

use cc_api::types::{ApiMessage, ContentBlock, Role};
use cc_session::persistence::{append_entry, TranscriptEntry};
use cc_session::storage::transcript_path;
use cc_tools::registry::ToolRegistry;

use crate::context::SystemContext;
use crate::query_loop::{self, QueryEvent, QueryParams};
use crate::token_budget::TokenBudget;
use crate::tool_execution::{ExecutionContext, PermissionCallback};

/// High-level query engine that manages conversation state.
pub struct QueryEngine {
    pub model: String,
    pub tools: Arc<ToolRegistry>,
    pub api_client: Arc<dyn cc_api::client::ApiClient>,
    pub max_turns: usize,
    pub system_context: SystemContext,
    pub messages: Vec<ApiMessage>,
    pub token_budget: TokenBudget,
    pub execution_context: Option<ExecutionContext>,
    pub permission_callback: Option<Arc<dyn PermissionCallback>>,
    /// Session persistence fields
    pub session_id: Option<String>,
    pub sessions_dir: Option<PathBuf>,
}

impl QueryEngine {
    pub fn new(api_client: Arc<dyn cc_api::client::ApiClient>, model: String) -> Self {
        Self {
            model,
            tools: Arc::new(ToolRegistry::new()),
            api_client,
            max_turns: 10,
            system_context: SystemContext::default(),
            messages: Vec::new(),
            token_budget: TokenBudget::default(),
            execution_context: None,
            permission_callback: None,
            session_id: None,
            sessions_dir: None,
        }
    }

    /// Set the execution context for permission checking and hooks.
    pub fn set_execution_context(&mut self, ctx: ExecutionContext) {
        self.execution_context = Some(ctx);
    }

    /// Set the permission callback for interactive prompts.
    pub fn set_permission_callback(&mut self, cb: Arc<dyn PermissionCallback>) {
        self.permission_callback = Some(cb);
    }

    /// Enable session persistence.
    pub fn enable_persistence(&mut self, sessions_dir: PathBuf, session_id: String) {
        self.session_id = Some(session_id);
        self.sessions_dir = Some(sessions_dir);
    }

    /// Persist a transcript entry if persistence is enabled.
    fn persist_entry(&self, entry_type: &str, data: serde_json::Value) {
        if let (Some(dir), Some(id)) = (&self.sessions_dir, &self.session_id) {
            let path = transcript_path(dir, id);
            let entry = TranscriptEntry {
                timestamp: chrono::Utc::now().to_rfc3339(),
                entry_type: entry_type.to_string(),
                data,
            };
            if let Err(e) = append_entry(&path, &entry) {
                tracing::warn!("Failed to persist transcript entry: {}", e);
            }
        }
    }

    /// Submit a user message and get back the full response text.
    pub async fn submit(&mut self, user_message: &str) -> Result<String, EngineError> {
        // Add user message
        self.messages.push(ApiMessage {
            role: Role::User,
            content: vec![ContentBlock::Text {
                text: user_message.to_string(),
            }],
        });

        // Persist user message
        self.persist_entry(
            "user_message",
            serde_json::json!({ "text": user_message }),
        );

        let system = self.system_context.to_system_blocks();

        let params = QueryParams {
            messages: self.messages.clone(),
            system,
            model: self.model.clone(),
            max_tokens: self.token_budget.max_output_tokens as u32,
            tools: Arc::clone(&self.tools),
            api_client: Arc::clone(&self.api_client),
            max_turns: self.max_turns,
            thinking: None,
            execution_context: self.execution_context.clone(),
            permission_callback: self.permission_callback.clone(),
        };

        let stream = query_loop::query(params);
        let mut stream = std::pin::pin!(stream);

        let mut full_text = String::new();
        let mut got_response = false;
        let mut last_error: Option<String> = None;

        while let Some(event) = stream.next().await {
            match event {
                QueryEvent::TextDelta(text) => {
                    full_text.push_str(&text);
                    got_response = true;
                }
                QueryEvent::ToolUseStart { ref id, ref name } => {
                    self.persist_entry(
                        "tool_use",
                        serde_json::json!({ "id": id, "name": name }),
                    );
                }
                QueryEvent::ToolResult {
                    ref id,
                    ref result,
                    is_error,
                } => {
                    self.persist_entry(
                        "tool_result",
                        serde_json::json!({
                            "id": id,
                            "result": result,
                            "is_error": is_error,
                        }),
                    );
                }
                QueryEvent::Error(ref e) => {
                    last_error = Some(e.clone());
                    break;
                }
                QueryEvent::TurnComplete { ref stop_reason } => {
                    if stop_reason == "max_turns" && !got_response {
                        return Err(EngineError::MaxTurnsExceeded);
                    }
                }
                _ => {}
            }
        }

        if let Some(err) = last_error {
            return Err(EngineError::Api(cc_api::errors::ApiError::ConnectionError {
                message: err,
            }));
        }

        if !got_response {
            return Err(EngineError::NoResponse);
        }

        // Add assistant response to conversation history
        self.messages.push(ApiMessage {
            role: Role::Assistant,
            content: vec![ContentBlock::Text {
                text: full_text.clone(),
            }],
        });

        // Persist assistant message
        self.persist_entry(
            "assistant_message",
            serde_json::json!({ "text": &full_text }),
        );

        Ok(full_text)
    }

    /// Submit and stream events.
    pub fn submit_streaming(
        &mut self,
        user_message: &str,
    ) -> impl Stream<Item = QueryEvent> + Send + '_ {
        // Add user message
        self.messages.push(ApiMessage {
            role: Role::User,
            content: vec![ContentBlock::Text {
                text: user_message.to_string(),
            }],
        });

        // Persist user message
        self.persist_entry(
            "user_message",
            serde_json::json!({ "text": user_message }),
        );

        let system = self.system_context.to_system_blocks();

        let params = QueryParams {
            messages: self.messages.clone(),
            system,
            model: self.model.clone(),
            max_tokens: self.token_budget.max_output_tokens as u32,
            tools: Arc::clone(&self.tools),
            api_client: Arc::clone(&self.api_client),
            max_turns: self.max_turns,
            thinking: None,
            execution_context: self.execution_context.clone(),
            permission_callback: self.permission_callback.clone(),
        };

        query_loop::query(params)
    }

    /// Get conversation history.
    pub fn messages(&self) -> &[ApiMessage] {
        &self.messages
    }

    /// Clear conversation history.
    pub fn clear_messages(&mut self) {
        self.messages.clear();
    }

    /// Set system context.
    pub fn set_system_context(&mut self, ctx: SystemContext) {
        self.system_context = ctx;
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("API error: {0}")]
    Api(#[from] cc_api::errors::ApiError),
    #[error("No response from model")]
    NoResponse,
    #[error("Max turns exceeded")]
    MaxTurnsExceeded,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Reuse mock from query_loop tests
    struct MockApiClient {
        responses: std::sync::Mutex<
            Vec<Result<cc_api::types::MessagesResponse, cc_api::errors::ApiError>>,
        >,
    }

    impl MockApiClient {
        fn new(
            responses: Vec<
                Result<cc_api::types::MessagesResponse, cc_api::errors::ApiError>,
            >,
        ) -> Self {
            let mut responses = responses;
            responses.reverse();
            Self {
                responses: std::sync::Mutex::new(responses),
            }
        }
    }

    impl std::fmt::Debug for MockApiClient {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("MockApiClient").finish()
        }
    }

    #[async_trait::async_trait]
    impl cc_api::client::ApiClient for MockApiClient {
        async fn stream_messages(
            &self,
            _request: cc_api::types::MessagesRequest,
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
            Err(cc_api::errors::ApiError::InvalidRequest {
                message: "not implemented".to_string(),
            })
        }

        async fn send_messages(
            &self,
            _request: cc_api::types::MessagesRequest,
        ) -> Result<cc_api::types::MessagesResponse, cc_api::errors::ApiError> {
            let mut responses = self.responses.lock().unwrap();
            responses.pop().unwrap_or(Err(
                cc_api::errors::ApiError::InvalidRequest {
                    message: "no more mock responses".to_string(),
                },
            ))
        }
    }

    fn make_text_response(text: &str) -> cc_api::types::MessagesResponse {
        cc_api::types::MessagesResponse {
            id: "msg_1".to_string(),
            model: "test".to_string(),
            role: cc_api::types::Role::Assistant,
            content: vec![ContentBlock::Text {
                text: text.to_string(),
            }],
            stop_reason: Some("end_turn".to_string()),
            usage: cc_api::types::Usage::default(),
        }
    }

    #[test]
    fn test_query_engine_construction() {
        let client = Arc::new(MockApiClient::new(vec![]));
        let engine = QueryEngine::new(client, "claude-test".to_string());
        assert_eq!(engine.model, "claude-test");
        assert!(engine.messages().is_empty());
        assert_eq!(engine.max_turns, 10);
    }

    #[test]
    fn test_clear_messages() {
        let client = Arc::new(MockApiClient::new(vec![]));
        let mut engine = QueryEngine::new(client, "test".to_string());
        engine.messages.push(ApiMessage {
            role: Role::User,
            content: vec![ContentBlock::Text {
                text: "hi".to_string(),
            }],
        });
        assert_eq!(engine.messages().len(), 1);
        engine.clear_messages();
        assert!(engine.messages().is_empty());
    }

    #[test]
    fn test_set_system_context() {
        let client = Arc::new(MockApiClient::new(vec![]));
        let mut engine = QueryEngine::new(client, "test".to_string());
        let ctx = SystemContext {
            cwd: "/tmp".to_string(),
            os: "linux".to_string(),
            date: "2026-03-31".to_string(),
            ..Default::default()
        };
        engine.set_system_context(ctx);
        assert_eq!(engine.system_context.cwd, "/tmp");
    }

    #[tokio::test]
    async fn test_submit_simple() {
        let client = Arc::new(MockApiClient::new(vec![Ok(make_text_response(
            "Hello there!",
        ))]));
        let mut engine = QueryEngine::new(client, "test".to_string());
        let result = engine.submit("Hi").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello there!");
        // Should have user + assistant messages
        assert_eq!(engine.messages().len(), 2);
    }

    #[tokio::test]
    async fn test_submit_api_error() {
        let client = Arc::new(MockApiClient::new(vec![Err(
            cc_api::errors::ApiError::ConnectionError {
                message: "timeout".to_string(),
            },
        )]));
        let mut engine = QueryEngine::new(client, "test".to_string());
        let result = engine.submit("Hi").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), EngineError::Api(_)));
    }

    #[tokio::test]
    async fn test_submit_no_response() {
        // Response with empty content
        let response = cc_api::types::MessagesResponse {
            id: "msg_1".to_string(),
            model: "test".to_string(),
            role: cc_api::types::Role::Assistant,
            content: vec![],
            stop_reason: Some("end_turn".to_string()),
            usage: cc_api::types::Usage::default(),
        };
        let client = Arc::new(MockApiClient::new(vec![Ok(response)]));
        let mut engine = QueryEngine::new(client, "test".to_string());
        let result = engine.submit("Hi").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), EngineError::NoResponse));
    }

    #[test]
    fn test_engine_error_display() {
        let err = EngineError::NoResponse;
        assert_eq!(err.to_string(), "No response from model");

        let err = EngineError::MaxTurnsExceeded;
        assert_eq!(err.to_string(), "Max turns exceeded");

        let err = EngineError::Api(cc_api::errors::ApiError::Timeout);
        assert!(err.to_string().contains("Timeout"));
    }

    #[test]
    fn test_default_token_budget() {
        let client = Arc::new(MockApiClient::new(vec![]));
        let engine = QueryEngine::new(client, "test".to_string());
        assert_eq!(engine.token_budget.context_window, 200_000);
        assert_eq!(engine.token_budget.max_output_tokens, 16_384);
    }
}
