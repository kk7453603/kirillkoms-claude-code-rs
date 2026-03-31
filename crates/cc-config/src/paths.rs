use std::path::{Path, PathBuf};

/// Get Claude config directory (~/.claude or $CLAUDE_CONFIG_DIR).
pub fn config_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("CLAUDE_CONFIG_DIR") {
        return PathBuf::from(dir);
    }
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".claude")
}

/// Get global settings path (~/.claude/settings.json).
pub fn global_settings_path() -> PathBuf {
    config_dir().join("settings.json")
}

/// Get project settings path (<project>/.claude/settings.json).
pub fn project_settings_path(project_root: &Path) -> PathBuf {
    project_root.join(".claude").join("settings.json")
}

/// Get local settings path (<project>/.claude/settings.local.json).
pub fn local_settings_path(project_root: &Path) -> PathBuf {
    project_root.join(".claude").join("settings.local.json")
}

/// Get session storage dir (~/.claude/sessions/).
pub fn sessions_dir() -> PathBuf {
    config_dir().join("sessions")
}

/// Get history file path (~/.claude/history.jsonl).
pub fn history_path() -> PathBuf {
    config_dir().join("history.jsonl")
}

/// Get CLAUDE.md file paths to check (in order of precedence).
///
/// Returns paths from most specific (project-level) to least specific (home dir).
pub fn claude_md_paths(project_root: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // Project-level paths (highest precedence)
    paths.push(project_root.join(".claude").join("CLAUDE.md"));
    paths.push(project_root.join("CLAUDE.md"));

    // Walk up parent directories
    let mut dir = project_root.parent();
    while let Some(parent) = dir {
        paths.push(parent.join(".claude").join("CLAUDE.md"));
        paths.push(parent.join("CLAUDE.md"));
        dir = parent.parent();
    }

    // Home directory
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let home_claude_md = home.join(".claude").join("CLAUDE.md");
    if !paths.contains(&home_claude_md) {
        paths.push(home_claude_md);
    }

    paths
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn config_dir_default() {
        unsafe { std::env::remove_var("CLAUDE_CONFIG_DIR"); }
        let dir = config_dir();
        assert!(
            dir.to_string_lossy().contains(".claude"),
            "config_dir should contain .claude, got: {:?}",
            dir
        );
    }

    #[test]
    fn config_dir_from_env() {
        unsafe {
            std::env::set_var("CLAUDE_CONFIG_DIR", "/tmp/custom-claude");
        }
        let dir = config_dir();
        assert_eq!(dir, PathBuf::from("/tmp/custom-claude"));
        unsafe { std::env::remove_var("CLAUDE_CONFIG_DIR"); }
    }

    #[test]
    fn global_settings_path_contains_settings_json() {
        unsafe { std::env::remove_var("CLAUDE_CONFIG_DIR"); }
        let p = global_settings_path();
        assert!(p.ends_with("settings.json"));
        assert!(p.to_string_lossy().contains(".claude"));
    }

    #[test]
    fn project_settings_path_correct() {
        let root = Path::new("/home/user/myproject");
        let p = project_settings_path(root);
        assert_eq!(p, PathBuf::from("/home/user/myproject/.claude/settings.json"));
    }

    #[test]
    fn local_settings_path_correct() {
        let root = Path::new("/home/user/myproject");
        let p = local_settings_path(root);
        assert_eq!(
            p,
            PathBuf::from("/home/user/myproject/.claude/settings.local.json")
        );
    }

    #[test]
    fn sessions_dir_correct() {
        unsafe { std::env::remove_var("CLAUDE_CONFIG_DIR"); }
        let p = sessions_dir();
        assert!(p.ends_with("sessions"));
        assert!(p.to_string_lossy().contains(".claude"));
    }

    #[test]
    fn history_path_correct() {
        unsafe { std::env::remove_var("CLAUDE_CONFIG_DIR"); }
        let p = history_path();
        assert!(p.ends_with("history.jsonl"));
    }

    #[test]
    fn claude_md_paths_includes_project_root() {
        let root = Path::new("/home/user/myproject");
        let paths = claude_md_paths(root);
        assert!(!paths.is_empty());
        // Should include project root paths first
        assert_eq!(paths[0], PathBuf::from("/home/user/myproject/.claude/CLAUDE.md"));
        assert_eq!(paths[1], PathBuf::from("/home/user/myproject/CLAUDE.md"));
    }

    #[test]
    fn claude_md_paths_walks_parents() {
        let root = Path::new("/home/user/myproject/subdir");
        let paths = claude_md_paths(root);
        // Should include parent directories
        let has_parent = paths.iter().any(|p| {
            p == &PathBuf::from("/home/user/myproject/CLAUDE.md")
        });
        assert!(has_parent, "Should include parent directory CLAUDE.md");
    }
}
