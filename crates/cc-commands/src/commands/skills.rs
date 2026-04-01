use crate::types::*;

pub static SKILLS: CommandDef = CommandDef {
    name: "skills",
    aliases: &[],
    description: "List available skills",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async {
            let mut lines = vec!["Available Skills:".to_string(), String::new()];
            lines.push("Bundled:".to_string());
            lines.push(
                "  /commit          Create a git commit with AI-generated message".to_string(),
            );
            lines.push("  /review-pr       Review a pull request".to_string());
            lines.push("  /simplify        Review changed code for quality".to_string());

            // Check for user-defined skills
            let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
            let skills_dir = cwd.join(".claude").join("skills");
            if skills_dir.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&skills_dir) {
                    let md_files: Vec<_> = entries
                        .flatten()
                        .filter(|e| e.path().extension().and_then(|ext| ext.to_str()) == Some("md"))
                        .collect();
                    if !md_files.is_empty() {
                        lines.push(String::new());
                        lines.push("User-defined:".to_string());
                        for entry in &md_files {
                            let name = entry
                                .path()
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("unknown")
                                .to_string();
                            lines.push(format!("  /{:<16} (user skill)", name));
                        }
                    }
                }
            }

            lines.push(String::new());
            lines.push("Add custom skills as .md files in .claude/skills/".to_string());

            Ok(CommandOutput::message(&lines.join("\n")))
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_skills_list() {
        let result = (SKILLS.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Available Skills:"));
        assert!(msg.contains("commit"));
        assert!(msg.contains("review-pr"));
    }
}
