use crate::types::*;

pub static MEMORY: CommandDef = CommandDef {
    name: "memory",
    aliases: &[],
    description: "View or edit CLAUDE.md memory files",
    argument_hint: Some("[view|edit|path]"),
    hidden: false,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            let cwd =
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
            let files = cc_config::claude_md::discover_claude_md_files(&cwd);

            match args.as_str() {
                "" | "view" => {
                    if files.is_empty() {
                        return Ok(CommandOutput::message(
                            "No CLAUDE.md files found.\n\
                             Create one with: /memory edit\n\
                             Locations checked:\n  \
                             .claude/CLAUDE.md\n  \
                             CLAUDE.md\n  \
                             ~/.claude/CLAUDE.md",
                        ));
                    }

                    let mut lines = vec![format!("Found {} CLAUDE.md file(s):", files.len())];
                    for path in &files {
                        lines.push(format!("\n--- {} ---", path.display()));
                        match std::fs::read_to_string(path) {
                            Ok(content) => {
                                let preview = if content.len() > 2000 {
                                    format!(
                                        "{}...\n(truncated, {} bytes total)",
                                        &content[..2000],
                                        content.len()
                                    )
                                } else {
                                    content
                                };
                                lines.push(preview);
                            }
                            Err(e) => lines.push(format!("  Error reading: {}", e)),
                        }
                    }
                    Ok(CommandOutput::message(&lines.join("\n")))
                }
                "edit" => {
                    let target = cwd.join("CLAUDE.md");
                    if !target.exists() {
                        match std::fs::write(
                            &target,
                            "# Project Memory\n\n<!-- Add project-specific instructions here -->\n",
                        ) {
                            Ok(()) => Ok(CommandOutput::message(&format!(
                                "Created {}. Edit it to add project instructions.",
                                target.display()
                            ))),
                            Err(e) => Ok(CommandOutput::message(&format!(
                                "Failed to create CLAUDE.md: {}",
                                e
                            ))),
                        }
                    } else {
                        Ok(CommandOutput::message(&format!(
                            "CLAUDE.md already exists at {}. \
                             Edit it directly to update project instructions.",
                            target.display()
                        )))
                    }
                }
                "path" => {
                    if files.is_empty() {
                        Ok(CommandOutput::message("No CLAUDE.md files found."))
                    } else {
                        let paths: Vec<String> =
                            files.iter().map(|p| p.display().to_string()).collect();
                        Ok(CommandOutput::message(&paths.join("\n")))
                    }
                }
                _ => Ok(CommandOutput::message(
                    "Usage: /memory [view|edit|path]",
                )),
            }
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_view() {
        let result = (MEMORY.handler)("").await.unwrap();
        assert!(result.should_continue);
        assert!(result.message.is_some());
    }

    #[tokio::test]
    async fn test_memory_path() {
        let result = (MEMORY.handler)("path").await.unwrap();
        assert!(result.should_continue);
    }

    #[tokio::test]
    async fn test_memory_unknown_subcommand() {
        let result = (MEMORY.handler)("badarg").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Usage:"));
    }
}
