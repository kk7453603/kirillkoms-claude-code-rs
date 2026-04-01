use crate::types::*;

pub static ONBOARDING: CommandDef = CommandDef {
    name: "onboarding",
    aliases: &[],
    description: "Run the onboarding flow for new users",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async move {
            Ok(CommandOutput::message(
                "Welcome to Claude Code!\n\n\
                 Let's get you set up. Here are the basics:\n\n\
                 1. Type naturally to ask Claude for help with code\n\
                 2. Use /help to see all available commands\n\
                 3. Use /config to customize your settings\n\
                 4. Use /model to change the AI model\n\
                 5. Use /memory to save project-specific notes\n\n\
                 Tips:\n\
                 - Claude can read, write, and edit files in your project\n\
                 - Use /compact to summarize long conversations\n\
                 - Use /cost to track your token usage\n\n\
                 You're all set! Start by asking Claude a question about your code.",
            ))
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_onboarding() {
        let result = (ONBOARDING.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Welcome"));
    }
}
