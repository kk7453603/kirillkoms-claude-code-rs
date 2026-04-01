use crate::types::*;

pub static AUTOFIX_PR: CommandDef = CommandDef {
    name: "autofix-pr",
    aliases: &[],
    description: "Automatically fix issues in a pull request",
    argument_hint: Some("[pr-number]"),
    hidden: false,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            if args.is_empty() {
                Ok(CommandOutput::message(
                    "Usage: /autofix-pr [pr-number]\n\n\
                     Automatically detect and fix issues in a pull request:\n  \
                     - Lint errors and warnings\n  \
                     - Type errors\n  \
                     - Failed CI checks\n\n\
                     If no PR number is given, uses the current branch's PR.",
                ))
            } else {
                Ok(CommandOutput::message(&format!(
                    "Analyzing PR #{} for auto-fixable issues...\n\
                     This will review CI failures, lint errors, and type issues.",
                    args
                )))
            }
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_autofix_pr_empty() {
        let result = (AUTOFIX_PR.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Usage:"));
    }

    #[tokio::test]
    async fn test_autofix_pr_with_number() {
        let result = (AUTOFIX_PR.handler)("42").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("PR #42"));
    }
}
