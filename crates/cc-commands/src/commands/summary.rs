use crate::types::*;

pub static SUMMARY: CommandDef = CommandDef {
    name: "summary",
    aliases: &[],
    description: "Summarize the current conversation",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async move {
            Ok(CommandOutput::message(
                "Generating conversation summary...\n\n\
                 This will produce a concise overview of the topics discussed,\n\
                 decisions made, and actions taken in the current session.",
            ))
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_summary() {
        let result = (SUMMARY.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("summary"));
    }
}
