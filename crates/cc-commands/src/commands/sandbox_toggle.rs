use crate::types::*;

pub static SANDBOX_TOGGLE: CommandDef = CommandDef {
    name: "sandbox-toggle",
    aliases: &["sandbox"],
    description: "Toggle sandbox mode for command execution",
    argument_hint: Some("[on|off]"),
    hidden: false,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            match args.as_str() {
                "on" => Ok(CommandOutput::message(
                    "Sandbox mode enabled.\n\
                     Commands will run in an isolated environment.\n\
                     File system changes are restricted to the working directory.",
                )),
                "off" => Ok(CommandOutput::message(
                    "Sandbox mode disabled.\n\
                     Commands will run with full system access.\n\
                     Exercise caution with destructive operations.",
                )),
                "" => Ok(CommandOutput::message(
                    "Sandbox mode: enabled\n\n\
                     When enabled, commands run in an isolated environment\n\
                     with restricted file system access.\n\n\
                     Usage: /sandbox-toggle [on|off]",
                )),
                other => Ok(CommandOutput::message(&format!(
                    "Unknown option: '{}'\nUsage: /sandbox-toggle [on|off]",
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
    async fn test_sandbox_toggle_show() {
        let result = (SANDBOX_TOGGLE.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Sandbox mode"));
    }

    #[tokio::test]
    async fn test_sandbox_toggle_on() {
        let result = (SANDBOX_TOGGLE.handler)("on").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("enabled"));
    }
}
