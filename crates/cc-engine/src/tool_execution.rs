use cc_api::types::ContentBlock;
use cc_hooks::dispatch::dispatch_hooks;
use cc_hooks::events;
use cc_hooks::types::{HookEventType, HookOutcome, HooksConfig};
use cc_permissions::checker::{PermissionContext, PermissionDecision};
use cc_tools::trait_def::{Tool, ToolError, ToolResult};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;

use crate::orchestration::ToolCallResult;

/// Execute a single tool call.
pub async fn execute_single_tool(
    tool: Arc<dyn Tool>,
    input: Value,
    tool_use_id: &str,
) -> ToolCallResult {
    let start = std::time::Instant::now();
    let tool_name = tool.name().to_string();

    let result = tool.call(input).await;
    let duration_ms = start.elapsed().as_millis() as u64;

    ToolCallResult {
        tool_use_id: tool_use_id.to_string(),
        tool_name,
        result,
        duration_ms,
    }
}

/// Callback for interactive permission prompts.
#[async_trait::async_trait]
pub trait PermissionCallback: Send + Sync {
    async fn ask_permission(
        &self,
        tool_name: &str,
        message: &str,
        input: &serde_json::Value,
    ) -> bool;
}

/// Default: always approve (for non-interactive mode).
pub struct AutoApproveCallback;

#[async_trait::async_trait]
impl PermissionCallback for AutoApproveCallback {
    async fn ask_permission(
        &self,
        _tool_name: &str,
        _message: &str,
        _input: &serde_json::Value,
    ) -> bool {
        true
    }
}

/// Interactive: prompt user on stderr.
pub struct InteractivePermissionCallback;

#[async_trait::async_trait]
impl PermissionCallback for InteractivePermissionCallback {
    async fn ask_permission(
        &self,
        tool_name: &str,
        message: &str,
        input: &serde_json::Value,
    ) -> bool {
        eprintln!("\n\u{26a0} Permission required for tool: {}", tool_name);
        eprintln!("  {}", message);
        if let Some(cmd) = input.get("command").and_then(|v| v.as_str()) {
            eprintln!("  Command: {}", cmd);
        }
        if let Some(path) = input.get("file_path").and_then(|v| v.as_str()) {
            eprintln!("  File: {}", path);
        }
        eprint!("  Allow? [y/N]: ");

        let mut response = String::new();
        std::io::stdin().read_line(&mut response).unwrap_or(0);
        let response = response.trim().to_lowercase();
        matches!(response.as_str(), "y" | "yes")
    }
}

/// Context passed through the execution pipeline for permissions, hooks, and session.
#[derive(Clone)]
pub struct ExecutionContext {
    pub permission_ctx: PermissionContext,
    pub hooks_config: HooksConfig,
    pub session_id: Option<String>,
    pub cwd: PathBuf,
}

impl ExecutionContext {
    pub fn new(permission_ctx: PermissionContext, cwd: PathBuf) -> Self {
        Self {
            permission_ctx,
            hooks_config: HooksConfig::new(),
            session_id: None,
            cwd,
        }
    }
}

