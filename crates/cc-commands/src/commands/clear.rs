use crate::types::*;

pub static CLEAR: CommandDef = CommandDef {
    name: "clear",
    aliases: &[],
    description: "Clear conversation history",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async {
            Ok(CommandOutput::message("Conversation cleared."))
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_clear() {
        let result = (CLEAR.handler)("").await.unwrap();
        assert_eq!(result.message.as_deref(), Some("Conversation cleared."));
        assert!(result.should_continue);
    }
}
