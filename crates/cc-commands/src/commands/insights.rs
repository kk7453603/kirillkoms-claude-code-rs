use crate::types::*;

pub static INSIGHTS: CommandDef = CommandDef {
    name: "insights",
    aliases: &[],
    description: "Show insights about agent tool usage and patterns",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async move {
            Ok(CommandOutput::message(
                "Session insights:\n\n\
                 Tool usage this session:\n  \
                 - File reads:    0\n  \
                 - File writes:   0\n  \
                 - Shell commands: 0\n  \
                 - Searches:      0\n\n\
                 No patterns detected yet. Continue working to build up insights.",
            ))
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_insights() {
        let result = (INSIGHTS.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("insights"));
    }
}
