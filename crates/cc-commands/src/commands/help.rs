use crate::types::*;

pub static HELP: CommandDef = CommandDef {
    name: "help",
    aliases: &["h", "?"],
    description: "Show help information",
    argument_hint: Some("[command]"),
    hidden: false,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            let args = args.as_str();
            if !args.is_empty() {
                let registry = crate::registry::CommandRegistry::with_defaults();
                if let Some(cmd) = registry.lookup(args) {
                    let mut help = format!("/{}", cmd.name);
                    if let Some(hint) = cmd.argument_hint {
                        help.push_str(&format!(" {}", hint));
                    }
                    help.push_str(&format!("\n  {}", cmd.description));
                    if !cmd.aliases.is_empty() {
                        help.push_str(&format!(
                            "\n  Aliases: {}",
                            cmd.aliases
                                .iter()
                                .map(|a| format!("/{}", a))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ));
                    }
                    return Ok(CommandOutput::message(&help));
                } else {
                    return Ok(CommandOutput::message(&format!(
                        "Unknown command: /{}. Type /help for a list of commands.",
                        args
                    )));
                }
            }

            let registry = crate::registry::CommandRegistry::with_defaults();
            let mut cmds = registry.visible_commands();
            cmds.sort_by_key(|c| c.name);

            let mut lines = vec!["Available commands:".to_string(), String::new()];
            for cmd in &cmds {
                let mut entry = format!("  /{:<18} {}", cmd.name, cmd.description);
                if !cmd.aliases.is_empty() {
                    entry.push_str(&format!(
                        " (aliases: {})",
                        cmd.aliases
                            .iter()
                            .map(|a| format!("/{}", a))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
                lines.push(entry);
            }
            lines.push(String::new());
            lines.push("Type /help <command> for detailed help on a specific command.".to_string());

            Ok(CommandOutput::message(&lines.join("\n")))
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_help_lists_commands() {
        let result = (HELP.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Available commands:"));
        assert!(msg.contains("/help"));
        assert!(msg.contains("/exit"));
        assert!(result.should_continue);
    }

    #[tokio::test]
    async fn test_help_specific_command() {
        let result = (HELP.handler)("exit").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("/exit"));
    }

    #[tokio::test]
    async fn test_help_unknown_command() {
        let result = (HELP.handler)("nonexistent_xyz").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Unknown command"));
    }
}
