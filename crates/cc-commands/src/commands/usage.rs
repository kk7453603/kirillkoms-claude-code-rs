use crate::types::*;

pub static USAGE: CommandDef = CommandDef {
    name: "usage",
    aliases: &[],
    description: "Show token usage details",
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
    async fn test_usage() {
        let result = (USAGE.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("status bar"));
    }
}
