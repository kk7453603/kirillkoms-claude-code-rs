use crate::types::*;

pub static BRANCH: CommandDef = CommandDef {
    name: "branch",
    aliases: &[],
    description: "Create or switch git branch",
    argument_hint: Some("[branch_name]"),
    hidden: false,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

            if !cc_utils::git::is_git_repo(&cwd).await {
                return Ok(CommandOutput::message("Not in a git repository."));
            }

            if args.is_empty() {
                // Show current branch and list branches
                let current = cc_utils::git::current_branch(&cwd)
                    .await
                    .unwrap_or_else(|_| "unknown".to_string());

                let branches = cc_utils::shell::execute_command(
                    "git",
                    &["branch", "--list", "--no-color"],
                    &cwd,
                )
                .await;

                let mut lines = vec![format!("Current branch: {}", current)];
                if let Ok(out) = branches {
                    if !out.stdout.trim().is_empty() {
                        lines.push(String::new());
                        lines.push("Local branches:".to_string());
                        for line in out.stdout.lines() {
                            lines.push(format!("  {}", line.trim()));
                        }
                    }
                }
                lines.push(String::new());
                lines.push("Switch branch: /branch <name>".to_string());
                return Ok(CommandOutput::message(&lines.join("\n")));
            }

            // Check if branch exists
            let check =
                cc_utils::shell::execute_command("git", &["rev-parse", "--verify", &args], &cwd)
                    .await;

            if check.is_ok() && check.as_ref().unwrap().exit_code == 0 {
                // Switch to existing branch
                let result =
                    cc_utils::shell::execute_command("git", &["checkout", &args], &cwd).await;
                match result {
                    Ok(out) if out.exit_code == 0 => Ok(CommandOutput::message(&format!(
                        "Switched to branch '{}'",
                        args
                    ))),
                    Ok(out) => Ok(CommandOutput::message(&format!(
                        "Failed to switch branch: {}",
                        out.stderr.trim()
                    ))),
                    Err(e) => Ok(CommandOutput::message(&format!(
                        "Failed to switch branch: {}",
                        e
                    ))),
                }
            } else {
                // Create new branch
                let result =
                    cc_utils::shell::execute_command("git", &["checkout", "-b", &args], &cwd).await;
                match result {
                    Ok(out) if out.exit_code == 0 => Ok(CommandOutput::message(&format!(
                        "Created and switched to new branch '{}'",
                        args
                    ))),
                    Ok(out) => Ok(CommandOutput::message(&format!(
                        "Failed to create branch: {}",
                        out.stderr.trim()
                    ))),
                    Err(e) => Ok(CommandOutput::message(&format!(
                        "Failed to create branch: {}",
                        e
                    ))),
                }
            }
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_branch_no_args() {
        let result = (BRANCH.handler)("").await.unwrap();
        assert!(result.should_continue);
        assert!(result.message.is_some());
    }
}
