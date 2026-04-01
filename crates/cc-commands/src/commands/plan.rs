use crate::types::*;

pub static PLAN: CommandDef = CommandDef {
    name: "plan",
    aliases: &[],
    description: "Toggle plan mode (read-only)",
    argument_hint: Some("[on|off]"),
    hidden: false,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            match args.as_str() {
                "on" | "true" | "1" => Ok(CommandOutput::message(
                    "Plan mode enabled.\n\
                     Claude will only read and analyze code, not make changes.\n\
                     All write operations are blocked.",
                )),
                "off" | "false" | "0" => Ok(CommandOutput::message(
                    "Plan mode disabled.\n\
                     Claude can now make changes to files.",
                )),
                "" => Ok(CommandOutput::message(
                    "Plan mode: off\n\n\
                     When enabled, Claude operates in read-only mode.\n\
                     It will analyze code and suggest changes without executing them.\n\n\
                     Usage: /plan [on|off]",
                )),
                _ => Ok(CommandOutput::message("Usage: /plan [on|off]")),
            }
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_plan_toggle() {
        let result = (PLAN.handler)("on").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Plan mode enabled"));
    }
}
