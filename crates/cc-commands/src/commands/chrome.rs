use crate::types::*;

pub static CHROME: CommandDef = CommandDef {
    name: "chrome",
    aliases: &[],
    description: "Open Claude in Chrome with MCP integration",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async move {
            Ok(CommandOutput::message(
                "Claude in Chrome.\n\n\
                 This command opens Claude in Chrome with MCP server integration,\n\
                 allowing Claude to interact with web pages.\n\n\
                 Prerequisites:\n  \
                 - Chrome browser installed\n  \
                 - Claude Code Chrome extension\n\n\
                 The Chrome integration is not yet available in the Rust build.",
            ))
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_chrome() {
        let result = (CHROME.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Chrome"));
    }
}
