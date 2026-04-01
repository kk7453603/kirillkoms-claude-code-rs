use serde::{Deserialize, Serialize};
use std::path::Path;

/// A single memory entry extracted from a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub content: String,
    pub category: MemoryCategory,
    pub timestamp: String,
}

/// Category of a memory
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MemoryCategory {
    ProjectStructure,
    UserPreference,
    TechnicalDecision,
    BugFix,
    Pattern,
}

/// Extract key information from conversation messages for memory.
///
/// Scans assistant messages for patterns that indicate important information:
/// - File structure mentions
/// - User preference expressions
/// - Technical decisions
/// - Bug fixes
/// - Code patterns
pub fn extract_memories(messages: &[serde_json::Value]) -> Vec<MemoryEntry> {
    let mut memories = Vec::new();
    let timestamp = chrono::Utc::now().to_rfc3339();

    for msg in messages {
        let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("");
        let content = msg.get("content").and_then(|c| c.as_str()).unwrap_or("");

        if role != "assistant" || content.is_empty() {
            continue;
        }

        let lower = content.to_lowercase();

        // Detect project structure mentions
        if lower.contains("project structure")
            || lower.contains("directory layout")
            || lower.contains("the codebase")
        {
            memories.push(MemoryEntry {
                content: truncate_content(content, 200),
                category: MemoryCategory::ProjectStructure,
                timestamp: timestamp.clone(),
            });
        }

        // Detect user preference signals
        if lower.contains("you prefer")
            || lower.contains("your preference")
            || lower.contains("as you requested")
        {
            memories.push(MemoryEntry {
                content: truncate_content(content, 200),
                category: MemoryCategory::UserPreference,
                timestamp: timestamp.clone(),
            });
        }

        // Detect technical decisions
        if lower.contains("decided to")
            || lower.contains("we chose")
            || lower.contains("the approach")
        {
            memories.push(MemoryEntry {
                content: truncate_content(content, 200),
                category: MemoryCategory::TechnicalDecision,
                timestamp: timestamp.clone(),
            });
        }

        // Detect bug fixes
        if lower.contains("fixed the bug")
            || lower.contains("the issue was")
            || lower.contains("root cause")
        {
            memories.push(MemoryEntry {
                content: truncate_content(content, 200),
                category: MemoryCategory::BugFix,
                timestamp: timestamp.clone(),
            });
        }

        // Detect patterns
        if lower.contains("pattern")
            || lower.contains("convention")
            || lower.contains("best practice")
        {
            memories.push(MemoryEntry {
                content: truncate_content(content, 200),
                category: MemoryCategory::Pattern,
                timestamp: timestamp.clone(),
            });
        }
    }

    memories
}

/// Truncate content to max_len characters, appending "..." if truncated.
fn truncate_content(content: &str, max_len: usize) -> String {
    if content.len() <= max_len {
        content.to_string()
    } else {
        let truncated: String = content.chars().take(max_len).collect();
        format!("{}...", truncated)
    }
}

/// Save memories to a JSON file
pub fn save_memories(memories: &[MemoryEntry], path: &Path) -> Result<(), std::io::Error> {
    let json = serde_json::to_string_pretty(memories)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(path, json)
}

/// Load memories from a JSON file
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
    use tempfile::TempDir;

    #[test]
    fn extract_project_structure_memory() {
        let messages = vec![json!({
            "role": "assistant",
            "content": "The project structure has three main crates."
        })];
        let memories = extract_memories(&messages);
        assert!(!memories.is_empty());
        assert_eq!(memories[0].category, MemoryCategory::ProjectStructure);
    }

    #[test]
    fn extract_bug_fix_memory() {
        let messages = vec![json!({
            "role": "assistant",
            "content": "I fixed the bug in the parser. The root cause was a missing null check."
        })];
        let memories = extract_memories(&messages);
        assert!(memories.len() >= 1);
        assert!(
            memories
                .iter()
                .any(|m| m.category == MemoryCategory::BugFix)
        );
    }

    #[test]
    fn skips_user_messages() {
        let messages = vec![json!({
            "role": "user",
            "content": "The project structure is complex."
        })];
        let memories = extract_memories(&messages);
        assert!(memories.is_empty());
    }

    #[test]
    fn save_and_load_memories() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("memories.json");

        let memories = vec![
            MemoryEntry {
                content: "Test memory".to_string(),
                category: MemoryCategory::Pattern,
                timestamp: "2025-01-01T00:00:00Z".to_string(),
            },
            MemoryEntry {
                content: "Another memory".to_string(),
                category: MemoryCategory::UserPreference,
                timestamp: "2025-01-02T00:00:00Z".to_string(),
            },
        ];

        save_memories(&memories, &path).unwrap();
        let loaded = load_memories(&path).unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].content, "Test memory");
        assert_eq!(loaded[1].category, MemoryCategory::UserPreference);
    }

    #[test]
    fn load_memories_nonexistent_returns_empty() {
        let loaded = load_memories(Path::new("/nonexistent/memories.json")).unwrap();
        assert!(loaded.is_empty());
    }

    #[test]
    fn truncate_long_content() {
        let long = "a".repeat(300);
        let messages = vec![json!({
            "role": "assistant",
            "content": format!("The project structure: {}", long)
        })];
        let memories = extract_memories(&messages);
        assert!(!memories.is_empty());
        // Should be truncated to ~200 chars + "..."
        assert!(memories[0].content.len() <= 210);
        assert!(memories[0].content.ends_with("..."));
    }
}
