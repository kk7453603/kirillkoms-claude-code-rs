use crate::types::*;

pub static STATUS: CommandDef = CommandDef {
    name: "status",
    aliases: &[],
    description: "Show current session status",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async {
            Ok(CommandOutput::message(
                "Session active\nMessages: 0\nTokens used: 0",
            ))
        })
    },
};
