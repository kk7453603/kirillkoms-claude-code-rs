use crate::types::*;

pub static INIT: CommandDef = CommandDef {
    name: "init",
    aliases: &[],
    description: "Initialize Claude in project",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async {
            let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

            let claude_dir = cwd.join(".claude");
            let claude_md = cwd.join("CLAUDE.md");
            let settings_path = claude_dir.join("settings.json");

            let mut actions = Vec::new();

            // Create .claude directory
            if !claude_dir.exists() {
                match std::fs::create_dir_all(&claude_dir) {
                    Ok(()) => actions.push(format!("Created {}", claude_dir.display())),
                    Err(e) => {
                        return Ok(CommandOutput::message(&format!(
                            "Failed to create .claude directory: {}",
                            e
                        )));
                    }
                }
            } else {
                actions.push(format!("{} already exists", claude_dir.display()));
            }

            // Create CLAUDE.md
            if !claude_md.exists() {
                let content = format!(
                    "# {}\n\n\
                     <!-- Project-specific instructions for Claude Code -->\n\n\
                     ## Project Overview\n\n\
                     <!-- Describe your project here -->\n\n\
                     ## Coding Conventions\n\n\
                     <!-- Add coding style and convention notes -->\n\n\
                     ## Important Notes\n\n\
                     <!-- Add any important context Claude should know -->\n",
                    cwd.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Project")
                );
                match std::fs::write(&claude_md, content) {
                    Ok(()) => actions.push(format!("Created {}", claude_md.display())),
                    Err(e) => actions.push(format!("Failed to create CLAUDE.md: {}", e)),
                }
            } else {
                actions.push(format!("{} already exists", claude_md.display()));
            }

            // Create settings.json
            if !settings_path.exists() {
                let settings = "{\n  \
                    \"permissions\": {\n    \
                        \"allow\": [],\n    \
                        \"deny\": []\n  \
                    }\n\
                }\n";
                match std::fs::write(&settings_path, settings) {
                    Ok(()) => actions.push(format!("Created {}", settings_path.display())),
                    Err(e) => actions.push(format!("Failed to create settings.json: {}", e)),
                }
            } else {
                actions.push(format!("{} already exists", settings_path.display()));
            }

            // Check .gitignore for settings.local.json
            let gitignore = cwd.join(".gitignore");
            let local_pattern = "settings.local.json";
            let needs_gitignore = if gitignore.exists() {
                let content = std::fs::read_to_string(&gitignore).unwrap_or_default();
                !content.contains(local_pattern)
            } else {
                true
            };
            if needs_gitignore {
                actions.push(format!(
                    "Tip: Add '.claude/settings.local.json' to .gitignore for personal settings"
                ));
            }

            let mut lines = vec![
                "Project initialized for Claude Code:".to_string(),
                String::new(),
            ];
            for action in &actions {
                lines.push(format!("  {}", action));
            }
            lines.push(String::new());
            lines.push("Edit CLAUDE.md to add project-specific instructions.".to_string());

            Ok(CommandOutput::message(&lines.join("\n")))
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_init_runs() {
        let result = (INIT.handler)("").await.unwrap();
        assert!(result.should_continue);
        assert!(result.message.is_some());
    }
}
