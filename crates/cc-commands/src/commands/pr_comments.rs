use crate::types::*;

pub static PR_COMMENTS: CommandDef = CommandDef {
    name: "pr-comments",
    aliases: &["pr-review"],
    description: "Review PR comments",
    argument_hint: Some("<pr_number>"),
    hidden: false,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            if args.is_empty() {
                return Ok(CommandOutput::message(
                    "Usage: /pr-comments <pr_number>\n\
                     Fetches and displays review comments from a pull request.",
                ));
            }

            let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

            let result = cc_utils::shell::execute_command(
                "gh",
                &[
                    "pr",
                    "view",
                    &args,
                    "--json",
                    "comments,reviews",
                    "--jq",
                    ".comments | length",
                ],
                &cwd,
            )
            .await;

            match result {
                Ok(out) if out.exit_code == 0 => {
                    let count = out.stdout.trim();
                    Ok(CommandOutput::message(&format!(
                        "PR #{}: {} comment(s)\n\
                         Use 'gh pr view {} --comments' for full details.\n\
                         Or ask the AI to review the PR comments.",
                        args, count, args
                    )))
                }
                Ok(out) => Ok(CommandOutput::message(&format!(
                    "Failed to fetch PR comments: {}",
                    out.stderr.trim()
                ))),
                Err(_) => Ok(CommandOutput::message(
                    "GitHub CLI (gh) not found. Install from https://cli.github.com/",
                )),
            }
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pr_comments_no_args() {
        let result = (PR_COMMENTS.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Usage:"));
    }
}
