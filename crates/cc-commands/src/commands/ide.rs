use crate::types::*;

pub static IDE: CommandDef = CommandDef {
    name: "ide",
    aliases: &[],
    description: "IDE integration info",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async {
            let mut lines = vec![
                "IDE Integration".to_string(),
                String::new(),
                "Supported editors:".to_string(),
                "  VS Code    - Install 'Claude Code' extension".to_string(),
                "  JetBrains  - Use terminal integration".to_string(),
                "  Vim/Neovim - Use terminal or plugin".to_string(),
                "  Emacs      - Use terminal integration".to_string(),
                String::new(),
            ];

            // Check for common editor env vars
            if std::env::var("VSCODE_PID").is_ok()
                || std::env::var("TERM_PROGRAM").as_deref() == Ok("vscode")
            {
                lines.push("Detected: VS Code terminal".to_string());
            } else if std::env::var("TERMINAL_EMULATOR").as_deref() == Ok("JetBrains-JediTerm") {
                lines.push("Detected: JetBrains terminal".to_string());
            } else if let Ok(term) = std::env::var("TERM_PROGRAM") {
                lines.push(format!("Terminal: {}", term));
            }

            Ok(CommandOutput::message(&lines.join("\n")))
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ide_info() {
        let result = (IDE.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("IDE Integration"));
        assert!(msg.contains("VS Code"));
    }
}
