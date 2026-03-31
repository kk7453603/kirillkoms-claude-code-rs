use std::collections::HashMap;
use std::sync::Arc;

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
}
