use crate::types::*;

pub static EXTRA_USAGE: CommandDef = CommandDef {
    name: "extra-usage",
    aliases: &[],
    description: "Show extended usage information and tips",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async move {
            Ok(CommandOutput::message(
                "Extended usage tips for Claude Code:\n\n\
                 Keyboard shortcuts:\n  \
                 Ctrl+C  - Cancel current generation\n  \
                 Ctrl+D  - Exit Claude Code\n  \
                 Ctrl+L  - Clear screen\n  \
                 Tab     - Accept autocomplete suggestion\n\n\
                 Advanced features:\n  \
                 - Use /compact to reduce context when running low\n  \
                 - Use /model to switch between models mid-conversation\n  \
                 - Use /config to customize behavior\n  \
                 - Pipe input: echo 'question' | claude\n  \
                 - Use --print for non-interactive mode",
            ))
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_extra_usage() {
        let result = (EXTRA_USAGE.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Extended usage"));
    }
}
