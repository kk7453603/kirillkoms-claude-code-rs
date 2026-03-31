use crate::types::*;

pub static DIFF: CommandDef = CommandDef {
    name: "diff",
    aliases: &[],
    description: "Show changes made in this session",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async { Ok(CommandOutput::message("No changes in this session.")) })
    },
};
