use crate::types::*;

pub static KEYBINDINGS: CommandDef = CommandDef {
    name: "keybindings",
    aliases: &["keys"],
    description: "Show keyboard shortcuts",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async {
            let lines = vec![
                "Keyboard Shortcuts:".to_string(),
                String::new(),
                "  Ctrl+C         - Cancel current operation".to_string(),
                "  Ctrl+D         - Exit (same as /exit)".to_string(),
                "  Ctrl+L         - Clear screen".to_string(),
                "  Ctrl+R         - Search command history".to_string(),
                "  Up/Down        - Navigate command history".to_string(),
                "  Tab            - Auto-complete commands".to_string(),
                "  Shift+Enter    - New line (multi-line input)".to_string(),
                "  Esc            - Cancel current input".to_string(),
                String::new(),
                "Slash commands start with '/'. Type /help for all commands.".to_string(),
            ];
            Ok(CommandOutput::message(&lines.join("\n")))
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_keybindings() {
        let result = (KEYBINDINGS.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Keyboard Shortcuts:"));
        assert!(msg.contains("Ctrl+C"));
    }
}
