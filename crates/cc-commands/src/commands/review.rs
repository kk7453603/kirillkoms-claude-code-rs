use crate::types::*;

pub static REVIEW: CommandDef = CommandDef {
    name: "review",
    aliases: &["pr"],
    description: "Review a pull request",
    argument_hint: Some("[pr_number]"),
    hidden: false,
    handler: |args| {
        let msg = if args.is_empty() {
            "Usage: /review <pr_number>".to_string()
        } else {
            format!("Reviewing PR #{args}...")
        };
        Box::pin(async move { Ok(CommandOutput::message(&msg)) })
    },
};
