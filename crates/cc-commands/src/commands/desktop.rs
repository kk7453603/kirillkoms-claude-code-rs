use crate::types::*;

pub static DESKTOP: CommandDef = CommandDef {
    name: "desktop",
    aliases: &[],
    description: "Open Claude Code in the desktop app",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async move {
            Ok(CommandOutput::message(
                "Claude Code Desktop App.\n\n\
                 Open the current session in the Claude Code desktop application.\n\n\
                 The desktop app provides:\n  \
                 - Native window management\n  \
                 - System notifications\n  \
                 - Keyboard shortcuts\n\n\
                 Desktop integration is not yet available in the Rust build.",
            ))
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_desktop() {
        let result = (DESKTOP.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Desktop"));
    }
}
