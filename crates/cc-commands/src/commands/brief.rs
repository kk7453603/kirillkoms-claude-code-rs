use crate::types::*;

pub static BRIEF: CommandDef = CommandDef {
    name: "brief",
    aliases: &[],
    description: "Toggle brief output mode for shorter responses",
    argument_hint: Some("[on|off]"),
    hidden: false,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            match args.as_str() {
                "on" => Ok(CommandOutput::message(
                    "Brief mode enabled. Responses will be more concise.",
                )),
                "off" => Ok(CommandOutput::message(
                    "Brief mode disabled. Responses will use normal verbosity.",
                )),
                "" => Ok(CommandOutput::message(
                    "Brief mode: off\n\n\
                     When enabled, Claude will give shorter, more concise responses.\n\n\
                     Usage: /brief [on|off]",
                )),
                other => Ok(CommandOutput::message(&format!(
                    "Unknown option: '{}'\nUsage: /brief [on|off]",
                    other
                ))),
            }
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_brief_show() {
        let result = (BRIEF.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Brief mode"));
    }
}
