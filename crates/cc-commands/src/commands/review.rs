use crate::types::*;

pub static REVIEW: CommandDef = CommandDef {
    name: "review",
    aliases: &["pr"],
    description: "Review a pull request",
    argument_hint: Some("[pr_number_or_url]"),
    hidden: false,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            if args.is_empty() {
                return Ok(CommandOutput::message(
                    "Usage: /review <pr_number_or_url>\n\n\
                     Examples:\n  \
                     /review 123\n  \
                     /review https://github.com/owner/repo/pull/123\n\n\
                     This will trigger an AI-powered code review of the pull request.",
                ));
            }

            let cwd =
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

            // Check if gh CLI is available
            let gh_check = cc_utils::shell::execute_command(
                "gh",
                &["--version"],
                &cwd,
            )
            .await;

            if gh_check.is_err() || gh_check.as_ref().is_ok_and(|o| o.exit_code != 0) {
                return Ok(CommandOutput::message(
                    "GitHub CLI (gh) not found. Install it from https://cli.github.com/\n\
                     Alternatively, provide a PR URL and the AI will fetch it via the API.",
                ));
            }

            // Fetch PR info
            let pr_ref = if args.starts_with("http") {
                args.clone()
            } else {
                args.clone()
            };

            let result = cc_utils::shell::execute_command(
                "gh",
                &["pr", "view", &pr_ref, "--json", "title,body,state,additions,deletions,changedFiles"],
                &cwd,
            )
            .await;

            match result {
                Ok(out) if out.exit_code == 0 => {
                    if let Ok(pr) = serde_json::from_str::<serde_json::Value>(&out.stdout) {
                        let title = pr["title"].as_str().unwrap_or("unknown");
                        let state = pr["state"].as_str().unwrap_or("unknown");
                        let additions = pr["additions"].as_u64().unwrap_or(0);
                        let deletions = pr["deletions"].as_u64().unwrap_or(0);
                        let changed = pr["changedFiles"].as_u64().unwrap_or(0);

                        Ok(CommandOutput::message(&format!(
                            "PR #{}: {}\n\
                             State: {} | +{} -{} | {} files changed\n\n\
                             Ask the AI to review this PR for a detailed analysis.",
                            pr_ref, title, state, additions, deletions, changed
                        )))
                    } else {
                        Ok(CommandOutput::message(&format!(
                            "Reviewing PR #{}...\n\
                             Ask the AI to review this PR for a detailed analysis.",
                            pr_ref
                        )))
                    }
                }
                Ok(out) => Ok(CommandOutput::message(&format!(
                    "Failed to fetch PR: {}",
                    out.stderr.trim()
                ))),
                Err(e) => Ok(CommandOutput::message(&format!(
                    "Failed to fetch PR: {}",
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
    async fn test_review_no_args() {
        let result = (REVIEW.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Usage:"));
        assert!(result.should_continue);
    }
}
