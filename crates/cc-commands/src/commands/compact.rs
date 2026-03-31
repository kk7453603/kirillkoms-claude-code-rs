use crate::types::*;

pub static COMPACT: CommandDef = CommandDef {
    name: "compact",
    aliases: &[],
    description: "Compact conversation to save context",
    argument_hint: Some("[instructions]"),
    hidden: false,
    handler: |args| {
        let msg = if args.is_empty() {
            "Compacting conversation...".to_string()
        } else {
            format!("Compacting conversation with instructions: {args}")
        };
        Box::pin(async move { Ok(CommandOutput::message(&msg)) })
    },
};
