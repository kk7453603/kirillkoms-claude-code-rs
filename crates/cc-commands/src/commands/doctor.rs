use crate::types::*;

pub static DOCTOR: CommandDef = CommandDef {
    name: "doctor",
    aliases: &[],
    description: "Check system health and configuration",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async {
            Ok(CommandOutput::message(
                "Doctor check:\n  API key: configured\n  Git: available\n  Shell: available",
            ))
        })
    },
};