/// Execute a single tool call with permission checking and hook dispatch.
pub async fn execute_single_tool_with_context(
    tool: Arc<dyn Tool>,
    input: Value,
    tool_use_id: &str,
    exec_ctx: &ExecutionContext,
    permission_callback: &dyn PermissionCallback,
) -> ToolCallResult {
    let tool_name = tool.name().to_string();

    // 1. Run PreToolUse hooks
    let pre_input = events::pre_tool_use_input(
        &tool_name,
        &input,
        exec_ctx.session_id.as_deref(),
    );
    let pre_result = dispatch_hooks(
        &exec_ctx.hooks_config,
        HookEventType::PreToolUse,
        &pre_input,
        &exec_ctx.cwd,
    )
    .await;

    let final_input = match pre_result {
        HookOutcome::Blocked { reason } => {
            return ToolCallResult {
                tool_use_id: tool_use_id.to_string(),
                tool_name,
                result: Ok(ToolResult::error(&format!(
                    "Blocked by hook: {}",
                    reason
                ))),
                duration_ms: 0,
            };
        }
        HookOutcome::Approved { updated_input, .. } => updated_input.unwrap_or(input),
        _ => input,
    };

    // 2. Check permissions
    let is_read_only = tool.is_read_only(&final_input);
    let is_destructive = tool.is_destructive(&final_input);
    let decision = exec_ctx.permission_ctx.check_permission(
        &tool_name,
        &final_input,
        is_read_only,
        is_destructive,
    );

    match decision {
        PermissionDecision::Allow { .. } => {}
        PermissionDecision::Deny { reason } => {
            return ToolCallResult {
                tool_use_id: tool_use_id.to_string(),
                tool_name,
                result: Ok(ToolResult::error(&format!(
                    "Permission denied: {}",
                    reason
                ))),
                duration_ms: 0,
            };
        }
        PermissionDecision::Ask { message, .. } => {
            let approved = permission_callback
                .ask_permission(&tool_name, &message, &final_input)
                .await;
            if !approved {
                return ToolCallResult {
                    tool_use_id: tool_use_id.to_string(),
                    tool_name,
                    result: Ok(ToolResult::error("User denied permission")),
                    duration_ms: 0,
                };
            }
        }
    }

    // 3. Execute tool
    let result = execute_single_tool(tool.clone(), final_input.clone(), tool_use_id).await;

    // 4. Run PostToolUse hooks
    let output_value = match &result.result {
        Ok(tr) => tr.content.clone(),
        Err(e) => serde_json::Value::String(e.to_string()),
    };
    let post_input = events::post_tool_use_input(
        &tool_name,
        &final_input,
        &output_value,
        exec_ctx.session_id.as_deref(),
    );
    let _ = dispatch_hooks(
        &exec_ctx.hooks_config,
        HookEventType::PostToolUse,
        &post_input,
        &exec_ctx.cwd,
    )
    .await;

    result
}

