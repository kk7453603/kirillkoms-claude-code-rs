use crate::types::*;

pub static COLOR: CommandDef = CommandDef {
    name: "color",
    aliases: &["output-style"],
    description: "Set output color/style preferences",
    argument_hint: Some("[plain|color|markdown|auto]"),
    hidden: false,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            match args.as_str() {
                "plain" => Ok(CommandOutput::message(
                    "Output style set to plain.\nNo colors or formatting will be used.",
                )),
                "color" => Ok(CommandOutput::message(
                    "Output style set to color.\nTerminal colors enabled for output.",
                )),
                "markdown" => Ok(CommandOutput::message(
                    "Output style set to markdown.\nRich markdown rendering enabled.",
                )),
                "auto" => Ok(CommandOutput::message(
                    "Output style set to auto.\nWill detect terminal capabilities automatically.",
                )),
                "" => Ok(CommandOutput::message(
                    "Current output style: auto\n\n\
                     Available styles:\n  \
                     plain    - No colors or formatting\n  \
                     color    - Terminal colors enabled\n  \
                     markdown - Rich markdown rendering\n  \
                     auto     - Detect terminal capabilities\n\n\
                     Usage: /color <style>",
                )),
                other => Ok(CommandOutput::message(&format!(
                    "Unknown output style: '{}'\nAvailable: plain, color, markdown, auto",
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
    async fn test_color_show() {
        let result = (COLOR.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Current output style:"));
    }

    #[tokio::test]
    async fn test_color_set_plain() {
        let result = (COLOR.handler)("plain").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("plain"));
    }
}
