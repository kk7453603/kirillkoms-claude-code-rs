use crate::types::*;

pub static ADVISOR: CommandDef = CommandDef {
    name: "advisor",
    aliases: &[],
    description: "Get advice on how to approach a coding task",
    argument_hint: Some("<question>"),
    hidden: false,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            if args.is_empty() {
                Ok(CommandOutput::message(
                    "Usage: /advisor <question>\n\n\
                     Ask for advice on how to approach a coding task.\n\
                     The advisor will suggest strategies without making changes.\n\n\
                     Examples:\n  \
                     /advisor how should I structure this API?\n  \
                     /advisor what's the best way to add caching here?",
                ))
            } else {
                Ok(CommandOutput::message(&format!(
                    "Advisor mode activated for: {}\n\n\
                     Analyzing your question and preparing recommendations...\n\
                     (Advisor provides suggestions only - no files will be modified.)",
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
    async fn test_advisor_empty() {
        let result = (ADVISOR.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Usage:"));
    }

    #[tokio::test]
    async fn test_advisor_with_question() {
        let result = (ADVISOR.handler)("how to add caching").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Advisor mode"));
    }
}
