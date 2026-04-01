use std::path::{Path, PathBuf};

/// Get session directory path
pub fn session_dir(sessions_root: &Path, session_id: &str) -> PathBuf {
    sessions_root.join(session_id)
}

/// Get transcript file path
pub fn transcript_path(sessions_root: &Path, session_id: &str) -> PathBuf {
    session_dir(sessions_root, session_id).join("transcript.jsonl")
}

/// List all session IDs by reading subdirectory names under sessions_root.
pub fn list_sessions(sessions_root: &Path) -> Result<Vec<String>, std::io::Error> {
    let mut sessions = Vec::new();
    let entries = std::fs::read_dir(sessions_root)?;
    for entry in entries {
        let entry = entry?;
        if entry.file_type()?.is_dir()
            && let Some(name) = entry.file_name().to_str()
        {
            sessions.push(name.to_string());
        }
    }
    sessions.sort();
    Ok(sessions)
}

/// Delete a session directory and all its contents.
pub fn delete_session(sessions_root: &Path, session_id: &str) -> Result<(), std::io::Error> {
    let dir = session_dir(sessions_root, session_id);
    if dir.exists() {
        std::fs::remove_dir_all(dir)
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Session not found: {}", session_id),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_dir() {
        let root = Path::new("/tmp/sessions");
        let dir = session_dir(root, "abc-123");
        assert_eq!(dir, PathBuf::from("/tmp/sessions/abc-123"));
    }

    #[test]
    fn test_transcript_path() {
        let root = Path::new("/tmp/sessions");
        let path = transcript_path(root, "abc-123");
        assert_eq!(
            path,
            PathBuf::from("/tmp/sessions/abc-123/transcript.jsonl")
        );
    }

    #[test]
    fn test_list_sessions_empty() {
        let dir = tempfile::tempdir().unwrap();
        let sessions = list_sessions(dir.path()).unwrap();
        assert!(sessions.is_empty());
    }

    #[test]
    fn test_list_sessions_with_dirs() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("session-1")).unwrap();
        std::fs::create_dir(dir.path().join("session-2")).unwrap();
        // A file should be ignored
        std::fs::write(dir.path().join("not-a-session.txt"), "").unwrap();

        let sessions = list_sessions(dir.path()).unwrap();
        assert_eq!(sessions, vec!["session-1", "session-2"]);
    }

    #[test]
    fn test_delete_session() {
        let dir = tempfile::tempdir().unwrap();
        let sid = "to-delete";
        std::fs::create_dir(dir.path().join(sid)).unwrap();
        std::fs::write(dir.path().join(sid).join("transcript.jsonl"), "{}").unwrap();

        delete_session(dir.path(), sid).unwrap();
        assert!(!dir.path().join(sid).exists());
    }

    #[test]
    fn test_delete_session_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let result = delete_session(dir.path(), "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_list_sessions_nonexistent_dir() {
        let result = list_sessions(Path::new("/nonexistent/path/xyz"));
        assert!(result.is_err());
    }
}
