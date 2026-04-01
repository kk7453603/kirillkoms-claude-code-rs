use crate::types::*;

pub static STATUS: CommandDef = CommandDef {
    name: "status",
    aliases: &[],
    description: "Show current session status",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async {
            let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

            let mut lines = vec!["Session Status".to_string(), String::new()];

            // Model
            let model = cc_config::model_config::default_model();
            lines.push(format!("  Model:      {}", model));

            // Working directory
            lines.push(format!("  Working dir: {}", cwd.display()));

            // Git info
            if cc_utils::git::is_git_repo(&cwd).await {
                if let Ok(branch) = cc_utils::git::current_branch(&cwd).await {
                    lines.push(format!("  Git branch:  {}", branch));
                }
                if let Ok(changed) = cc_utils::git::changed_files(&cwd).await {
                    lines.push(format!("  Changed files: {}", changed.len()));
                }
            } else {
                lines.push("  Git:         not a repository".to_string());
            }

            // CLAUDE.md
            let has_memory = cc_config::claude_md::has_claude_md(&cwd);
            lines.push(format!(
                "  CLAUDE.md:   {}",
                if has_memory { "found" } else { "not found" }
            ));

            // Platform
            lines.push(format!(
                "  Platform:    {}-{}",
                std::env::consts::OS,
                std::env::consts::ARCH
            ));
            lines.push(format!("  Version:     {}", env!("CARGO_PKG_VERSION")));

            Ok(CommandOutput::message(&lines.join("\n")))
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_status_shows_info() {
        let result = (STATUS.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Session Status"));
        assert!(msg.contains("Model:"));
        assert!(msg.contains("Working dir:"));
        assert!(result.should_continue);
    }
}
