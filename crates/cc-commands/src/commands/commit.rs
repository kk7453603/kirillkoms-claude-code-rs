use crate::types::*;

pub static COMMIT: CommandDef = CommandDef {
    name: "commit",
    aliases: &[],
    description: "Create a git commit with AI-generated message",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async {
            Ok(CommandOutput::message(
                "Analyzing staged changes for commit message...",
            ))
        })
    },
};
