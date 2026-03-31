use crate::types::*;

pub static HELP: CommandDef = CommandDef {
    name: "help",
    aliases: &["h", "?"],
    description: "Show help information",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async {
            Ok(CommandOutput::message(
                "Available commands: /help, /clear, /compact, /config, /cost, /diff, /doctor, \
                 /exit, /model, /status, /version, /memory, /resume, /session, /theme, \
                 /permissions, /context, /commit, /review, /hooks, /mcp",
            ))
        })
    },
};
