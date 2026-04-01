use std::path::Path;

/// A suggested prompt to show the user
#[derive(Debug, Clone)]
pub struct PromptSuggestion {
    pub text: String,
    pub category: SuggestionCategory,
}

/// Category of prompt suggestion
#[derive(Debug, Clone, PartialEq)]
pub enum SuggestionCategory {
    General,
    ProjectSpecific,
    RecentlyModified,
}

/// Suggest example prompts based on project context.
///
/// Inspects the project root for common files (Cargo.toml, package.json, etc.)
/// and returns relevant prompt suggestions.
pub fn suggest_prompts(project_root: &Path) -> Vec<PromptSuggestion> {
    let mut suggestions = Vec::new();

    // Always include general suggestions
    suggestions.push(PromptSuggestion {
        text: "Explain the project structure".to_string(),
        category: SuggestionCategory::General,
    });
    suggestions.push(PromptSuggestion {
        text: "Find and fix potential bugs".to_string(),
        category: SuggestionCategory::General,
    });

    // Rust project
    if project_root.join("Cargo.toml").exists() {
        suggestions.push(PromptSuggestion {
            text: "Run cargo test and fix any failures".to_string(),
            category: SuggestionCategory::ProjectSpecific,
        });
        suggestions.push(PromptSuggestion {
            text: "Add documentation to public functions".to_string(),
            category: SuggestionCategory::ProjectSpecific,
        });
    }

    // Node.js project
    if project_root.join("package.json").exists() {
        suggestions.push(PromptSuggestion {
            text: "Run npm test and fix any failures".to_string(),
            category: SuggestionCategory::ProjectSpecific,
        });
        suggestions.push(PromptSuggestion {
            text: "Check for outdated dependencies".to_string(),
            category: SuggestionCategory::ProjectSpecific,
        });
    }

    // Python project
    if project_root.join("pyproject.toml").exists()
        || project_root.join("setup.py").exists()
        || project_root.join("requirements.txt").exists()
    {
        suggestions.push(PromptSuggestion {
            text: "Run pytest and fix any failures".to_string(),
            category: SuggestionCategory::ProjectSpecific,
        });
        suggestions.push(PromptSuggestion {
            text: "Add type hints to functions".to_string(),
            category: SuggestionCategory::ProjectSpecific,
        });
    }

    // Go project
    if project_root.join("go.mod").exists() {
        suggestions.push(PromptSuggestion {
            text: "Run go test ./... and fix any failures".to_string(),
            category: SuggestionCategory::ProjectSpecific,
        });
    }

    // Docker
    if project_root.join("Dockerfile").exists() || project_root.join("docker-compose.yml").exists()
    {
        suggestions.push(PromptSuggestion {
            text: "Review the Dockerfile for best practices".to_string(),
            category: SuggestionCategory::ProjectSpecific,
        });
    }

    // Check for recently modified files
    if let Ok(entries) = std::fs::read_dir(project_root) {
        let mut recent_files: Vec<_> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file() && !e.file_name().to_string_lossy().starts_with('.'))
            .filter_map(|e| {
                e.metadata()
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .map(|t| (e.path(), t))
            })
            .collect();

        recent_files.sort_by(|a, b| b.1.cmp(&a.1));

        if let Some((path, _)) = recent_files.first() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                suggestions.push(PromptSuggestion {
                    text: format!("Review recent changes in {}", name),
                    category: SuggestionCategory::RecentlyModified,
                });
            }
        }
    }

    suggestions
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn general_suggestions_always_present() {
        let tmp = TempDir::new().unwrap();
        let suggestions = suggest_prompts(tmp.path());
        assert!(suggestions.len() >= 2);
        assert!(
            suggestions
                .iter()
                .any(|s| s.category == SuggestionCategory::General)
        );
    }

    #[test]
    fn rust_project_suggestions() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();
        let suggestions = suggest_prompts(tmp.path());
        assert!(
            suggestions.iter().any(|s| s.text.contains("cargo test")
                && s.category == SuggestionCategory::ProjectSpecific)
        );
    }

    #[test]
    fn node_project_suggestions() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("package.json"), "{}").unwrap();
        let suggestions = suggest_prompts(tmp.path());
        assert!(suggestions.iter().any(|s| s.text.contains("npm test")));
    }

    #[test]
    fn python_project_suggestions() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("requirements.txt"), "flask\n").unwrap();
        let suggestions = suggest_prompts(tmp.path());
        assert!(suggestions.iter().any(|s| s.text.contains("pytest")));
    }

    #[test]
    fn recently_modified_file_suggestion() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("main.rs"), "fn main() {}").unwrap();
        let suggestions = suggest_prompts(tmp.path());
        assert!(
            suggestions
                .iter()
                .any(|s| s.category == SuggestionCategory::RecentlyModified)
        );
    }
}
