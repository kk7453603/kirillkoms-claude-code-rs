use crate::types::*;

pub static RELEASE_NOTES: CommandDef = CommandDef {
    name: "release-notes",
    aliases: &["changelog"],
    description: "Generate release notes from git history",
    argument_hint: Some("[since_tag]"),
    hidden: false,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

            if !cc_utils::git::is_git_repo(&cwd).await {
                return Ok(CommandOutput::message("Not in a git repository."));
            }

            let range = if args.is_empty() {
                // Get last tag
                let tag_result = cc_utils::shell::execute_command(
                    "git",
                    &["describe", "--tags", "--abbrev=0"],
                    &cwd,
                )
                .await;
                match tag_result {
                    Ok(out) if out.exit_code == 0 => {
                        format!("{}..HEAD", out.stdout.trim())
                    }
                    _ => "HEAD~20..HEAD".to_string(),
                }
            } else {
                format!("{}..HEAD", args)
            };

            let log_result = cc_utils::shell::execute_command(
                "git",
                &["log", &range, "--pretty=format:%s (%an)", "--no-merges"],
                &cwd,
            )
            .await;

            match log_result {
                Ok(out) if !out.stdout.trim().is_empty() => {
                    let mut lines = vec![format!("Commits in {}:", range), String::new()];
                    for line in out.stdout.lines().take(50) {
                        lines.push(format!("  - {}", line));
                    }
                    lines.push(String::new());
                    lines.push("Ask the AI to format these as proper release notes.".to_string());
                    Ok(CommandOutput::message(&lines.join("\n")))
                }
                Ok(_) => Ok(CommandOutput::message(&format!(
                    "No commits found in range: {}",
                    range
                ))),
                Err(e) => Ok(CommandOutput::message(&format!(
                    "Failed to read git log: {}",
                    e
                ))),
            }
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_release_notes_runs() {
        let result = (RELEASE_NOTES.handler)("").await.unwrap();
        assert!(result.should_continue);
        assert!(result.message.is_some());
    }
}
