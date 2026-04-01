use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// All lifecycle event types that hooks can respond to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum HookEventType {
    PreToolUse,
    PostToolUse,
    PostToolUseFailure,
    UserPromptSubmit,
    SessionStart,
    Setup,
    SubagentStart,
    FileChanged,
    CwdChanged,
    WorktreeCreate,
    PermissionRequest,
    PermissionDenied,
    Notification,
}

impl HookEventType {
    /// Returns a slice of all possible event types.
    pub fn all() -> &'static [HookEventType] {
        &[
            Self::PreToolUse,
            Self::PostToolUse,
            Self::PostToolUseFailure,
            Self::UserPromptSubmit,
            Self::SessionStart,
            Self::Setup,
            Self::SubagentStart,
            Self::FileChanged,
            Self::CwdChanged,
            Self::WorktreeCreate,
            Self::PermissionRequest,
            Self::PermissionDenied,
            Self::Notification,
        ]
    }

    /// Returns the string representation of this event type.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PreToolUse => "PreToolUse",
            Self::PostToolUse => "PostToolUse",
            Self::PostToolUseFailure => "PostToolUseFailure",
            Self::UserPromptSubmit => "UserPromptSubmit",
            Self::SessionStart => "SessionStart",
            Self::Setup => "Setup",
            Self::SubagentStart => "SubagentStart",
            Self::FileChanged => "FileChanged",
            Self::CwdChanged => "CwdChanged",
            Self::WorktreeCreate => "WorktreeCreate",
            Self::PermissionRequest => "PermissionRequest",
            Self::PermissionDenied => "PermissionDenied",
            Self::Notification => "Notification",
        }
    }
}

impl fmt::Display for HookEventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Configuration for a single hook command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookConfig {
    /// The shell command to execute.
    pub command: String,
    /// Timeout in milliseconds (default: 60000).
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

fn default_timeout() -> u64 {
    60_000
}

/// Input passed to a hook via env var and stdin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookInput {
    pub hook_event: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_input: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_output: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
}

/// Structured JSON output from a hook's stdout.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HookJsonOutput {
    #[serde(default)]
    pub decision: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(rename = "suppressOutput", default)]
    pub suppress_output: Option<bool>,
    #[serde(rename = "updatedInput", default)]
    pub updated_input: Option<serde_json::Value>,
    #[serde(rename = "updatedOutput", default)]
    pub updated_output: Option<serde_json::Value>,
}

/// The result of executing one or more hooks.
#[derive(Debug, Clone)]
pub enum HookOutcome {
    /// Hook approved / succeeded.
    Approved {
        message: Option<String>,
        updated_input: Option<serde_json::Value>,
    },
    /// Hook blocked the action.
    Blocked { reason: String },
    /// Hook errored during execution.
    Error { message: String },
    /// Hook exceeded its timeout.
    TimedOut { timeout_ms: u64 },
    /// No hooks configured for this event.
    NoHooks,
}

/// Top-level hooks configuration mapping event types to hook commands.
#[derive(Debug, Clone)]
pub struct HooksConfig {
    pub hooks: HashMap<HookEventType, Vec<HookConfig>>,
}

impl Default for HooksConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl HooksConfig {
    /// Create an empty hooks configuration.
    pub fn new() -> Self {
        Self {
            hooks: HashMap::new(),
        }
    }

    /// Register a hook for the given event type.
    pub fn add(&mut self, event: HookEventType, config: HookConfig) {
        self.hooks.entry(event).or_default().push(config);
    }

    /// Get all hooks registered for the given event type.
    pub fn get(&self, event: &HookEventType) -> &[HookConfig] {
        match self.hooks.get(event) {
            Some(configs) => configs,
            None => &[],
        }
    }

