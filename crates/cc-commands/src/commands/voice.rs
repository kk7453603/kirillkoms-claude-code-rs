use crate::types::*;

pub static VOICE: CommandDef = CommandDef {
    name: "voice",
    aliases: &[],
    description: "Toggle voice input mode",
    argument_hint: Some("[on|off]"),
    hidden: false,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            match args.as_str() {
                "on" | "true" | "1" => Ok(CommandOutput::message(
                    "Voice mode is not yet supported in this version.\n\
                     This feature requires a microphone and speech-to-text service.",
                )),
                "off" | "false" | "0" => Ok(CommandOutput::message(
                    "Voice mode disabled.",
                )),
                "" => Ok(CommandOutput::message(
                    "Voice mode: off (not available)\n\n\
                     Voice input is not yet supported in this version.\n\
                     Usage: /voice [on|off]",
                )),
                _ => Ok(CommandOutput::message("Usage: /voice [on|off]")),
            }
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_voice() {
        let result = (VOICE.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Voice mode:"));
    }
}
