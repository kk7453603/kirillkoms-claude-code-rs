use crate::types::*;

pub static SECURITY_REVIEW: CommandDef = CommandDef {
    name: "security-review",
    aliases: &[],
    description: "Complete a security review of pending changes on the current branch",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async move {
            Ok(CommandOutput::message(
                "Starting security review of current branch changes...\n\n\
                 This will analyze:\n  \
                 - Input validation and sanitization\n  \
                 - Authentication and authorization\n  \
                 - Secrets and credential handling\n  \
                 - SQL injection and XSS vectors\n  \
                 - Dependency vulnerabilities\n  \
                 - File system access patterns\n\n\
                 Reviewing changes against the default branch...",
            ))
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_security_review() {
        let result = (SECURITY_REVIEW.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("security review"));
    }
}
