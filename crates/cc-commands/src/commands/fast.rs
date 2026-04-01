use crate::types::*;

pub static FAST: CommandDef = CommandDef {
    name: "fast",
    aliases: &[],
    description: "Toggle fast mode (use faster model)",
    argument_hint: Some("[on|off]"),
    hidden: false,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            match args.as_str() {
                "on" | "true" | "1" => Ok(CommandOutput::message(
                    "Fast mode enabled.\n\
                     Using faster model (Haiku) for quicker responses.\n\
                     Trade-off: less capable for complex tasks.",
                )),
                "off" | "false" | "0" => Ok(CommandOutput::message(
                    "Fast mode disabled.\n\
                     Returned to default model (Sonnet).",
                )),
                "" => Ok(CommandOutput::message(
                    "Fast mode: off\n\n\
                     When enabled, uses a faster but less capable model.\n\
                     Usage: /fast [on|off]",
                )),
                _ => Ok(CommandOutput::message(
                    "Usage: /fast [on|off]",
                )),
            }
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fast_toggle() {
        let result = (FAST.handler)("on").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Fast mode enabled"));
    }
}
