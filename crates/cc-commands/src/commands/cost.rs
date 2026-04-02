use crate::types::*;

pub static COST: CommandDef = CommandDef {
    name: "cost",
    aliases: &[],
    description: "Show token usage and cost for this session",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async {
            Ok(CommandOutput::message(
                "Cost tracking is shown in the status bar. \
                 Session totals: check status bar at the bottom.",
            ))
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cost_shows_summary() {
        let result = (COST.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("status bar"));
        assert!(result.should_continue);
    }
}
