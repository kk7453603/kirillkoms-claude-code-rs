use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryEntry {
    pub content: String,
    pub category: MemoryCategory,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MemoryCategory {
    ProjectStructure,
    UserPreference,
    TechnicalDecision,
    BugFix,
    Pattern,
}

/// Extract key information from conversation messages for memory.
///
/// Looks for assistant messages that contain keywords indicating memorable information
/// such as project structure notes, user preferences, technical decisions, bug fixes,
/// and recurring patterns.
pub fn extract_memories(messages: &[serde_json::Value]) -> Vec<MemoryEntry> {
    let now = chrono::Utc::now().to_rfc3339();
    let mut memories = Vec::new();

    for msg in messages {
        let role = msg.get("role").and_then(|v| v.as_str()).unwrap_or("");
        let content = msg.get("content").and_then(|v| v.as_str()).unwrap_or("");

        if role != "assistant" || content.is_empty() {
            continue;
        }

        // Look for project structure indicators
        if content.contains("project structure")
            || content.contains("directory layout")
            || content.contains("folder structure")
        {
            memories.push(MemoryEntry {
                content: extract_summary(content, 200),
                category: MemoryCategory::ProjectStructure,
                timestamp: now.clone(),
            });
        }

        // Look for user preference indicators
        if content.contains("you prefer")
            || content.contains("your preference")
            || content.contains("you like to")
        {
            memories.push(MemoryEntry {
                content: extract_summary(content, 200),
                category: MemoryCategory::UserPreference,
                timestamp: now.clone(),
            });
        }

        // Look for technical decision indicators
        if content.contains("decided to")
            || content.contains("chose to")
            || content.contains("the approach")
        {
            memories.push(MemoryEntry {
                content: extract_summary(content, 200),
                category: MemoryCategory::TechnicalDecision,
                timestamp: now.clone(),
            });
        }

        // Look for bug fix indicators
        if content.contains("fixed the bug")
            || content.contains("the fix was")
            || content.contains("root cause")
        {
            memories.push(MemoryEntry {
                content: extract_summary(content, 200),
                category: MemoryCategory::BugFix,
                timestamp: now.clone(),
            });
        }

        // Look for pattern indicators
        if content.contains("pattern")
            || content.contains("convention")
            || content.contains("best practice")
        {
            memories.push(MemoryEntry {
                content: extract_summary(content, 200),
                category: MemoryCategory::Pattern,
                timestamp: now.clone(),
            });
        }
    }

    memories
}

/// Extract the first `max_len` characters as a summary.
fn extract_summary(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        let truncated: String = text.chars().take(max_len).collect();
        format!("{}...", truncated)
    }
}

/// Save memories to a JSON file.
pub fn save_memories(memories: &[MemoryEntry], path: &Path) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(memories)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(path, json)
}

/// Load memories from a JSON file.
pub fn load_memories(path: &Path) -> Result<Vec<MemoryEntry>, std::io::Error> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = std::fs::read_to_string(path)?;
    let memories: Vec<MemoryEntry> = serde_json::from_str(&content)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    Ok(memories)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_memories_project_structure() {
        let messages = vec![json!({
            "role": "assistant",
            "content": "The project structure uses a monorepo layout with crates/"
        })];
        let memories = extract_memories(&messages);
        assert!(!memories.is_empty());
        assert_eq!(memories[0].category, MemoryCategory::ProjectStructure);
    }

    #[test]
    fn test_extract_memories_bug_fix() {
        let messages = vec![json!({
            "role": "assistant",
            "content": "I fixed the bug by correcting the off-by-one error in the loop"
        })];
        let memories = extract_memories(&messages);
        assert!(!memories.is_empty());
        assert_eq!(memories[0].category, MemoryCategory::BugFix);
    }

    #[test]
    fn test_extract_memories_ignores_user_messages() {
        let messages = vec![json!({
            "role": "user",
            "content": "The project structure is interesting"
        })];
        let memories = extract_memories(&messages);
        assert!(memories.is_empty());
    }

    #[test]
    fn test_save_and_load_memories() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("memories.json");

        let memories = vec![
            MemoryEntry {
                content: "Uses workspace layout".to_string(),
                category: MemoryCategory::ProjectStructure,
                timestamp: "2025-01-01T00:00:00Z".to_string(),
            },
            MemoryEntry {
                content: "Prefers explicit error types".to_string(),
                category: MemoryCategory::UserPreference,
                timestamp: "2025-01-01T00:00:00Z".to_string(),
            },
        ];

        save_memories(&memories, &path).unwrap();
        let loaded = load_memories(&path).unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].content, "Uses workspace layout");
        assert_eq!(loaded[1].category, MemoryCategory::UserPreference);
    }

    #[test]
    fn test_load_memories_nonexistent_file() {
        let result = load_memories(Path::new("/nonexistent/memories.json")).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_extract_summary_truncation() {
        let long_text = "a".repeat(300);
        let summary = extract_summary(&long_text, 200);
        assert_eq!(summary.len(), 203); // 200 + "..."
        assert!(summary.ends_with("..."));
    }
}
