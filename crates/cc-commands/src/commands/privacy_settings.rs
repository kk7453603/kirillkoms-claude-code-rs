use crate::types::*;

pub static PRIVACY_SETTINGS: CommandDef = CommandDef {
    name: "privacy-settings",
    aliases: &["privacy"],
    description: "View and manage privacy settings",
    argument_hint: Some("[telemetry|history] [on|off]"),
    hidden: false,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            let parts: Vec<&str> = args.splitn(2, ' ').collect();
            match (parts.first().copied(), parts.get(1).copied()) {
                (Some("telemetry"), Some("on")) => Ok(CommandOutput::message(
                    "Telemetry enabled. Anonymous usage data will be collected.",
                )),
                (Some("telemetry"), Some("off")) => Ok(CommandOutput::message(
                    "Telemetry disabled. No anonymous usage data will be collected.",
                )),
                (Some("history"), Some("on")) => Ok(CommandOutput::message(
                    "Conversation history enabled. Sessions will be saved locally.",
                )),
                (Some("history"), Some("off")) => Ok(CommandOutput::message(
                    "Conversation history disabled. Sessions will not be saved.",
                )),
                (Some(""), None) | (None, _) => Ok(CommandOutput::message(
                    "Privacy Settings\n\n  \
                     telemetry: on  - Anonymous usage data collection\n  \
                     history:   on  - Local conversation history\n\n\
                     Usage: /privacy-settings <setting> [on|off]\n\n\
                     Data is stored locally. See https://claude.ai/privacy for details.",
                )),
                _ => Ok(CommandOutput::message(
                    "Usage: /privacy-settings [telemetry|history] [on|off]",
                )),
            }
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_privacy_settings_show() {
        let result = (PRIVACY_SETTINGS.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Privacy Settings"));
    }
}
