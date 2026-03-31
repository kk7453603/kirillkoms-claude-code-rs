use crate::types::*;

pub static VERSION: CommandDef = CommandDef {
    name: "version",
    aliases: &["v"],
    description: "Show version information",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async {
            Ok(CommandOutput::message(&format!(
                "claude-code v{}",
                env!("CARGO_PKG_VERSION")
            )))
        })
    },
};