/// Convert tool results to API content blocks.
pub fn tool_result_to_content_block(
    tool_use_id: &str,
    result: &Result<ToolResult, ToolError>,
) -> ContentBlock {
    match result {
        Ok(tr) => ContentBlock::ToolResult {
            tool_use_id: tool_use_id.to_string(),
            content: tr.content.clone(),
            is_error: if tr.is_error { Some(true) } else { Some(false) },
        },
        Err(e) => ContentBlock::ToolResult {
            tool_use_id: tool_use_id.to_string(),
            content: Value::String(e.to_string()),
            is_error: Some(true),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct MockTool {
        name: String,
        read_only: bool,
    }

    #[async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &str {
            &self.name
        }

        fn input_schema(&self) -> Value {
            serde_json::json!({
                "type": "object",
                "properties": {}
            })
        }

        async fn call(&self, _input: Value) -> Result<ToolResult, ToolError> {
            Ok(ToolResult::text("mock result"))
        }

        fn description(&self) -> String {
            "A mock tool".to_string()
        }

        fn is_read_only(&self, _input: &Value) -> bool {
            self.read_only
        }

        fn is_concurrency_safe(&self, _input: &Value) -> bool {
            self.read_only
        }
    }

    struct FailingTool;

    #[async_trait]
    impl Tool for FailingTool {
        fn name(&self) -> &str {
            "failing"
        }

        fn input_schema(&self) -> Value {
            serde_json::json!({"type": "object"})
        }

        async fn call(&self, _input: Value) -> Result<ToolResult, ToolError> {
            Err(ToolError::ExecutionFailed {
                message: "something broke".to_string(),
            })
        }

        fn description(&self) -> String {
            "A failing tool".to_string()
        }

        fn is_read_only(&self, _input: &Value) -> bool {
            true
        }

        fn is_concurrency_safe(&self, _input: &Value) -> bool {
            true
        }
    }

    #[tokio::test]
    async fn test_execute_single_tool_success() {
        let tool = Arc::new(MockTool {
            name: "test_tool".to_string(),
            read_only: true,
        });
        let result =
            execute_single_tool(tool, serde_json::json!({}), "tu_1").await;
        assert_eq!(result.tool_use_id, "tu_1");
        assert_eq!(result.tool_name, "test_tool");
        assert!(result.result.is_ok());
        let tr = result.result.unwrap();
        assert!(!tr.is_error);
        assert_eq!(tr.content, Value::String("mock result".to_string()));
    }

    #[tokio::test]
    async fn test_execute_single_tool_failure() {
        let tool: Arc<dyn Tool> = Arc::new(FailingTool);
        let result =
            execute_single_tool(tool, serde_json::json!({}), "tu_2").await;
        assert_eq!(result.tool_use_id, "tu_2");
        assert!(result.result.is_err());
    }

    #[tokio::test]
    async fn test_execute_single_tool_records_duration() {
        let tool = Arc::new(MockTool {
            name: "fast".to_string(),
            read_only: true,
        });
        let result =
            execute_single_tool(tool, serde_json::json!({}), "tu_3").await;
        // Duration should be non-negative (might be 0 for very fast calls)
        assert!(result.duration_ms < 10_000);
    }

    #[test]
    fn test_tool_result_to_content_block_success() {
        let tr = Ok(ToolResult::text("hello"));
        let block = tool_result_to_content_block("tu_1", &tr);
        match block {
            ContentBlock::ToolResult {
                tool_use_id,
                content,
                is_error,
            } => {
                assert_eq!(tool_use_id, "tu_1");
                assert_eq!(content, Value::String("hello".to_string()));
                assert_eq!(is_error, Some(false));
            }
            _ => panic!("expected ToolResult"),
        }
    }

    #[test]
    fn test_tool_result_to_content_block_tool_error_flag() {
        let tr = Ok(ToolResult::error("bad input"));
        let block = tool_result_to_content_block("tu_2", &tr);
        match block {
            ContentBlock::ToolResult { is_error, .. } => {
                assert_eq!(is_error, Some(true));
            }
            _ => panic!("expected ToolResult"),
        }
    }

    #[test]
    fn test_tool_result_to_content_block_error() {
        let tr: Result<ToolResult, ToolError> = Err(ToolError::ExecutionFailed {
            message: "boom".to_string(),
        });
        let block = tool_result_to_content_block("tu_3", &tr);
        match block {
            ContentBlock::ToolResult {
                tool_use_id,
                content,
                is_error,
            } => {
                assert_eq!(tool_use_id, "tu_3");
                assert!(content.as_str().unwrap().contains("boom"));
                assert_eq!(is_error, Some(true));
            }
            _ => panic!("expected ToolResult"),
        }
    }

    #[test]
    fn test_execution_context_new() {
        let perm_ctx = cc_permissions::checker::PermissionContext::new(
            cc_permissions::modes::PermissionMode::Default,
        );
        let ctx = ExecutionContext::new(perm_ctx, PathBuf::from("/tmp"));
        assert_eq!(ctx.cwd, PathBuf::from("/tmp"));
        assert!(ctx.session_id.is_none());
        assert!(ctx.hooks_config.is_empty());
    }

    #[tokio::test]
    async fn test_auto_approve_callback() {
        let cb = AutoApproveCallback;
        let approved = cb
            .ask_permission("Bash", "run command", &serde_json::json!({}))
            .await;
        assert!(approved);
    }

    #[tokio::test]
    async fn test_execute_with_context_permission_allow() {
        // BypassPermissions mode allows everything
        let perm_ctx = cc_permissions::checker::PermissionContext::new(
            cc_permissions::modes::PermissionMode::BypassPermissions,
        );
        let exec_ctx = ExecutionContext::new(perm_ctx, PathBuf::from("/tmp"));
        let cb = AutoApproveCallback;

        let tool: Arc<dyn Tool> = Arc::new(MockTool {
            name: "write_tool".to_string(),
            read_only: false,
        });

        let result = execute_single_tool_with_context(
            tool,
            serde_json::json!({}),
            "tu_perm",
            &exec_ctx,
            &cb,
        )
        .await;

        assert_eq!(result.tool_use_id, "tu_perm");
        assert!(result.result.is_ok());
        let tr = result.result.unwrap();
        assert_eq!(tr.content, serde_json::Value::String("mock result".into()));
    }

    #[tokio::test]
    async fn test_execute_with_context_permission_deny() {
        // Plan mode denies non-read-only tools
        let perm_ctx = cc_permissions::checker::PermissionContext::new(
            cc_permissions::modes::PermissionMode::Plan,
        );
        let exec_ctx = ExecutionContext::new(perm_ctx, PathBuf::from("/tmp"));
        let cb = AutoApproveCallback;

        let tool: Arc<dyn Tool> = Arc::new(MockTool {
            name: "write_tool".to_string(),
            read_only: false,
        });

        let result = execute_single_tool_with_context(
            tool,
            serde_json::json!({}),
            "tu_deny",
            &exec_ctx,
            &cb,
        )
        .await;

        assert_eq!(result.tool_use_id, "tu_deny");
        assert!(result.result.is_ok());
        let tr = result.result.unwrap();
        assert!(tr.is_error);
        assert!(tr.content.as_str().unwrap().contains("Permission denied"));
    }

    #[tokio::test]
    async fn test_execute_with_context_permission_ask_approved() {
        // Default mode asks for write tools, AutoApproveCallback approves
        let perm_ctx = cc_permissions::checker::PermissionContext::new(
            cc_permissions::modes::PermissionMode::Default,
        );
        let exec_ctx = ExecutionContext::new(perm_ctx, PathBuf::from("/tmp"));
        let cb = AutoApproveCallback;

        let tool: Arc<dyn Tool> = Arc::new(MockTool {
            name: "write_tool".to_string(),
            read_only: false,
        });

        let result = execute_single_tool_with_context(
            tool,
            serde_json::json!({}),
            "tu_ask",
            &exec_ctx,
            &cb,
        )
        .await;

        // AutoApprove says yes, so tool should execute
        assert!(result.result.is_ok());
        let tr = result.result.unwrap();
        assert!(!tr.is_error);
    }

    /// A callback that always denies permission.
    struct DenyCallback;

    #[async_trait]
    impl PermissionCallback for DenyCallback {
        async fn ask_permission(
            &self,
            _tool_name: &str,
            _message: &str,
            _input: &serde_json::Value,
        ) -> bool {
            false
        }
    }

    #[tokio::test]
    async fn test_execute_with_context_permission_ask_denied() {
        // Default mode asks for write tools, DenyCallback denies
        let perm_ctx = cc_permissions::checker::PermissionContext::new(
            cc_permissions::modes::PermissionMode::Default,
        );
        let exec_ctx = ExecutionContext::new(perm_ctx, PathBuf::from("/tmp"));
        let cb = DenyCallback;

        let tool: Arc<dyn Tool> = Arc::new(MockTool {
            name: "write_tool".to_string(),
            read_only: false,
        });

        let result = execute_single_tool_with_context(
            tool,
            serde_json::json!({}),
            "tu_denied",
            &exec_ctx,
            &cb,
        )
        .await;

        assert!(result.result.is_ok());
        let tr = result.result.unwrap();
        assert!(tr.is_error);
        assert!(tr.content.as_str().unwrap().contains("User denied"));
    }

    #[tokio::test]
    async fn test_execute_with_context_hook_block() {
        let perm_ctx = cc_permissions::checker::PermissionContext::new(
            cc_permissions::modes::PermissionMode::BypassPermissions,
        );
        let mut exec_ctx = ExecutionContext::new(perm_ctx, PathBuf::from("/tmp"));

        // Add a hook that blocks
        exec_ctx.hooks_config.add(
            cc_hooks::types::HookEventType::PreToolUse,
            cc_hooks::types::HookConfig {
                command: r#"echo '{"decision":"block","reason":"test block"}'"#.to_string(),
                timeout_ms: 5000,
            },
        );

        let cb = AutoApproveCallback;
        let tool: Arc<dyn Tool> = Arc::new(MockTool {
            name: "any_tool".to_string(),
            read_only: true,
        });

        let result = execute_single_tool_with_context(
            tool,
            serde_json::json!({}),
            "tu_hook",
            &exec_ctx,
            &cb,
        )
        .await;

        assert!(result.result.is_ok());
        let tr = result.result.unwrap();
        assert!(tr.is_error);
        assert!(tr.content.as_str().unwrap().contains("Blocked by hook"));
        assert!(tr.content.as_str().unwrap().contains("test block"));
    }

    #[tokio::test]
    async fn test_execute_with_context_read_only_default_mode() {
        // Default mode allows read-only tools without asking
        let perm_ctx = cc_permissions::checker::PermissionContext::new(
            cc_permissions::modes::PermissionMode::Default,
        );
        let exec_ctx = ExecutionContext::new(perm_ctx, PathBuf::from("/tmp"));
        let cb = DenyCallback; // Would deny if asked, but shouldn't be asked

        let tool: Arc<dyn Tool> = Arc::new(MockTool {
            name: "read_tool".to_string(),
            read_only: true,
        });

        let result = execute_single_tool_with_context(
            tool,
            serde_json::json!({}),
            "tu_ro",
            &exec_ctx,
            &cb,
        )
        .await;

        // Read-only tool should be auto-allowed in Default mode
        assert!(result.result.is_ok());
        let tr = result.result.unwrap();
        assert!(!tr.is_error);
    }
}
