use std::path::PathBuf;

use crate::dispatch::dispatch_hooks;
use crate::events;
use crate::types::{HookEventType, HookOutcome, HooksConfig};

/// Session-scoped hook lifecycle manager.
///
/// Holds the hooks configuration alongside a session ID and working directory,
/// providing convenience methods that build the correct [`HookInput`] and
/// dispatch hooks for each lifecycle event.
pub struct SessionHookManager {
    config: HooksConfig,
    session_id: String,
    cwd: PathBuf,
}

impl SessionHookManager {
    /// Create a new session hook manager.
    pub fn new(config: HooksConfig, session_id: String, cwd: PathBuf) -> Self {
        Self {
            config,
            session_id,
            cwd,
        }
    }

    /// Returns a reference to the underlying hooks configuration.
    pub fn config(&self) -> &HooksConfig {
        &self.config
    }

    /// Returns the session ID.
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Returns the current working directory.
    pub fn cwd(&self) -> &PathBuf {
        &self.cwd
    }

    /// Run SessionStart hooks.
    pub async fn run_session_start(&self) -> HookOutcome {
        let input = events::session_start_input(&self.session_id, &self.cwd.to_string_lossy());
        dispatch_hooks(&self.config, HookEventType::SessionStart, &input, &self.cwd).await
    }

    /// Run PreToolUse hooks.
    pub async fn run_pre_tool_use(
        &self,
        tool_name: &str,
        tool_input: &serde_json::Value,
    ) -> HookOutcome {
        let input = events::pre_tool_use_input(tool_name, tool_input, Some(&self.session_id));
        dispatch_hooks(&self.config, HookEventType::PreToolUse, &input, &self.cwd).await
    }

    /// Run PostToolUse hooks.
    pub async fn run_post_tool_use(
        &self,
        tool_name: &str,
        tool_input: &serde_json::Value,
        tool_output: &serde_json::Value,
    ) -> HookOutcome {
        let input =
            events::post_tool_use_input(tool_name, tool_input, tool_output, Some(&self.session_id));
        dispatch_hooks(&self.config, HookEventType::PostToolUse, &input, &self.cwd).await
    }

    /// Run UserPromptSubmit hooks.
    pub async fn run_user_prompt_submit(&self, prompt: &str) -> HookOutcome {
        let input = events::user_prompt_submit_input(prompt, Some(&self.session_id));
        dispatch_hooks(
            &self.config,
            HookEventType::UserPromptSubmit,
            &input,
            &self.cwd,
        )
        .await
    }

