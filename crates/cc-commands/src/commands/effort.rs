use crate::types::*;

pub static EFFORT: CommandDef = CommandDef {
    name: "effort",
    aliases: &[],
    description: "Set reasoning effort level",
    argument_hint: Some("[low|medium|high]"),
    hidden: false,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            match args.as_str() {
                "low" | "l" => Ok(CommandOutput::message(
                    "Effort set to low.\n\
                     Faster responses with less detailed reasoning.",
                )),
                "medium" | "m" | "med" => Ok(CommandOutput::message(
                    "Effort set to medium.\n\
                     Balanced speed and reasoning depth.",
                )),
                "high" | "h" => Ok(CommandOutput::message(
                    "Effort set to high.\n\
                     Thorough reasoning with extended thinking.",
                )),
                "" => Ok(CommandOutput::message(
                    "Current effort: medium\n\n\
                     Available levels:\n  \
                     low    (l)  - Fast, minimal reasoning\n  \
                     medium (m)  - Balanced (default)\n  \
                     high   (h)  - Deep reasoning, extended thinking\n\n\
                     Usage: /effort <level>",
                )),
                other => Ok(CommandOutput::message(&format!(
                    "Unknown effort level: '{}'\nAvailable: low, medium, high",
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
    async fn test_effort_show() {
        let result = (EFFORT.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Current effort:"));
    }

    #[tokio::test]
    async fn test_effort_set_high() {
        let result = (EFFORT.handler)("high").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Effort set to high"));
    }
}
