use crate::types::*;

pub static CONTEXT: CommandDef = CommandDef {
    name: "context",
    aliases: &["ctx"],
    description: "Show context information",
    argument_hint: Some("[show]"),
    hidden: false,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            let cwd =
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

            match args.as_str() {
                "" | "show" => {
                    let mut lines = vec!["Context Information".to_string(), String::new()];

                    // Working directory
                    lines.push(format!("  Working directory: {}", cwd.display()));

                    // CLAUDE.md files
                    let claude_files =
                        cc_config::claude_md::discover_claude_md_files(&cwd);
                    if claude_files.is_empty() {
                        lines.push("  CLAUDE.md files:   none".to_string());
                    } else {
                        lines.push(format!(
                            "  CLAUDE.md files:   {}",
                            claude_files.len()
                        ));
                        for f in &claude_files {
                            let size = std::fs::metadata(f)
                                .map(|m| cc_utils::format::format_bytes(m.len()))
                                .unwrap_or_else(|_| "?".to_string());
                            lines.push(format!("    {} ({})", f.display(), size));
                        }
                    }

                    // Model info
                    let model = cc_config::model_config::default_model();
                    if let Some(cfg) = cc_config::model_config::get_model_config(model) {
                        lines.push(format!(
                            "  Context window:    {} tokens",
                            cc_utils::format::format_tokens(cfg.context_window as u64)
                        ));
                        lines.push(format!(
                            "  Max output:        {} tokens",
                            cc_utils::format::format_tokens(cfg.max_output_tokens as u64)
                        ));
                    }

                    // Git info
                    if cc_utils::git::is_git_repo(&cwd).await {
                        if let Ok(root) = cc_utils::git::repo_root(&cwd).await {
                            lines.push(format!(
                                "  Git root:          {}",
                                root.display()
                            ));
                        }
                    }

                    Ok(CommandOutput::message(&lines.join("\n")))
                }
                _ => Ok(CommandOutput::message(
                    "Usage: /context [show]\nShows information about the current context.",
                )),
            }
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_context_show() {
        let result = (CONTEXT.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Context Information"));
        assert!(msg.contains("Working directory:"));
        assert!(result.should_continue);
    }
}
