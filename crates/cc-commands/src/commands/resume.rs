use crate::types::*;

pub static RESUME: CommandDef = CommandDef {
    name: "resume",
    aliases: &[],
    description: "Resume a previous session",
    argument_hint: Some("[session_id]"),
    hidden: false,
    handler: |args| {
        let msg = if args.is_empty() {
            "No session ID provided. Use /resume <session_id>".to_string()
        } else {
            format!("Resuming session: {args}")
        };
        Box::pin(async move { Ok(CommandOutput::message(&msg)) })
    },
};
