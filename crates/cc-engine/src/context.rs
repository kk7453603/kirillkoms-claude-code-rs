use cc_api::types::{CacheControl, SystemBlock};
use std::path::Path;

/// System prompt parts
#[derive(Debug, Clone, Default)]
pub struct SystemContext {
    pub git_branch: Option<String>,
    pub git_status: Option<String>,
    pub cwd: String,
    pub os: String,
    pub date: String,
    pub claude_md_content: Option<String>,
    pub custom_system_prompt: Option<String>,
    pub append_system_prompt: Option<String>,
}

impl SystemContext {
    /// Build from current environment
    pub async fn from_env(project_root: &Path) -> Self {
        let git_branch = cc_utils::git::current_branch(project_root).await.ok();
        let git_status = cc_utils::git::git_status(project_root).await.ok();

        let cwd = project_root.to_str().unwrap_or(".").to_string();

        let os = std::env::consts::OS.to_string();
        let date = chrono::Utc::now().format("%Y-%m-%d").to_string();

        // Try to read CLAUDE.md from project root
        let claude_md_path = project_root.join("CLAUDE.md");
        let claude_md_content = tokio::fs::read_to_string(&claude_md_path).await.ok();

        Self {
            git_branch,
            git_status,
            cwd,
            os,
            date,
            claude_md_content,
            custom_system_prompt: None,
            append_system_prompt: None,
        }
    }

    /// Assemble into system prompt blocks for the API
    pub fn to_system_blocks(&self) -> Vec<SystemBlock> {
        let mut blocks = Vec::new();

        // Custom system prompt replaces the default if set
        if let Some(custom) = &self.custom_system_prompt {
            blocks.push(SystemBlock::Text {
                text: custom.clone(),
                cache_control: Some(CacheControl {
                    cache_type: "ephemeral".to_string(),
                }),
            });
        } else {
            // Build default system prompt
            let mut parts = Vec::new();

            parts.push(format!(
                "You are an AI coding assistant. You help users with software engineering tasks.\n\
                 Current date: {}. OS: {}. Working directory: {}.\n\n\
                 When using tools, follow these principles:\n\
                 - Read files before modifying them\n\
                 - Use the appropriate tool for each task (Bash for commands, Read for files, Grep for search)\n\
                 - Handle errors gracefully and report them clearly\n\
                 - Be concise in your responses",
                self.date, self.os, self.cwd
            ));

            if let Some(branch) = &self.git_branch {
                parts.push(format!("Git branch: {}", branch));
            }

            if let Some(status) = &self.git_status
                && !status.trim().is_empty()
            {
                parts.push(format!("Git status:\n{}", status));
            }

            blocks.push(SystemBlock::Text {
                text: parts.join("\n\n"),
                cache_control: Some(CacheControl {
                    cache_type: "ephemeral".to_string(),
                }),
            });
        }

        // CLAUDE.md content gets its own block with cache control
        if let Some(claude_md) = &self.claude_md_content {
            blocks.push(SystemBlock::Text {
                text: format!("# Project Instructions (CLAUDE.md)\n\n{}", claude_md),
                cache_control: Some(CacheControl {
                    cache_type: "ephemeral".to_string(),
                }),
            });
        }

        // Append system prompt at the end
        if let Some(append) = &self.append_system_prompt {
            blocks.push(SystemBlock::Text {
                text: append.clone(),
                cache_control: None,
            });
        }

        blocks
    }

    /// Get total estimated tokens for system prompt
    pub fn estimated_tokens(&self) -> usize {
        let blocks = self.to_system_blocks();
        blocks
            .iter()
            .map(|b| match b {
                SystemBlock::Text { text, .. } => cc_utils::tokens::estimate_tokens(text),
            })
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_context_produces_blocks() {
        let ctx = SystemContext::default();
        let blocks = ctx.to_system_blocks();
        // Should have at least the default system block
        assert!(!blocks.is_empty());
        match &blocks[0] {
            SystemBlock::Text {
                text,
                cache_control,
            } => {
                assert!(text.contains("AI coding assistant"));
                assert!(cache_control.is_some());
            }
        }
    }

    #[test]
    fn test_context_with_git_info() {
        let ctx = SystemContext {
            git_branch: Some("main".to_string()),
            git_status: Some(" M src/lib.rs\n".to_string()),
            cwd: "/home/user/project".to_string(),
            os: "linux".to_string(),
            date: "2026-03-31".to_string(),
            ..Default::default()
        };
        let blocks = ctx.to_system_blocks();
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            SystemBlock::Text { text, .. } => {
                assert!(text.contains("main"));
                assert!(text.contains("src/lib.rs"));
                assert!(text.contains("linux"));
                assert!(text.contains("2026-03-31"));
            }
        }
    }

    #[test]
    fn test_context_with_claude_md() {
        let ctx = SystemContext {
            cwd: "/tmp".to_string(),
            os: "linux".to_string(),
            date: "2026-03-31".to_string(),
            claude_md_content: Some("Always use Rust.".to_string()),
            ..Default::default()
        };
        let blocks = ctx.to_system_blocks();
        assert_eq!(blocks.len(), 2);
        match &blocks[1] {
            SystemBlock::Text {
                text,
                cache_control,
            } => {
                assert!(text.contains("Always use Rust."));
                assert!(text.contains("CLAUDE.md"));
                assert!(cache_control.is_some());
            }
        }
    }

    #[test]
    fn test_custom_system_prompt_replaces_default() {
        let ctx = SystemContext {
            cwd: "/tmp".to_string(),
            os: "linux".to_string(),
            date: "2026-03-31".to_string(),
            custom_system_prompt: Some("You are a pirate.".to_string()),
            ..Default::default()
        };
        let blocks = ctx.to_system_blocks();
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            SystemBlock::Text { text, .. } => {
                assert_eq!(text, "You are a pirate.");
            }
        }
    }

    #[test]
    fn test_append_system_prompt() {
        let ctx = SystemContext {
            cwd: "/tmp".to_string(),
            os: "linux".to_string(),
            date: "2026-03-31".to_string(),
            append_system_prompt: Some("Be concise.".to_string()),
            ..Default::default()
        };
        let blocks = ctx.to_system_blocks();
        assert_eq!(blocks.len(), 2);
        match &blocks[1] {
            SystemBlock::Text {
                text,
                cache_control,
            } => {
                assert_eq!(text, "Be concise.");
                assert!(cache_control.is_none());
            }
        }
    }

    #[test]
    fn test_estimated_tokens() {
        let ctx = SystemContext {
            cwd: "/tmp".to_string(),
            os: "linux".to_string(),
            date: "2026-03-31".to_string(),
            ..Default::default()
        };
        let tokens = ctx.estimated_tokens();
        assert!(tokens > 0);
    }

    #[test]
    fn test_empty_git_status_not_included() {
        let ctx = SystemContext {
            git_status: Some("   ".to_string()),
            cwd: "/tmp".to_string(),
            os: "linux".to_string(),
            date: "2026-03-31".to_string(),
            ..Default::default()
        };
        let blocks = ctx.to_system_blocks();
        match &blocks[0] {
            SystemBlock::Text { text, .. } => {
                assert!(!text.contains("Git status"));
            }
        }
    }
}
