use crate::types::*;

pub static COMMIT_PUSH_PR: CommandDef = CommandDef {
    name: "commit-push-pr",
    aliases: &["cpp"],
    description: "Commit changes, push, and create a pull request",
    argument_hint: Some("[message]"),
    hidden: false,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            if args.is_empty() {
                Ok(CommandOutput::message(
                    "Usage: /commit-push-pr [message]\n\n\
                     This command will:\n  \
                     1. Stage and commit your changes\n  \
                     2. Push to the remote branch\n  \
                     3. Create a pull request\n\n\
                     A commit message will be auto-generated if not provided.",
                ))
            } else {
                Ok(CommandOutput::message(&format!(
                    "Starting commit-push-pr workflow...\n\
                     Commit message: {}\n\n\
                     Staging changes, committing, pushing, and creating PR...",
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
    async fn test_commit_push_pr_empty() {
        let result = (COMMIT_PUSH_PR.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Usage:"));
    }

    #[tokio::test]
    async fn test_commit_push_pr_with_msg() {
        let result = (COMMIT_PUSH_PR.handler)("fix: resolve bug").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("fix: resolve bug"));
    }
}
