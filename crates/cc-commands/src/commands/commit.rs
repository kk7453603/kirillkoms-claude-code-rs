use crate::types::*;

pub static COMMIT: CommandDef = CommandDef {
    name: "commit",
    aliases: &[],
    description: "Create a git commit with AI-generated message",
    argument_hint: Some("[message]"),
    hidden: false,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            let cwd =
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

            if !cc_utils::git::is_git_repo(&cwd).await {
                return Ok(CommandOutput::message("Not in a git repository."));
            }

            // Check for staged changes
            let status_output = cc_utils::shell::execute_command(
                "git",
                &["diff", "--cached", "--stat"],
                &cwd,
            )
            .await;

            let has_staged = match &status_output {
                Ok(out) => out.exit_code == 0 && !out.stdout.trim().is_empty(),
                Err(_) => false,
            };

            if !has_staged {
                // Check for any changes at all
                let changed = cc_utils::git::changed_files(&cwd).await.unwrap_or_default();
                if changed.is_empty() {
                    return Ok(CommandOutput::message(
                        "No changes to commit. Working tree is clean.",
                    ));
                }
                return Ok(CommandOutput::message(
                    "No staged changes. Stage files first with 'git add <files>'.\n\
                     Unstaged changes detected - use 'git add -A' to stage all.",
                ));
            }

            if !args.is_empty() {
                // Use provided message
                let result = cc_utils::shell::execute_command(
                    "git",
                    &["commit", "-m", &args],
                    &cwd,
                )
                .await;
                match result {
                    Ok(out) if out.exit_code == 0 => {
                        Ok(CommandOutput::message(&format!(
                            "Committed with message: {}\n{}",
                            args,
                            out.stdout.trim()
                        )))
                    }
                    Ok(out) => Ok(CommandOutput::message(&format!(
                        "Commit failed: {}",
                        out.stderr.trim()
                    ))),
                    Err(e) => Ok(CommandOutput::message(&format!(
                        "Commit failed: {}",
                        e
                    ))),
                }
            } else {
                // Show staged diff for AI to generate message
                let staged = cc_utils::shell::execute_command(
                    "git",
                    &["diff", "--cached", "--stat"],
                    &cwd,
                )
                .await;
                let stat_info = staged
                    .map(|o| o.stdout)
                    .unwrap_or_else(|_| String::new());

                Ok(CommandOutput::message(&format!(
                    "Staged changes:\n{}\n\
                     Provide a commit message: /commit <message>\n\
                     Or let the AI generate one by asking it to commit these changes.",
                    stat_info.trim()
                )))
            }
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_commit_no_args() {
        let result = (COMMIT.handler)("").await.unwrap();
        assert!(result.should_continue);
        assert!(result.message.is_some());
    }
}
