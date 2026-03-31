use crate::types::*;

pub static HOOKS: CommandDef = CommandDef {
    name: "hooks",
    aliases: &[],
    description: "Manage event hooks",
    argument_hint: Some("[list|add|remove]"),
    hidden: true,
    handler: |args| {
        let msg = if args.is_empty() {
            "Registered hooks: (none)".to_string()
        } else {
            format!("Hooks command: {args}")
        };
        Box::pin(async move { Ok(CommandOutput::message(&msg)) })
    },
};
