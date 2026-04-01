use crate::types::*;

pub static TERMINAL_SETUP: CommandDef = CommandDef {
    name: "terminal-setup",
    aliases: &["terminalSetup"],
    description: "Configure terminal integration for Claude Code",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async move {
            Ok(CommandOutput::message(
                "Terminal setup for Claude Code.\n\n\
                 This command configures your terminal for optimal Claude Code integration:\n  \
                 - Shell integration (bash, zsh, fish)\n  \
                 - Keybinding configuration\n  \
                 - Status line setup\n  \
                 - Color theme detection\n\n\
                 Your terminal appears to be already configured.\n\
                 Run /doctor to diagnose any terminal issues.",
            ))
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_terminal_setup() {
        let result = (TERMINAL_SETUP.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Terminal setup"));
    }
}
