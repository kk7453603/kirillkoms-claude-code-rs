use crate::types::*;

pub static PERMISSIONS: CommandDef = CommandDef {
    name: "permissions",
    aliases: &["perms"],
    description: "View or manage tool permissions",
    argument_hint: Some("[allow|deny|reset] [tool]"),
    hidden: false,
    handler: |args| {
        let msg = if args.is_empty() {
            "Current permissions:\n  All tools: ask".to_string()
        } else {
            format!("Permissions updated: {args}")
        };
        Box::pin(async move { Ok(CommandOutput::message(&msg)) })
    },
};