    /// Run FileChanged hooks.
    pub async fn run_file_changed(&self, file_path: &str) -> HookOutcome {
        let input = events::file_changed_input(file_path, Some(&self.session_id));
        dispatch_hooks(&self.config, HookEventType::FileChanged, &input, &self.cwd).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{HookConfig, HookEventType, HookOutcome, HooksConfig};
    use std::path::PathBuf;

    fn empty_manager() -> SessionHookManager {
        SessionHookManager::new(
            HooksConfig::new(),
            "test-session".to_string(),
            PathBuf::from("/tmp"),
        )
    }

    #[test]
    fn test_new_manager() {
        let mgr = empty_manager();
        assert_eq!(mgr.session_id(), "test-session");
        assert_eq!(mgr.cwd(), &PathBuf::from("/tmp"));
        assert!(mgr.config().is_empty());
    }

    #[tokio::test]
    async fn test_session_start_no_hooks() {
        let mgr = empty_manager();
        let outcome = mgr.run_session_start().await;
        match outcome {
            HookOutcome::NoHooks => {}
            other => panic!("Expected NoHooks, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_pre_tool_use_no_hooks() {
        let mgr = empty_manager();
        let outcome = mgr
            .run_pre_tool_use("Bash", &serde_json::json!({"command": "ls"}))
            .await;
        match outcome {
            HookOutcome::NoHooks => {}
            other => panic!("Expected NoHooks, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_post_tool_use_no_hooks() {
        let mgr = empty_manager();
        let outcome = mgr
            .run_post_tool_use(
                "Bash",
                &serde_json::json!({"command": "ls"}),
                &serde_json::json!({"output": "file.txt"}),
            )
            .await;
        match outcome {
            HookOutcome::NoHooks => {}
            other => panic!("Expected NoHooks, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_user_prompt_submit_no_hooks() {
        let mgr = empty_manager();
        let outcome = mgr.run_user_prompt_submit("hello world").await;
        match outcome {
            HookOutcome::NoHooks => {}
            other => panic!("Expected NoHooks, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_file_changed_no_hooks() {
        let mgr = empty_manager();
        let outcome = mgr.run_file_changed("/tmp/test.rs").await;
        match outcome {
            HookOutcome::NoHooks => {}
            other => panic!("Expected NoHooks, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_session_start_with_hook() {
        let mut config = HooksConfig::new();
        config.add(
            HookEventType::SessionStart,
            HookConfig {
                command: r#"echo '{"decision":"approve","message":"started"}'"#.to_string(),
                timeout_ms: 5000,
            },
        );
        let mgr = SessionHookManager::new(config, "s1".to_string(), PathBuf::from("/tmp"));
        let outcome = mgr.run_session_start().await;
        match outcome {
            HookOutcome::Approved { message, .. } => {
                assert_eq!(message.as_deref(), Some("started"));
            }
            other => panic!("Expected Approved, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_pre_tool_use_with_blocking_hook() {
        let mut config = HooksConfig::new();
        config.add(
            HookEventType::PreToolUse,
            HookConfig {
                command: r#"echo '{"decision":"block","reason":"not allowed"}'"#.to_string(),
                timeout_ms: 5000,
            },
        );
        let mgr = SessionHookManager::new(config, "s2".to_string(), PathBuf::from("/tmp"));
        let outcome = mgr
            .run_pre_tool_use("Bash", &serde_json::json!({"command": "rm -rf /"}))
            .await;
        match outcome {
            HookOutcome::Blocked { reason } => {
                assert_eq!(reason, "not allowed");
            }
            other => panic!("Expected Blocked, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_post_tool_use_with_hook() {
        let mut config = HooksConfig::new();
        config.add(
            HookEventType::PostToolUse,
            HookConfig {
                command: "true".to_string(),
                timeout_ms: 5000,
            },
        );
        let mgr = SessionHookManager::new(config, "s3".to_string(), PathBuf::from("/tmp"));
        let outcome = mgr
            .run_post_tool_use(
                "Write",
                &serde_json::json!({"file": "a.txt"}),
                &serde_json::json!({"status": "ok"}),
            )
            .await;
        match outcome {
            HookOutcome::Approved {
                message,
                updated_input,
            } => {
                assert!(message.is_none());
                assert!(updated_input.is_none());
            }
            other => panic!("Expected Approved, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_user_prompt_submit_with_hook() {
        let mut config = HooksConfig::new();
        config.add(
            HookEventType::UserPromptSubmit,
            HookConfig {
                command: r#"echo '{"message":"prompt received"}'"#.to_string(),
                timeout_ms: 5000,
            },
        );
        let mgr = SessionHookManager::new(config, "s4".to_string(), PathBuf::from("/tmp"));
        let outcome = mgr.run_user_prompt_submit("fix the bug").await;
        match outcome {
            HookOutcome::Approved { message, .. } => {
                assert_eq!(message.as_deref(), Some("prompt received"));
            }
            other => panic!("Expected Approved, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_manager_passes_session_id_to_hooks() {
        let mut config = HooksConfig::new();
        // Hook checks that CLAUDE_HOOK_INPUT contains the session id.
        config.add(
            HookEventType::PreToolUse,
            HookConfig {
                command: r#"printf '{"message":"sid-%s"}' "$(printf '%s' "$CLAUDE_HOOK_INPUT" | grep -o 'my-session-42')""#.to_string(),
                timeout_ms: 5000,
            },
        );
        let mgr =
            SessionHookManager::new(config, "my-session-42".to_string(), PathBuf::from("/tmp"));
        let outcome = mgr.run_pre_tool_use("Bash", &serde_json::json!({})).await;
        match outcome {
            HookOutcome::Approved { message, .. } => {
                assert_eq!(message.as_deref(), Some("sid-my-session-42"));
            }
            other => panic!("Expected Approved, got {:?}", other),
        }
    }
}
