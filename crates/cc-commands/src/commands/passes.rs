use crate::types::*;

pub static PASSES: CommandDef = CommandDef {
    name: "passes",
    aliases: &[],
    description: "Show available Claude Code passes and subscription info",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async move {
            Ok(CommandOutput::message(
                "Claude Code passes:\n\n\
                 Passes provide additional usage beyond the standard limits.\n\
                 Check your current plan and available passes at:\n  \
                 https://claude.ai/settings/billing\n\n\
                 Use /usage to see your current usage statistics.",
            ))
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_passes() {
        let result = (PASSES.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("passes"));
    }
}
