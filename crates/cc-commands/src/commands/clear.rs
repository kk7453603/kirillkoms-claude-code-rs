use crate::types::*;

pub static CLEAR: CommandDef = CommandDef {
    name: "clear",
    aliases: &[],
    description: "Clear conversation history",
    argument_hint: None,
    hidden: false,
    handler: |_args| Box::pin(async { Ok(CommandOutput::message("Conversation cleared.")) }),
};
