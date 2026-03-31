use crate::types::*;

pub static CONFIG: CommandDef = CommandDef {
    name: "config",
    aliases: &[],
    description: "View or modify configuration",
    argument_hint: Some("[key] [value]"),
    hidden: false,
    handler: |args| {
        let msg = if args.is_empty() {
            "Current configuration:\n  model: claude-sonnet-4-6\n  permission_mode: default".to_string()
        } else {
            format!("Config updated: {args}")
        };
        Box::pin(async move { Ok(CommandOutput::message(&msg)) })
    },
};
