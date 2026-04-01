use crate::types::HookInput;

/// Build hook input for a PreToolUse event.
pub fn pre_tool_use_input(
    tool_name: &str,
    tool_input: &serde_json::Value,
    session_id: Option<&str>,
) -> HookInput {
    HookInput {
        hook_event: "PreToolUse".to_string(),
        tool_name: Some(tool_name.to_string()),
        tool_input: Some(tool_input.clone()),
        tool_output: None,
        session_id: session_id.map(|s| s.to_string()),
        cwd: None,
    }
}

/// Build hook input for a PostToolUse event.
pub fn post_tool_use_input(
    tool_name: &str,
    tool_input: &serde_json::Value,
    tool_output: &serde_json::Value,
    session_id: Option<&str>,
) -> HookInput {
    HookInput {
        hook_event: "PostToolUse".to_string(),
        tool_name: Some(tool_name.to_string()),
        tool_input: Some(tool_input.clone()),
        tool_output: Some(tool_output.clone()),
        session_id: session_id.map(|s| s.to_string()),
        cwd: None,
    }
}

/// Build hook input for a SessionStart event.
pub fn session_start_input(session_id: &str, cwd: &str) -> HookInput {
    HookInput {
        hook_event: "SessionStart".to_string(),
        tool_name: None,
        tool_input: None,
        tool_output: None,
        session_id: Some(session_id.to_string()),
        cwd: Some(cwd.to_string()),
    }
}

/// Build hook input for a UserPromptSubmit event.
pub fn user_prompt_submit_input(prompt: &str, session_id: Option<&str>) -> HookInput {
    HookInput {
        hook_event: "UserPromptSubmit".to_string(),
        tool_name: None,
        tool_input: Some(serde_json::json!({ "prompt": prompt })),
        tool_output: None,
        session_id: session_id.map(|s| s.to_string()),
        cwd: None,
    }
}

/// Build hook input for a FileChanged event.
pub fn file_changed_input(file_path: &str, session_id: Option<&str>) -> HookInput {
    HookInput {
        hook_event: "FileChanged".to_string(),
        tool_name: None,
        tool_input: Some(serde_json::json!({ "file_path": file_path })),
        tool_output: None,
        session_id: session_id.map(|s| s.to_string()),
        cwd: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pre_tool_use_input() {
        let input = pre_tool_use_input(
            "Bash",
            &serde_json::json!({"command": "ls -la"}),
            Some("sess-1"),
        );
        assert_eq!(input.hook_event, "PreToolUse");
        assert_eq!(input.tool_name.as_deref(), Some("Bash"));
        assert_eq!(input.tool_input.as_ref().unwrap()["command"], "ls -la");
        assert!(input.tool_output.is_none());
        assert_eq!(input.session_id.as_deref(), Some("sess-1"));
        assert!(input.cwd.is_none());
    }

    #[test]
    fn test_pre_tool_use_input_no_session() {
        let input = pre_tool_use_input("Read", &serde_json::json!({}), None);
        assert_eq!(input.hook_event, "PreToolUse");
        assert!(input.session_id.is_none());
    }

    #[test]
    fn test_post_tool_use_input() {
        let tool_in = serde_json::json!({"file": "test.rs"});
        let tool_out = serde_json::json!({"content": "hello"});
        let input = post_tool_use_input("Read", &tool_in, &tool_out, Some("sess-2"));
        assert_eq!(input.hook_event, "PostToolUse");
        assert_eq!(input.tool_name.as_deref(), Some("Read"));
        assert_eq!(input.tool_input.as_ref().unwrap()["file"], "test.rs");
        assert_eq!(input.tool_output.as_ref().unwrap()["content"], "hello");
        assert_eq!(input.session_id.as_deref(), Some("sess-2"));
    }

    #[test]
    fn test_session_start_input() {
        let input = session_start_input("sess-42", "/home/user/project");
        assert_eq!(input.hook_event, "SessionStart");
        assert!(input.tool_name.is_none());
        assert!(input.tool_input.is_none());
        assert!(input.tool_output.is_none());
        assert_eq!(input.session_id.as_deref(), Some("sess-42"));
        assert_eq!(input.cwd.as_deref(), Some("/home/user/project"));
    }

    #[test]
    fn test_user_prompt_submit_input() {
        let input = user_prompt_submit_input("fix the bug", Some("sess-7"));
        assert_eq!(input.hook_event, "UserPromptSubmit");
        assert_eq!(input.tool_input.as_ref().unwrap()["prompt"], "fix the bug");
        assert_eq!(input.session_id.as_deref(), Some("sess-7"));
    }

    #[test]
    fn test_user_prompt_submit_input_no_session() {
        let input = user_prompt_submit_input("hello", None);
        assert!(input.session_id.is_none());
    }

    #[test]
    fn test_file_changed_input() {
        let input = file_changed_input("/tmp/foo.rs", Some("sess-9"));
        assert_eq!(input.hook_event, "FileChanged");
        assert_eq!(
            input.tool_input.as_ref().unwrap()["file_path"],
            "/tmp/foo.rs"
        );
        assert_eq!(input.session_id.as_deref(), Some("sess-9"));
    }

    #[test]
    fn test_file_changed_input_no_session() {
        let input = file_changed_input("/tmp/bar.rs", None);
        assert!(input.session_id.is_none());
    }

    #[test]
    fn test_all_builders_produce_valid_json() {
        // Ensure all builders serialize to JSON without error.
        let inputs = vec![
            pre_tool_use_input("T", &serde_json::json!({}), None),
            post_tool_use_input("T", &serde_json::json!({}), &serde_json::json!({}), None),
            session_start_input("s", "/"),
            user_prompt_submit_input("p", None),
            file_changed_input("f", None),
        ];
        for input in &inputs {
            let json = serde_json::to_string(input);
            assert!(json.is_ok(), "Failed to serialize: {:?}", input);
        }
    }
}