    /// Returns true if no hooks are configured.
    pub fn is_empty(&self) -> bool {
        self.hooks.is_empty() || self.hooks.values().all(|v| v.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_event_type_serialization() {
        let event = HookEventType::PreToolUse;
        let json = serde_json::to_string(&event).unwrap();
        assert_eq!(json, "\"PreToolUse\"");

        let event = HookEventType::PostToolUseFailure;
        let json = serde_json::to_string(&event).unwrap();
        assert_eq!(json, "\"PostToolUseFailure\"");
    }

    #[test]
    fn test_hook_event_type_deserialization() {
        let event: HookEventType = serde_json::from_str("\"SessionStart\"").unwrap();
        assert_eq!(event, HookEventType::SessionStart);

        let event: HookEventType = serde_json::from_str("\"FileChanged\"").unwrap();
        assert_eq!(event, HookEventType::FileChanged);
    }

    #[test]
    fn test_hook_event_type_all() {
        let all = HookEventType::all();
        assert_eq!(all.len(), 13);
        assert!(all.contains(&HookEventType::PreToolUse));
        assert!(all.contains(&HookEventType::Notification));
    }

    #[test]
    fn test_hook_event_type_as_str() {
        assert_eq!(HookEventType::PreToolUse.as_str(), "PreToolUse");
        assert_eq!(HookEventType::CwdChanged.as_str(), "CwdChanged");
        assert_eq!(HookEventType::Notification.as_str(), "Notification");
    }

    #[test]
    fn test_hook_event_type_display() {
        assert_eq!(format!("{}", HookEventType::PreToolUse), "PreToolUse");
    }

    #[test]
    fn test_hook_config_deserialization_with_defaults() {
        let json = r#"{"command": "echo hello"}"#;
        let config: HookConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.command, "echo hello");
        assert_eq!(config.timeout_ms, 60_000);
    }

    #[test]
    fn test_hook_config_deserialization_with_timeout() {
        let json = r#"{"command": "my-hook.sh", "timeout_ms": 5000}"#;
        let config: HookConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.command, "my-hook.sh");
        assert_eq!(config.timeout_ms, 5000);
    }

    #[test]
    fn test_hook_input_serialization() {
        let input = HookInput {
            hook_event: "PreToolUse".to_string(),
            tool_name: Some("Bash".to_string()),
            tool_input: Some(serde_json::json!({"command": "ls"})),
            tool_output: None,
            session_id: Some("sess-123".to_string()),
            cwd: Some("/tmp".to_string()),
        };
        let json = serde_json::to_value(&input).unwrap();
        assert_eq!(json["hook_event"], "PreToolUse");
        assert_eq!(json["tool_name"], "Bash");
        // tool_output should be absent (skip_serializing_if)
        assert!(json.get("tool_output").is_none());
    }

    #[test]
    fn test_hook_json_output_deserialization() {
        let json = r#"{
            "decision": "approve",
            "message": "looks good",
            "updatedInput": {"key": "value"}
        }"#;
        let output: HookJsonOutput = serde_json::from_str(json).unwrap();
        assert_eq!(output.decision.as_deref(), Some("approve"));
        assert_eq!(output.message.as_deref(), Some("looks good"));
        assert!(output.updated_input.is_some());
        assert!(output.reason.is_none());
        assert!(output.suppress_output.is_none());
    }

    #[test]
    fn test_hook_json_output_default() {
        let output = HookJsonOutput::default();
        assert!(output.decision.is_none());
        assert!(output.reason.is_none());
        assert!(output.message.is_none());
        assert!(output.suppress_output.is_none());
        assert!(output.updated_input.is_none());
        assert!(output.updated_output.is_none());
    }

    #[test]
    fn test_hooks_config_new_is_empty() {
        let config = HooksConfig::new();
        assert!(config.is_empty());
    }

    #[test]
    fn test_hooks_config_default_is_empty() {
        let config = HooksConfig::default();
        assert!(config.is_empty());
    }

    #[test]
    fn test_hooks_config_add_and_get() {
        let mut config = HooksConfig::new();
        config.add(
            HookEventType::PreToolUse,
            HookConfig {
                command: "echo pre".to_string(),
                timeout_ms: 5000,
            },
        );
        config.add(
            HookEventType::PreToolUse,
            HookConfig {
                command: "echo pre2".to_string(),
                timeout_ms: 3000,
            },
        );
        config.add(
            HookEventType::SessionStart,
            HookConfig {
                command: "echo start".to_string(),
                timeout_ms: 10000,
            },
        );

        assert!(!config.is_empty());

        let pre_hooks = config.get(&HookEventType::PreToolUse);
        assert_eq!(pre_hooks.len(), 2);
        assert_eq!(pre_hooks[0].command, "echo pre");
        assert_eq!(pre_hooks[1].command, "echo pre2");

        let start_hooks = config.get(&HookEventType::SessionStart);
        assert_eq!(start_hooks.len(), 1);

        // Event with no hooks returns empty slice.
        let post_hooks = config.get(&HookEventType::PostToolUse);
        assert!(post_hooks.is_empty());
    }
}
