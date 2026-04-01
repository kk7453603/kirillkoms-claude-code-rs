use crate::types::*;

pub static STICKERS: CommandDef = CommandDef {
    name: "stickers",
    aliases: &[],
    description: "Show available Claude Code stickers",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async move {
            Ok(CommandOutput::message(
                "Claude Code Stickers\n\n\
                 Collect stickers by using Claude Code! Available stickers:\n  \
                 - First Commit: Make your first commit with Claude\n  \
                 - Bug Squasher: Fix 10 bugs\n  \
                 - Code Reviewer: Complete 5 code reviews\n  \
                 - Power User: Use 20 different commands\n  \
                 - Night Owl: Code after midnight\n\n\
                 Visit https://claude.ai/stickers to view your collection.",
            ))
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_stickers() {
        let result = (STICKERS.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Stickers"));
    }
}
