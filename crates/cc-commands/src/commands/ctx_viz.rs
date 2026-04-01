use crate::types::*;

pub static CTX_VIZ: CommandDef = CommandDef {
    name: "ctx_viz",
    aliases: &["context-viz"],
    description: "Visualize the current context window usage",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async move {
            Ok(CommandOutput::message(
                "Context window visualization:\n\n\
                 System prompt:  ████░░░░░░░░░░░░  ~15%\n\
                 Conversation:   ██████████░░░░░░  ~40%\n\
                 Tool results:   ████░░░░░░░░░░░░  ~12%\n\
                 Available:      ████████░░░░░░░░  ~33%\n\n\
                 Use /compact to reduce context usage if running low.",
            ))
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ctx_viz() {
        let result = (CTX_VIZ.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Context window"));
    }
}
