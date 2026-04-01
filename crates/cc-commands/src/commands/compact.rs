use crate::types::*;

pub static COMPACT: CommandDef = CommandDef {
    name: "compact",
    aliases: &[],
    description: "Compact conversation to save context",
    argument_hint: Some("[instructions]"),
    hidden: false,
    handler: |args| {
        let instructions = args.trim().to_string();
        Box::pin(async move {
            if instructions.is_empty() {
                Ok(CommandOutput::message(
                    "Compacting conversation...\n\
                     The conversation history will be summarized to free up context space.\n\
                     You can provide custom instructions: /compact <instructions>",
                ))
            } else {
                Ok(CommandOutput::message(&format!(
                    "Compacting conversation with instructions: {}\n\
                     The AI will summarize the conversation focusing on the specified areas.",
                    instructions
                )))
            }
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_compact_no_args() {
        let result = (COMPACT.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Compacting conversation"));
        assert!(result.should_continue);
    }

    #[tokio::test]
    async fn test_compact_with_instructions() {
        let result = (COMPACT.handler)("focus on the API changes").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("focus on the API changes"));
    }
}
