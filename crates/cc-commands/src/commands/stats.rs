use crate::types::*;

pub static STATS: CommandDef = CommandDef {
    name: "stats",
    aliases: &[],
    description: "Show session statistics",
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
    async fn test_stats() {
        let result = (STATS.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("status bar"));
    }
}
