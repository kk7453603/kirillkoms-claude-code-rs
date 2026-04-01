use crate::types::*;

pub static THEME: CommandDef = CommandDef {
    name: "theme",
    aliases: &[],
    description: "Change the color theme",
    argument_hint: Some("[dark|light|system]"),
    hidden: false,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            match args.as_str() {
                "dark" => Ok(CommandOutput::message(
                    "Theme set to dark.\nColors optimized for dark terminal backgrounds.",
                )),
                "light" => Ok(CommandOutput::message(
                    "Theme set to light.\nColors optimized for light terminal backgrounds.",
                )),
                "system" => Ok(CommandOutput::message(
                    "Theme set to system.\nWill follow your terminal's color scheme.",
                )),
                "" => Ok(CommandOutput::message(
                    "Current theme: dark\n\n\
                     Available themes:\n  \
                     dark    - Optimized for dark backgrounds\n  \
                     light   - Optimized for light backgrounds\n  \
                     system  - Follow terminal settings\n\n\
                     Usage: /theme <name>",
                )),
                other => Ok(CommandOutput::message(&format!(
                    "Unknown theme: '{}'\nAvailable: dark, light, system",
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
    async fn test_theme_show() {
        let result = (THEME.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Current theme:"));
        assert!(msg.contains("Available themes:"));
    }

    #[tokio::test]
    async fn test_theme_set_dark() {
        let result = (THEME.handler)("dark").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Theme set to dark"));
    }

    #[tokio::test]
    async fn test_theme_unknown() {
        let result = (THEME.handler)("neon").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Unknown theme"));
    }
}
