use crate::persistence::{self, PersistenceError};
use crate::storage;

/// Resume data needed to restore a session.
#[derive(Debug, Clone)]
pub struct ResumeData {
    pub session_id: String,
    pub messages: Vec<serde_json::Value>,
    pub project_root: Option<String>,
}

/// Load resume data from a session transcript.
///
/// Reads the transcript JSONL file and reconstructs the messages and
/// project root from the stored entries.
pub fn load_resume_data(
    sessions_root: &std::path::Path,
    session_id: &str,
) -> Result<ResumeData, PersistenceError> {
    let path = storage::transcript_path(sessions_root, session_id);
    let entries = persistence::read_entries(&path)?;

    let mut messages = Vec::new();
    let mut project_root = None;

    for entry in &entries {
        match entry.entry_type.as_str() {
            "user_message" | "assistant_message" => {
                let mut msg = entry.data.clone();
                if let Some(obj) = msg.as_object_mut() {
                    obj.insert(
                        "_role".to_string(),
                        serde_json::Value::String(entry.entry_type.clone()),
                    );
                }
                messages.push(msg);
            }
            "session_start" => {
                if let Some(root) = entry.data.get("project_root").and_then(|v| v.as_str()) {
                    project_root = Some(root.to_string());
                }
            }
            _ => {
                // Other entry types are ignored for resume
            }
        }
    }

    Ok(ResumeData {
        session_id: session_id.to_string(),
        messages,
        project_root,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::{TranscriptEntry, append_entry};

    fn make_entry(entry_type: &str, data: serde_json::Value) -> TranscriptEntry {
        TranscriptEntry {
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            entry_type: entry_type.to_string(),
            data,
        }
    }

    #[test]
    fn test_load_resume_data() {
        let dir = tempfile::tempdir().unwrap();
        let sid = "test-session";
        let path = storage::transcript_path(dir.path(), sid);

        append_entry(
            &path,
            &make_entry(
                "session_start",
                serde_json::json!({"project_root": "/home/user/project"}),
            ),
        )
        .unwrap();
        append_entry(
            &path,
            &make_entry("user_message", serde_json::json!({"text": "hello"})),
        )
        .unwrap();
        append_entry(
            &path,
            &make_entry("assistant_message", serde_json::json!({"text": "hi"})),
        )
        .unwrap();
        append_entry(
            &path,
            &make_entry("tool_use", serde_json::json!({"tool": "read_file"})),
        )
        .unwrap();

        let data = load_resume_data(dir.path(), sid).unwrap();
        assert_eq!(data.session_id, sid);
        assert_eq!(data.messages.len(), 2); // only user + assistant messages
        assert_eq!(data.project_root, Some("/home/user/project".to_string()));
    }

    #[test]
    fn test_load_resume_data_no_project_root() {
        let dir = tempfile::tempdir().unwrap();
        let sid = "no-root";
        let path = storage::transcript_path(dir.path(), sid);

        append_entry(
            &path,
            &make_entry("user_message", serde_json::json!({"text": "hello"})),
        )
        .unwrap();

        let data = load_resume_data(dir.path(), sid).unwrap();
        assert!(data.project_root.is_none());
        assert_eq!(data.messages.len(), 1);
    }

    #[test]
    fn test_load_resume_data_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let result = load_resume_data(dir.path(), "nonexistent");
        assert!(matches!(result, Err(PersistenceError::NotFound(_))));
    }

    #[test]
    fn test_load_resume_data_empty_session() {
        let dir = tempfile::tempdir().unwrap();
        let sid = "empty";
        let path = storage::transcript_path(dir.path(), sid);
        // Create an empty file
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "").unwrap();

        let data = load_resume_data(dir.path(), sid).unwrap();
        assert!(data.messages.is_empty());
        assert!(data.project_root.is_none());
    }
}
