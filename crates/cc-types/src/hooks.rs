use serde::{Deserialize, Serialize};

use crate::permissions::PermissionBehavior;

/// Events that can trigger hooks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookEvent {
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

/// Input data passed to a hook.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookInput {
    pub event: HookEvent,
    pub tool_name: Option<String>,
    pub tool_input: Option<serde_json::Value>,
    pub tool_output: Option<serde_json::Value>,
    pub session_id: Option<String>,
}

/// Output returned from a hook execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookOutput {
    pub decision: Option<PermissionBehavior>,
    pub reason: Option<String>,
    pub updated_input: Option<serde_json::Value>,
    pub updated_output: Option<serde_json::Value>,
}

/// Result of running a hook.
#[derive(Debug, Clone)]
pub enum HookResult {
    Success { output: Option<HookOutput> },
    Blocking { message: String },
    Error { message: String },
    Cancelled,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hook_event_serde_roundtrip() {
        let events = [
            (HookEvent::PreToolUse, "\"pre_tool_use\""),
            (HookEvent::PostToolUse, "\"post_tool_use\""),
            (HookEvent::PostToolUseFailure, "\"post_tool_use_failure\""),
            (HookEvent::UserPromptSubmit, "\"user_prompt_submit\""),
            (HookEvent::SessionStart, "\"session_start\""),
            (HookEvent::Setup, "\"setup\""),
            (HookEvent::SubagentStart, "\"subagent_start\""),
            (HookEvent::FileChanged, "\"file_changed\""),
            (HookEvent::CwdChanged, "\"cwd_changed\""),
            (HookEvent::WorktreeCreate, "\"worktree_create\""),
            (HookEvent::PermissionRequest, "\"permission_request\""),
            (HookEvent::PermissionDenied, "\"permission_denied\""),
            (HookEvent::Notification, "\"notification\""),
        ];
        for (event, expected_json) in events {
            let json = serde_json::to_string(&event).unwrap();
            assert_eq!(json, expected_json);
            let back: HookEvent = serde_json::from_str(&json).unwrap();
            assert_eq!(event, back);
        }
    }

    #[test]
    fn hook_event_hash_in_set() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(HookEvent::PreToolUse);
        set.insert(HookEvent::PostToolUse);
        set.insert(HookEvent::PreToolUse); // duplicate
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn hook_input_serde_roundtrip() {
        let input = HookInput {
            event: HookEvent::PreToolUse,
            tool_name: Some("bash".to_string()),
            tool_input: Some(serde_json::json!({"command": "ls"})),
            tool_output: None,
            session_id: Some("sess-123".to_string()),
        };
        let json = serde_json::to_string(&input).unwrap();
        let back: HookInput = serde_json::from_str(&json).unwrap();
        assert_eq!(back.event, HookEvent::PreToolUse);
        assert_eq!(back.tool_name, Some("bash".to_string()));
        assert!(back.tool_output.is_none());
    }

    #[test]
    fn hook_input_minimal() {
        let input = HookInput {
            event: HookEvent::SessionStart,
            tool_name: None,
            tool_input: None,
            tool_output: None,
            session_id: None,
        };
        let json = serde_json::to_string(&input).unwrap();
        let back: HookInput = serde_json::from_str(&json).unwrap();
        assert_eq!(back.event, HookEvent::SessionStart);
        assert!(back.tool_name.is_none());
    }

    #[test]
    fn hook_output_serde_roundtrip() {
        let output = HookOutput {
            decision: Some(PermissionBehavior::Allow),
            reason: Some("Auto-approved".to_string()),
            updated_input: Some(serde_json::json!({"command": "ls -la"})),
            updated_output: None,
        };
        let json = serde_json::to_string(&output).unwrap();
        let back: HookOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(back.decision, Some(PermissionBehavior::Allow));
        assert_eq!(back.reason, Some("Auto-approved".to_string()));
    }

    #[test]
    fn hook_result_construction() {
        let success = HookResult::Success { output: None };
        match &success {
            HookResult::Success { output } => assert!(output.is_none()),
            _ => panic!("expected Success"),
        }

        let blocking = HookResult::Blocking {
            message: "Blocked by policy".to_string(),
        };
        match &blocking {
            HookResult::Blocking { message } => assert_eq!(message, "Blocked by policy"),
            _ => panic!("expected Blocking"),
        }

        let error = HookResult::Error {
            message: "Hook failed".to_string(),
        };
        match &error {
            HookResult::Error { message } => assert_eq!(message, "Hook failed"),
            _ => panic!("expected Error"),
        }

        let cancelled = HookResult::Cancelled;
        assert!(matches!(cancelled, HookResult::Cancelled));
    }

    #[test]
    fn hook_result_success_with_output() {
        let result = HookResult::Success {
            output: Some(HookOutput {
                decision: Some(PermissionBehavior::Deny),
                reason: Some("Not allowed".to_string()),
                updated_input: None,
                updated_output: None,
            }),
        };
        match result {
            HookResult::Success { output: Some(o) } => {
                assert_eq!(o.decision, Some(PermissionBehavior::Deny));
            }
            _ => panic!("expected Success with output"),
        }
    }
}
