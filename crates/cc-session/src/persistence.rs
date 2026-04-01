use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptEntry {
    pub timestamp: String,
    #[serde(rename = "type")]
    pub entry_type: String,
    pub data: serde_json::Value,
}

#[derive(Debug, thiserror::Error)]
pub enum PersistenceError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error at line {line}: {message}")]
    Parse { line: usize, message: String },
    #[error("Session not found: {0}")]
    NotFound(String),
}

/// Append a transcript entry to the JSONL file.
///
/// Creates parent directories and the file if they don't exist.
pub fn append_entry(path: &Path, entry: &TranscriptEntry) -> Result<(), std::io::Error> {
    use std::io::Write;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;

    let json = serde_json::to_string(entry)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    writeln!(file, "{}", json)?;
    Ok(())
}

/// Read all entries from a transcript JSONL file.
pub fn read_entries(path: &Path) -> Result<Vec<TranscriptEntry>, PersistenceError> {
    if !path.exists() {
        return Err(PersistenceError::NotFound(path.display().to_string()));
    }

    let content = std::fs::read_to_string(path)?;
    let mut entries = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let entry: TranscriptEntry =
            serde_json::from_str(trimmed).map_err(|e| PersistenceError::Parse {
                line: line_num + 1,
                message: e.to_string(),
            })?;
        entries.push(entry);
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(entry_type: &str, data: serde_json::Value) -> TranscriptEntry {
        TranscriptEntry {
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            entry_type: entry_type.to_string(),
            data,
        }
    }

    #[test]
    fn test_append_and_read_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session1").join("transcript.jsonl");

        let entry1 = make_entry("user_message", serde_json::json!({"text": "hello"}));
        let entry2 = make_entry("assistant_message", serde_json::json!({"text": "hi"}));

        append_entry(&path, &entry1).unwrap();
        append_entry(&path, &entry2).unwrap();

        let entries = read_entries(&path).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].entry_type, "user_message");
        assert_eq!(entries[1].entry_type, "assistant_message");
        assert_eq!(entries[0].data["text"], "hello");
        assert_eq!(entries[1].data["text"], "hi");
    }

    #[test]
    fn test_read_nonexistent_file() {
        let result = read_entries(Path::new("/nonexistent/transcript.jsonl"));
        assert!(matches!(result, Err(PersistenceError::NotFound(_))));
    }

    #[test]
    fn test_read_invalid_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.jsonl");
        std::fs::write(&path, "not json\n").unwrap();

        let result = read_entries(&path);
        assert!(matches!(
            result,
            Err(PersistenceError::Parse { line: 1, .. })
        ));
    }

    #[test]
    fn test_read_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.jsonl");
        std::fs::write(&path, "").unwrap();

        // Empty file exists but no entries - not "not found"
        // However our read_entries will return empty vec since lines are empty
        // Actually the file exists so it won't be NotFound
        let entries = read_entries(&path).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_read_file_with_blank_lines() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("blanks.jsonl");

        let entry = make_entry("test", serde_json::json!({}));
        let json = serde_json::to_string(&entry).unwrap();
        std::fs::write(&path, format!("{}\n\n{}\n", json, json)).unwrap();

        let entries = read_entries(&path).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_transcript_entry_serialization() {
        let entry = make_entry("msg", serde_json::json!({"key": "value"}));
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"type\":\"msg\""));

        let deser: TranscriptEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.entry_type, "msg");
    }

    #[test]
    fn test_persistence_error_display() {
        let e = PersistenceError::NotFound("abc".to_string());
        assert_eq!(e.to_string(), "Session not found: abc");

        let e = PersistenceError::Parse {
            line: 5,
            message: "unexpected token".to_string(),
        };
        assert!(e.to_string().contains("line 5"));
    }
}
