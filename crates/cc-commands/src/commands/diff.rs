use crate::types::*;

pub static DIFF: CommandDef = CommandDef {
    name: "diff",
    aliases: &[],
    description: "Show changes made in this session",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async {
            let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

            if !cc_utils::git::is_git_repo(&cwd).await {
                return Ok(CommandOutput::message("Not in a git repository."));
            }

            match cc_utils::git::git_diff(&cwd).await {
                Ok(diff) => {
                    if diff.trim().is_empty() {
                        Ok(CommandOutput::message("No uncommitted changes."))
                    } else {
                        let added = diff
                            .lines()
                            .filter(|l| l.starts_with('+') && !l.starts_with("+++"))
                            .count();
                        let removed = diff
                            .lines()
                            .filter(|l| l.starts_with('-') && !l.starts_with("---"))
                            .count();
                        let header = format!("Changes: +{} -{}\n", added, removed);
                        let truncated = if diff.len() > 4000 {
                            format!(
                                "{}...\n(truncated, {} total bytes)",
                                &diff[..4000],
                                diff.len()
                            )
                        } else {
                            diff
                        };
                        Ok(CommandOutput::message(&format!("{}{}", header, truncated)))
                    }
                }
                Err(e) => Ok(CommandOutput::message(&format!(
                    "Failed to get diff: {}",
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
    async fn test_diff_runs() {
        let result = (DIFF.handler)("").await.unwrap();
        assert!(result.should_continue);
        assert!(result.message.is_some());
    }
}
