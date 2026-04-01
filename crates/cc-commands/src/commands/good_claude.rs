use crate::types::*;

pub static GOOD_CLAUDE: CommandDef = CommandDef {
    name: "good-claude",
    aliases: &["gc"],
    description: "Give positive feedback for the last response",
    argument_hint: Some("[message]"),
    hidden: false,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            let base = "Positive feedback recorded! This helps improve future responses.";
            if args.is_empty() {
                Ok(CommandOutput::message(base))
            } else {
                Ok(CommandOutput::message(&format!(
                    "{}\n\nNote: {}",
                    base, args
                )))
            }
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_good_claude() {
        let result = (GOOD_CLAUDE.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Positive feedback"));
    }

    #[tokio::test]
    async fn test_good_claude_with_note() {
        let result = (GOOD_CLAUDE.handler)("great answer").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("great answer"));
    }
}
