use crate::types::*;

pub static MEMORY: CommandDef = CommandDef {
    name: "memory",
    aliases: &[],
    description: "View or edit CLAUDE.md memory files",
    argument_hint: Some("[view|edit]"),
    hidden: false,
    handler: |_args| {
        Box::pin(async { Ok(CommandOutput::message("Memory files:\n  No CLAUDE.md found.")) })
    },
};
