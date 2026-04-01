use crate::types::*;

pub static RATE_LIMIT_OPTIONS: CommandDef = CommandDef {
    name: "rate-limit-options",
    aliases: &[],
    description: "Show options when rate limit is reached",
    argument_hint: None,
    hidden: true,
    handler: |_args| {
        Box::pin(async move {
            Ok(CommandOutput::message(
                "Rate limit reached. Options:\n\n  \
                 1. Wait for the rate limit to reset (usually a few minutes)\n  \
                 2. Switch to a different model with /model\n  \
                 3. Use /compact to reduce context size\n  \
                 4. Purchase additional passes at https://claude.ai/settings/billing\n\n\
                 Use /usage to check your current usage.",
            ))
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limit_options() {
        let result = (RATE_LIMIT_OPTIONS.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Rate limit"));
    }
}
