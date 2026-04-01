use std::path::PathBuf;

/// Simplified tool context for execution.
#[derive(Debug, Clone)]
pub struct ToolContext {
    pub cwd: PathBuf,
    pub project_root: PathBuf,
    pub session_id: String,
    pub model: String,
}

impl ToolContext {
    pub fn new(cwd: PathBuf) -> Self {
        let project_root = cwd.clone();
        Self {
            cwd,
            project_root,
            session_id: uuid::Uuid::new_v4().to_string(),
            model: "claude-sonnet-4-20250514".to_string(),
        }
    }

    pub fn with_project_root(mut self, root: PathBuf) -> Self {
        self.project_root = root;
        self
    }

    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }

    pub fn with_session_id(mut self, session_id: String) -> Self {
        self.session_id = session_id;
        self
    }
}

impl Default for ToolContext {
    fn default() -> Self {
        Self::new(std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_context() {
        let ctx = ToolContext::new(PathBuf::from("/tmp"));
        assert_eq!(ctx.cwd, PathBuf::from("/tmp"));
        assert_eq!(ctx.project_root, PathBuf::from("/tmp"));
        assert!(!ctx.session_id.is_empty());
    }

    #[test]
    fn test_builder_methods() {
        let ctx = ToolContext::new(PathBuf::from("/tmp"))
            .with_project_root(PathBuf::from("/home"))
            .with_model("claude-opus-4-20250514".to_string())
            .with_session_id("test-session".to_string());
        assert_eq!(ctx.project_root, PathBuf::from("/home"));
        assert_eq!(ctx.model, "claude-opus-4-20250514");
        assert_eq!(ctx.session_id, "test-session");
    }

    #[test]
    fn test_default_context() {
        let ctx = ToolContext::default();
        assert!(!ctx.session_id.is_empty());
        assert!(!ctx.model.is_empty());
    }
}
