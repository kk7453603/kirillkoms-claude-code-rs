use std::path::PathBuf;
use std::sync::Arc;
use tokio_stream::{Stream, StreamExt};

use cc_api::types::{ApiMessage, ContentBlock, Role};
use cc_session::persistence::{TranscriptEntry, append_entry};
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
    /// Idle timeout per turn (None = disabled).
    pub turn_timeout: Option<std::time::Duration>,
    /// Session-level cache for tool support detection.
    pub tools_supported: Option<bool>,
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
            turn_timeout: None,
            tools_supported: None,
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
        self.persist_entry("user_message", serde_json::json!({ "text": user_message }));

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
            tools_supported: self.tools_supported,
            turn_timeout: self.turn_timeout,
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
                QueryEvent::ToolUseStart {
                    ref id,
                    ref name,
                    ref input,
                } => {
                    self.persist_entry(
                        "tool_use",
                        serde_json::json!({ "id": id, "name": name, "input": input }),
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
            return Err(EngineError::Api(
                cc_api::errors::ApiError::ConnectionError { message: err },
            ));
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

        // Check if auto-compaction is needed after this turn
        self.maybe_compact().await;

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
        self.persist_entry("user_message", serde_json::json!({ "text": user_message }));

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
            tools_supported: self.tools_supported,
            turn_timeout: self.turn_timeout,
        };

        query_loop::query(params)
    }

    /// Estimate total token count across all messages.
    pub fn estimate_message_tokens(&self) -> usize {
        self.messages
            .iter()
            .flat_map(|m| m.content.iter())
            .map(|c| match c {
                ContentBlock::Text { text } => text.len() / 4 + 1,
                ContentBlock::ToolUse { input, .. } => input.to_string().len() / 4 + 1,
                ContentBlock::ToolResult { content, .. } => content.to_string().len() / 4 + 1,
                ContentBlock::Thinking { thinking, .. } => thinking.len() / 4 + 1,
                _ => 100, // conservative estimate for images etc.
            })
            .sum()
    }

    /// Check if auto-compaction is needed, and if so, compact old messages.
    pub async fn maybe_compact(&mut self) {
        let total_tokens = self.estimate_message_tokens();
        if !cc_compact::autocompact::should_compact(total_tokens, &Default::default()) {
            return;
        }
        if self.messages.len() < 4 {
            return;
        }
        let keep_count = self.messages.len() / 2;
        let compact_count = self.messages.len() - keep_count;
        let messages_to_compact = &self.messages[..compact_count];

        match cc_compact::compact::compact_messages(
            self.api_client.as_ref(),
            messages_to_compact,
            &self.model,
        )
        .await
        {
            Ok(summary) => {
                let summary_msg = ApiMessage {
                    role: Role::User,
                    content: vec![ContentBlock::Text {
                        text: format!("[Previous conversation summary: {}]", summary),
                    }],
                };
                self.messages = std::iter::once(summary_msg)
                    .chain(self.messages[compact_count..].iter().cloned())
                    .collect();
                tracing::info!("Compacted {} messages into summary", compact_count);
            }
            Err(e) => {
                tracing::warn!("Auto-compaction failed: {}", e);
            }
        }
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
    /// Helper to convert a non-streaming response into a sequence of StreamEvents.
    fn response_to_stream_events(
        response: cc_api::types::MessagesResponse,
    ) -> Vec<cc_api::types::StreamEvent> {
        use cc_api::types::*;
        let mut events = Vec::new();

        events.push(StreamEvent::MessageStart {
            message: MessagesResponse {
                id: response.id.clone(),
                model: response.model.clone(),
                role: response.role,
                content: vec![],
                stop_reason: None,
                usage: response.usage.clone(),
            },
        });

        for (i, block) in response.content.iter().enumerate() {
            events.push(StreamEvent::ContentBlockStart {
                index: i,
                content_block: match block {
                    ContentBlock::Text { .. } => ContentBlock::Text {
                        text: String::new(),
                    },
                    other => other.clone(),
                },
            });
            match block {
                ContentBlock::Text { text } => {
                    events.push(StreamEvent::ContentBlockDelta {
                        index: i,
                        delta: cc_api::types::ContentDelta::TextDelta { text: text.clone() },
                    });
                }
                _ => {}
            }
            events.push(StreamEvent::ContentBlockStop { index: i });
        }

        events.push(StreamEvent::MessageDelta {
            delta: cc_api::types::MessageDeltaBody {
                stop_reason: response.stop_reason.clone(),
            },
            usage: Some(response.usage.clone()),
        });

        events.push(StreamEvent::MessageStop);
        events
    }

    struct MockApiClient {
        responses: std::sync::Mutex<
            Vec<Result<cc_api::types::MessagesResponse, cc_api::errors::ApiError>>,
        >,
    }

    impl MockApiClient {
        fn new(
            responses: Vec<Result<cc_api::types::MessagesResponse, cc_api::errors::ApiError>>,
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
            let mut responses = self.responses.lock().unwrap();
            match responses.pop() {
                Some(Ok(response)) => {
                    let events = response_to_stream_events(response);
                    let stream = futures::stream::iter(events.into_iter().map(Ok));
                    Ok(Box::pin(stream))
                }
                Some(Err(e)) => Err(e),
                None => Err(cc_api::errors::ApiError::InvalidRequest {
                    message: "no more mock responses".to_string(),
                }),
            }
        }

        async fn send_messages(
            &self,
            _request: cc_api::types::MessagesRequest,
        ) -> Result<cc_api::types::MessagesResponse, cc_api::errors::ApiError> {
            let mut responses = self.responses.lock().unwrap();
            responses
                .pop()
                .unwrap_or(Err(cc_api::errors::ApiError::InvalidRequest {
                    message: "no more mock responses".to_string(),
                }))
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

    #[test]
    fn test_new_engine_has_no_execution_context() {
        let client = Arc::new(MockApiClient::new(vec![]));
        let engine = QueryEngine::new(client, "test".to_string());
        assert!(engine.execution_context.is_none());
        assert!(engine.permission_callback.is_none());
        assert!(engine.session_id.is_none());
        assert!(engine.sessions_dir.is_none());
    }

    #[test]
    fn test_set_execution_context() {
        let client = Arc::new(MockApiClient::new(vec![]));
        let mut engine = QueryEngine::new(client, "test".to_string());

        let perm_ctx = cc_permissions::checker::PermissionContext::new(
            cc_permissions::modes::PermissionMode::Default,
        );
        let exec_ctx = crate::tool_execution::ExecutionContext::new(
            perm_ctx,
            std::path::PathBuf::from("/tmp"),
        );
        engine.set_execution_context(exec_ctx);
        assert!(engine.execution_context.is_some());
    }

    #[test]
    fn test_set_permission_callback() {
        let client = Arc::new(MockApiClient::new(vec![]));
        let mut engine = QueryEngine::new(client, "test".to_string());

        engine.set_permission_callback(Arc::new(crate::tool_execution::AutoApproveCallback));
        assert!(engine.permission_callback.is_some());
    }

    #[test]
    fn test_enable_persistence() {
        let client = Arc::new(MockApiClient::new(vec![]));
        let mut engine = QueryEngine::new(client, "test".to_string());

        engine.enable_persistence(
            std::path::PathBuf::from("/tmp/sessions"),
            "test-session-id".to_string(),
        );
        assert_eq!(engine.session_id.as_deref(), Some("test-session-id"));
        assert_eq!(
            engine.sessions_dir.as_deref(),
            Some(std::path::Path::new("/tmp/sessions"))
        );
    }

    #[tokio::test]
    async fn test_submit_persists_transcript() {
        let dir = tempfile::tempdir().unwrap();
        let sessions_dir = dir.path().to_path_buf();
        let session_id = "persist-test";

        let client = Arc::new(MockApiClient::new(vec![Ok(make_text_response("Hello!"))]));
        let mut engine = QueryEngine::new(client, "test".to_string());
        engine.enable_persistence(sessions_dir.clone(), session_id.to_string());

        let result = engine.submit("Hi there").await;
        assert!(result.is_ok());

        // Check that transcript was written
        let transcript_path = cc_session::storage::transcript_path(&sessions_dir, session_id);
        let entries = cc_session::persistence::read_entries(&transcript_path).unwrap();

        // Should have user_message and assistant_message entries
        assert!(entries.len() >= 2);
        assert_eq!(entries[0].entry_type, "user_message");
        assert_eq!(entries[0].data["text"], "Hi there");

        let last = entries.last().unwrap();
        assert_eq!(last.entry_type, "assistant_message");
        assert_eq!(last.data["text"], "Hello!");
    }

    #[tokio::test]
    async fn test_submit_with_execution_context() {
        let client = Arc::new(MockApiClient::new(vec![Ok(make_text_response("response"))]));
        let mut engine = QueryEngine::new(client, "test".to_string());

        // Set up execution context with bypass permissions
        let perm_ctx = cc_permissions::checker::PermissionContext::new(
            cc_permissions::modes::PermissionMode::BypassPermissions,
        );
        let exec_ctx = crate::tool_execution::ExecutionContext::new(
            perm_ctx,
            std::path::PathBuf::from("/tmp"),
        );
        engine.set_execution_context(exec_ctx);
        engine.set_permission_callback(Arc::new(crate::tool_execution::AutoApproveCallback));

        let result = engine.submit("test").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "response");
    }
}
