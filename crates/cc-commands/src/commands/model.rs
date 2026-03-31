use crate::types::*;

pub static MODEL: CommandDef = CommandDef {
    name: "model",
    aliases: &[],
    description: "View or change the current model",
    argument_hint: Some("[model_name]"),
    hidden: false,
    handler: |args| {
        let msg = if args.is_empty() {
            "Current model: claude-sonnet-4-6".to_string()
        } else {
            format!("Model changed to: {args}")
        };
        Box::pin(async move { Ok(CommandOutput::message(&msg)) })
    },
};
