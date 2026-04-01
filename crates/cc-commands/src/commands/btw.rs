use crate::types::*;

pub static BTW: CommandDef = CommandDef {
    name: "btw",
    aliases: &[],
    description: "Send a side note without changing conversation context",
    argument_hint: Some("<message>"),
    hidden: false,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            if args.is_empty() {
                Ok(CommandOutput::message(
                    "Usage: /btw <message>\n\n\
                     Send a side note to Claude without changing the main conversation context.\n\
                     Useful for quick asides or corrections.",
                ))
            } else {
                Ok(CommandOutput::message(&format!(
                    "(Side note recorded: {})",
                    args
                )))
            }
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_btw_no_args() {
        let result = (BTW.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Usage:"));
    }

    #[tokio::test]
    async fn test_btw_with_message() {
        let result = (BTW.handler)("also check tests").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("also check tests"));
    }
}
