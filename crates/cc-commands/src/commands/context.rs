use crate::types::*;

pub static CONTEXT: CommandDef = CommandDef {
    name: "context",
    aliases: &["ctx"],
    description: "Manage context files and URLs",
    argument_hint: Some("[add|remove|list] [path]"),
    hidden: false,
    handler: |args| {
        let msg = if args.is_empty() {
            "Context items: (none)".to_string()
        } else {
            format!("Context updated: {args}")
        };
        Box::pin(async move { Ok(CommandOutput::message(&msg)) })
    },
};
