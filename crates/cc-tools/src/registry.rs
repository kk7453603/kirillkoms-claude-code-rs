use std::collections::HashMap;
use std::sync::Arc;

use crate::tools::{
    agent::AgentTool,
    ask_user::AskUserQuestionTool,
    bash::BashTool,
    communication::{BriefTool, SendMessageTool},
    config_tool::ConfigTool,
    file_edit::FileEditTool,
    file_read::FileReadTool,
    file_write::FileWriteTool,
    glob::GlobTool,
    grep::GrepTool,
    lsp::LspTool,
    mcp_tools::{ListMcpResourcesTool, ReadMcpResourceTool},
    notebook_edit::NotebookEditTool,
    plan_mode::{EnterPlanModeTool, ExitPlanModeV2Tool},
    powershell::PowerShellTool,
    skill::SkillTool,
    sleep::SleepTool,
    task_tools::{
        TaskCreateTool, TaskGetTool, TaskListTool, TaskOutputTool, TaskStopTool, TaskUpdateTool,
    },
    todo_write::TodoWriteTool,
    tool_search::ToolSearchTool,
    web_fetch::WebFetchTool,
    web_search::WebSearchTool,
    worktree::{EnterWorktreeTool, ExitWorktreeTool},
};
use crate::trait_def::Tool;

/// Registry for managing available tools.
#[derive(Default)]
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Create a registry with all default tools registered.
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();

        // Core tools (always available)
        registry.register(Arc::new(BashTool::new()));
        registry.register(Arc::new(FileReadTool::new()));
        registry.register(Arc::new(FileEditTool::new()));
        registry.register(Arc::new(FileWriteTool::new()));
        registry.register(Arc::new(GlobTool::new()));
        registry.register(Arc::new(GrepTool::new()));

        // Deferred / advanced tools
        registry.register(Arc::new(AgentTool::new()));
        registry.register(Arc::new(AskUserQuestionTool::new()));
        registry.register(Arc::new(BriefTool::new()));
        registry.register(Arc::new(SendMessageTool::new()));
        registry.register(Arc::new(ConfigTool::new()));
        registry.register(Arc::new(LspTool::new()));
        registry.register(Arc::new(ListMcpResourcesTool::new()));
        registry.register(Arc::new(ReadMcpResourceTool::new()));
        registry.register(Arc::new(NotebookEditTool::new()));
        registry.register(Arc::new(EnterPlanModeTool::new()));
        registry.register(Arc::new(ExitPlanModeV2Tool::new()));
        registry.register(Arc::new(PowerShellTool::new()));
        registry.register(Arc::new(SkillTool::new()));
        registry.register(Arc::new(SleepTool::new()));
        registry.register(Arc::new(TaskCreateTool::new()));
        registry.register(Arc::new(TaskGetTool::new()));
        registry.register(Arc::new(TaskUpdateTool::new()));
        registry.register(Arc::new(TaskStopTool::new()));
        registry.register(Arc::new(TaskListTool::new()));
        registry.register(Arc::new(TaskOutputTool::new()));
        registry.register(Arc::new(TodoWriteTool::new()));
        registry.register(Arc::new(ToolSearchTool::new()));
        registry.register(Arc::new(WebFetchTool::new()));
        registry.register(Arc::new(WebSearchTool::new()));
        registry.register(Arc::new(EnterWorktreeTool::new()));
        registry.register(Arc::new(ExitWorktreeTool::new()));

        registry
    }

    /// Register a tool.
    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        let name = tool.name().to_string();
        self.tools.insert(name, tool);
    }

    /// Look up a tool by name or alias.
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        if let Some(tool) = self.tools.get(name) {
            return Some(Arc::clone(tool));
        }
        // Search aliases
        for tool in self.tools.values() {
            if tool.aliases().contains(&name) {
                return Some(Arc::clone(tool));
            }
        }
        None
    }

    /// Get all enabled tools.
    pub fn enabled_tools(&self) -> Vec<Arc<dyn Tool>> {
        self.tools
            .values()
            .filter(|t| t.is_enabled())
            .cloned()
            .collect()
    }

    /// Get all tool names.
    pub fn tool_names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// Number of registered tools.
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

impl std::fmt::Debug for ToolRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolRegistry")
            .field("tool_count", &self.tools.len())
            .field("tool_names", &self.tool_names())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_registry() {
        let reg = ToolRegistry::new();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
        assert!(reg.get("nonexistent").is_none());
        assert!(reg.enabled_tools().is_empty());
    }

    #[test]
    fn test_with_defaults() {
        let reg = ToolRegistry::with_defaults();
        assert!(!reg.is_empty());

        // Core tools should be present
        assert!(reg.get("Bash").is_some());
        assert!(reg.get("Read").is_some());
        assert!(reg.get("Edit").is_some());
        assert!(reg.get("Write").is_some());
        assert!(reg.get("Glob").is_some());
        assert!(reg.get("Grep").is_some());

        // Deferred tools
        assert!(reg.get("Agent").is_some());
        assert!(reg.get("TodoWrite").is_some());
        assert!(reg.get("Sleep").is_some());
        assert!(reg.get("NotebookEdit").is_some());
        assert!(reg.get("WebFetch").is_some());
        assert!(reg.get("ToolSearch").is_some());

        // Aliases
        assert!(reg.get("FileRead").is_some());
        assert!(reg.get("SubAgent").is_some());
    }

    #[test]
    fn test_with_defaults_has_all_tools() {
        let reg = ToolRegistry::with_defaults();
        // We register 31 tools total (some multi-struct files contribute multiple)
        assert!(
            reg.len() >= 28,
            "Expected at least 28 tools, got {}",
            reg.len()
        );
    }

    #[test]
    fn test_enabled_tools_filter() {
        let reg = ToolRegistry::with_defaults();
        let enabled = reg.enabled_tools();
        // PowerShell may not be enabled on Linux, so enabled count may differ
        assert!(!enabled.is_empty());
    }
}
