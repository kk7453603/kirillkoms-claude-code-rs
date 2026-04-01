use crate::types::*;

pub static STATUSLINE: CommandDef = CommandDef {
    name: "statusline",
    aliases: &[],
    description: "Set up Claude Code's status line UI",
    argument_hint: Some("[on|off]"),
    hidden: false,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            match args.as_str() {
                "on" => Ok(CommandOutput::message(
                    "Status line enabled.\n\
                     The status line will show model, token usage, and session info.",
                )),
                "off" => Ok(CommandOutput::message("Status line disabled.")),
                "" => Ok(CommandOutput::message(
                    "Status line configuration.\n\n\
                     The status line shows real-time information at the bottom of the terminal:\n  \
                     - Current model\n  \
                     - Token usage\n  \
                     - Session name\n  \
                     - Active tools\n\n\
                     Usage: /statusline [on|off]",
                )),
                _ => Ok(CommandOutput::message(
                    "Unknown option. Usage: /statusline [on|off]",
                )),
            }
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_statusline_show() {
        let result = (STATUSLINE.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Status line"));
    }

    #[tokio::test]
    async fn test_statusline_on() {
        let result = (STATUSLINE.handler)("on").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("enabled"));
    }
}
