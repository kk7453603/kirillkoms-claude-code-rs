use std::collections::HashMap;
use std::path::{Path, PathBuf};

use chrono::Utc;

/// A single recorded file edit.
#[derive(Debug, Clone)]
pub struct FileEdit {
    pub timestamp: String,
    pub tool_name: String,
    pub old_content: Option<String>,
    pub new_content: String,
}

/// Tracks file edits made during a session.
#[derive(Debug, Default)]
pub struct FileHistory {
    edits: HashMap<PathBuf, Vec<FileEdit>>,
}

impl FileHistory {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a file edit.
    pub fn record_edit(
        &mut self,
        path: &Path,
        tool_name: &str,
        old_content: Option<String>,
        new_content: String,
    ) {
        let entry = FileEdit {
            timestamp: Utc::now().to_rfc3339(),
            tool_name: tool_name.to_string(),
            old_content,
            new_content,
        };
        self.edits.entry(path.to_path_buf()).or_default().push(entry);
    }

    /// Get the edit history for a specific file.
    pub fn get_history(&self, path: &Path) -> Option<&[FileEdit]> {
        self.edits.get(path).map(|v| v.as_slice())
    }

    /// Get all files that have been modified.
    pub fn modified_files(&self) -> Vec<&Path> {
        self.edits.keys().map(|p| p.as_path()).collect()
    }

    /// Get the total number of edits across all files.
    pub fn total_edits(&self) -> usize {
        self.edits.values().map(|v| v.len()).sum()
    }

    /// Get the most recent edit for a file.
    pub fn last_edit(&self, path: &Path) -> Option<&FileEdit> {
        self.edits.get(path).and_then(|v| v.last())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_new_history_is_empty() {
        let history = FileHistory::new();
        assert_eq!(history.total_edits(), 0);
        assert!(history.modified_files().is_empty());
    }

    #[test]
    fn test_record_edit() {
        let mut history = FileHistory::new();
        let path = Path::new("/tmp/test.rs");
        history.record_edit(path, "Write", None, "fn main() {}".to_string());

        assert_eq!(history.total_edits(), 1);
        assert_eq!(history.modified_files().len(), 1);
        assert!(history.modified_files().contains(&path));
    }

    #[test]
    fn test_get_history() {
        let mut history = FileHistory::new();
        let path = Path::new("/tmp/test.rs");
        history.record_edit(path, "Write", None, "v1".to_string());
        history.record_edit(
            path,
            "Edit",
            Some("v1".to_string()),
            "v2".to_string(),
        );

        let edits = history.get_history(path).unwrap();
        assert_eq!(edits.len(), 2);
        assert_eq!(edits[0].tool_name, "Write");
        assert_eq!(edits[1].tool_name, "Edit");
        assert!(edits[0].old_content.is_none());
        assert_eq!(edits[1].old_content.as_deref(), Some("v1"));
    }

    #[test]
    fn test_get_history_missing_file() {
        let history = FileHistory::new();
        assert!(history.get_history(Path::new("/nonexistent")).is_none());
    }

    #[test]
    fn test_last_edit() {
        let mut history = FileHistory::new();
        let path = Path::new("/tmp/test.rs");
        history.record_edit(path, "Write", None, "first".to_string());
        history.record_edit(path, "Edit", Some("first".to_string()), "second".to_string());

        let last = history.last_edit(path).unwrap();
        assert_eq!(last.tool_name, "Edit");
        assert_eq!(last.new_content, "second");
    }

    #[test]
    fn test_last_edit_missing_file() {
        let history = FileHistory::new();
        assert!(history.last_edit(Path::new("/nonexistent")).is_none());
    }

    #[test]
    fn test_multiple_files() {
        let mut history = FileHistory::new();
        let path_a = Path::new("/tmp/a.rs");
        let path_b = Path::new("/tmp/b.rs");

        history.record_edit(path_a, "Write", None, "a1".to_string());
        history.record_edit(path_b, "Write", None, "b1".to_string());
        history.record_edit(path_a, "Edit", Some("a1".to_string()), "a2".to_string());

        assert_eq!(history.total_edits(), 3);
        assert_eq!(history.modified_files().len(), 2);
        assert_eq!(history.get_history(path_a).unwrap().len(), 2);
        assert_eq!(history.get_history(path_b).unwrap().len(), 1);
    }

    #[test]
    fn test_timestamp_is_set() {
        let mut history = FileHistory::new();
        let path = Path::new("/tmp/test.rs");
        history.record_edit(path, "Write", None, "content".to_string());

        let edit = &history.get_history(path).unwrap()[0];
        // Timestamp should be a non-empty RFC3339 string
        assert!(!edit.timestamp.is_empty());
        assert!(edit.timestamp.contains('T'));
    }

    #[test]
    fn test_default() {
        let history = FileHistory::default();
        assert_eq!(history.total_edits(), 0);
    }

    #[test]
    fn test_file_edit_clone() {
        let edit = FileEdit {
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            tool_name: "Write".to_string(),
            old_content: None,
            new_content: "content".to_string(),
        };
        let cloned = edit.clone();
        assert_eq!(cloned.tool_name, "Write");
        assert_eq!(cloned.new_content, "content");
    }

    #[test]
    fn test_file_edit_debug() {
        let edit = FileEdit {
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            tool_name: "Write".to_string(),
            old_content: None,
            new_content: "content".to_string(),
        };
        let debug = format!("{:?}", edit);
        assert!(debug.contains("Write"));
    }
}
