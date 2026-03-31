use crate::types::*;

pub static COST: CommandDef = CommandDef {
    name: "cost",
    aliases: &[],
    description: "Show token usage and cost",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async {
            Ok(CommandOutput::message(
                "Session cost: $0.00\nTotal tokens: 0 input, 0 output",
            ))
        })
    },
};
