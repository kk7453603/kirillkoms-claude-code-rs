use crate::types::*;

pub static EXIT: CommandDef = CommandDef {
    name: "exit",
    aliases: &["quit", "q"],
    description: "Exit the application",
    argument_hint: None,
    hidden: false,
    handler: |_args| Box::pin(async { Ok(CommandOutput::exit()) }),
};
