use crate::types::*;

pub static FEEDBACK: CommandDef = CommandDef {
    name: "feedback",
    aliases: &[],
    description: "Send feedback to the team",
    argument_hint: Some("<message>"),
    hidden: false,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            if args.is_empty() {
                Ok(CommandOutput::message(
                    "Usage: /feedback <message>\n\n\
                     Share your thoughts, suggestions, or issues.\n\
                     Your feedback helps improve Claude Code.\n\n\
                     For bug reports, use /bug instead.",
                ))
            } else {
                Ok(CommandOutput::message(&format!(
                    "Thank you for your feedback!\n\n\
                     Message: {}\n\n\
                     Your feedback has been noted. For urgent issues,\n\
                     visit https://github.com/anthropics/claude-code/issues",
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
    async fn test_feedback_no_args() {
        let result = (FEEDBACK.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Usage:"));
    }

    #[tokio::test]
    async fn test_feedback_with_message() {
        let result = (FEEDBACK.handler)("great tool").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("great tool"));
    }
}
