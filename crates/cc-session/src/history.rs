use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub session_id: String,
    pub timestamp: String,
    pub prompt: String,
    pub project_root: Option<String>,
}

/// Append a history entry to the global history JSONL file.
///
/// Creates the file and parent directories if they don't exist.
pub fn append_history(history_path: &Path, entry: &HistoryEntry) -> Result<(), std::io::Error> {
    if let Some(parent) = history_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(history_path)?;

    let json = serde_json::to_string(entry)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    writeln!(file, "{}", json)?;
    Ok(())
}

/// Read recent history entries, returning up to `count` most recent entries.
///
/// Entries are returned in chronological order (oldest first).
pub fn read_recent_history(
    history_path: &Path,
    count: usize,
) -> Result<Vec<HistoryEntry>, std::io::Error> {
    if !history_path.exists() {
        return Ok(vec![]);
    }

    let content = std::fs::read_to_string(history_path)?;
    let mut entries = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(entry) = serde_json::from_str::<HistoryEntry>(trimmed) {
            entries.push(entry);
        }
    }

    // Return only the last `count` entries
    let start = entries.len().saturating_sub(count);
    Ok(entries[start..].to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(id: &str, prompt: &str) -> HistoryEntry {
        HistoryEntry {
            session_id: id.to_string(),
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            prompt: prompt.to_string(),
            project_root: None,
        }
    }

    #[test]
    fn test_append_and_read_history() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.jsonl");

        append_history(&path, &make_entry("s1", "hello")).unwrap();
        append_history(&path, &make_entry("s2", "world")).unwrap();

        let entries = read_recent_history(&path, 10).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].session_id, "s1");
        assert_eq!(entries[1].session_id, "s2");
    }

    #[test]
    fn test_read_recent_history_limit() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.jsonl");

        for i in 0..5 {
            append_history(&path, &make_entry(&format!("s{}", i), "prompt")).unwrap();
        }

        let entries = read_recent_history(&path, 2).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].session_id, "s3");
        assert_eq!(entries[1].session_id, "s4");
    }

    #[test]
    fn test_read_recent_history_nonexistent() {
        let entries = read_recent_history(Path::new("/nonexistent/history.jsonl"), 10).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_read_recent_history_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.jsonl");
        std::fs::write(&path, "").unwrap();

        let entries = read_recent_history(&path, 10).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_history_entry_serialization() {
        let entry = HistoryEntry {
            session_id: "abc".to_string(),
            timestamp: "2025-06-01T12:00:00Z".to_string(),
            prompt: "test prompt".to_string(),
            project_root: Some("/home/user".to_string()),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let deser: HistoryEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.session_id, "abc");
        assert_eq!(deser.project_root, Some("/home/user".to_string()));
    }

    #[test]
    fn test_read_history_skips_bad_lines() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mixed.jsonl");

        let good = make_entry("s1", "hello");
        let json = serde_json::to_string(&good).unwrap();
        std::fs::write(&path, format!("{}\nnot json\n{}\n", json, json)).unwrap();

        let entries = read_recent_history(&path, 10).unwrap();
        assert_eq!(entries.len(), 2); // bad line skipped
    }
}
