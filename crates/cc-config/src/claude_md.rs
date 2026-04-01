use std::path::{Path, PathBuf};

/// Discover CLAUDE.md files for the current project.
///
/// Searches for CLAUDE.md in the project root and its `.claude/` subdirectory,
/// then walks up parent directories doing the same. Files are returned in
/// order from most specific (project root) to least specific (parents).
/// Only paths that actually exist on disk are returned.
pub fn discover_claude_md_files(project_root: &Path) -> Vec<PathBuf> {
    let candidates = crate::paths::claude_md_paths(project_root);
    candidates.into_iter().filter(|p| p.is_file()).collect()
}

/// Load and concatenate all CLAUDE.md content.
///
/// Discovers all CLAUDE.md files and concatenates their contents separated by
/// newlines, with the most specific file first.
pub fn load_claude_md_content(project_root: &Path) -> Result<String, std::io::Error> {
    let files = discover_claude_md_files(project_root);
    if files.is_empty() {
        return Ok(String::new());
    }

    let mut content = String::new();
    for (i, path) in files.iter().enumerate() {
        if i > 0 {
            content.push_str("\n\n");
        }
        let file_content = std::fs::read_to_string(path)?;
        content.push_str(&file_content);
    }

    Ok(content)
}

/// Check if a directory contains a CLAUDE.md file.
///
/// Checks for both `<dir>/CLAUDE.md` and `<dir>/.claude/CLAUDE.md`.
pub fn has_claude_md(dir: &Path) -> bool {
    dir.join("CLAUDE.md").is_file() || dir.join(".claude").join("CLAUDE.md").is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discover_no_files() {
        let dir = tempfile::tempdir().unwrap();
        let files = discover_claude_md_files(dir.path());
        assert!(files.is_empty());
    }

    #[test]
    fn discover_root_claude_md() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("CLAUDE.md"),
            "# Project\nInstructions here.",
        )
        .unwrap();
        let files = discover_claude_md_files(dir.path());
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("CLAUDE.md"));
    }

    #[test]
    fn discover_dotclaude_claude_md() {
        let dir = tempfile::tempdir().unwrap();
        let claude_dir = dir.path().join(".claude");
        std::fs::create_dir_all(&claude_dir).unwrap();
        std::fs::write(claude_dir.join("CLAUDE.md"), "# Config\nSettings.").unwrap();
        let files = discover_claude_md_files(dir.path());
        assert_eq!(files.len(), 1);
        assert!(files[0].to_string_lossy().contains(".claude"));
    }

    #[test]
    fn discover_both_locations() {
        let dir = tempfile::tempdir().unwrap();
        let claude_dir = dir.path().join(".claude");
        std::fs::create_dir_all(&claude_dir).unwrap();
        std::fs::write(claude_dir.join("CLAUDE.md"), "From .claude dir").unwrap();
        std::fs::write(dir.path().join("CLAUDE.md"), "From root").unwrap();
        let files = discover_claude_md_files(dir.path());
        assert_eq!(files.len(), 2);
        // .claude/CLAUDE.md should come before root CLAUDE.md (more specific first)
        assert!(files[0].to_string_lossy().contains(".claude"));
    }

    #[test]
    fn load_content_empty_when_no_files() {
        let dir = tempfile::tempdir().unwrap();
        let content = load_claude_md_content(dir.path()).unwrap();
        assert!(content.is_empty());
    }

    #[test]
    fn load_content_single_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("CLAUDE.md"), "Hello World").unwrap();
        let content = load_claude_md_content(dir.path()).unwrap();
        assert_eq!(content, "Hello World");
    }

    #[test]
    fn load_content_multiple_files_concatenated() {
        let dir = tempfile::tempdir().unwrap();
        let claude_dir = dir.path().join(".claude");
        std::fs::create_dir_all(&claude_dir).unwrap();
        std::fs::write(claude_dir.join("CLAUDE.md"), "Part 1").unwrap();
        std::fs::write(dir.path().join("CLAUDE.md"), "Part 2").unwrap();
        let content = load_claude_md_content(dir.path()).unwrap();
        assert!(content.contains("Part 1"));
        assert!(content.contains("Part 2"));
        assert!(content.contains("\n\n"));
    }

    #[test]
    fn has_claude_md_false_for_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!has_claude_md(dir.path()));
    }

    #[test]
    fn has_claude_md_true_for_root_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("CLAUDE.md"), "content").unwrap();
        assert!(has_claude_md(dir.path()));
    }

    #[test]
    fn has_claude_md_true_for_dotclaude_file() {
        let dir = tempfile::tempdir().unwrap();
        let claude_dir = dir.path().join(".claude");
        std::fs::create_dir_all(&claude_dir).unwrap();
        std::fs::write(claude_dir.join("CLAUDE.md"), "content").unwrap();
        assert!(has_claude_md(dir.path()));
    }

    #[test]
    fn has_claude_md_false_for_directory_named_claude_md() {
        let dir = tempfile::tempdir().unwrap();
        // Create a directory named CLAUDE.md (not a file)
        std::fs::create_dir_all(dir.path().join("CLAUDE.md")).unwrap();
        assert!(!has_claude_md(dir.path()));
    }
}
