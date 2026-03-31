use crate::types::*;

pub static SESSION: CommandDef = CommandDef {
    name: "session",
    aliases: &[],
    description: "Manage sessions",
    argument_hint: Some("[list|new|delete]"),
    hidden: false,
    handler: |args| {
        let msg = match args.trim() {
            "list" | "" => "Active sessions:\n  (none)".to_string(),
            "new" => "New session created.".to_string(),
            _ => format!("Session command: {args}"),
        };
        Box::pin(async move { Ok(CommandOutput::message(&msg)) })
    },
};
