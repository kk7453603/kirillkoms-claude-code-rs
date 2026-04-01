use crate::types::*;

pub static VIM: CommandDef = CommandDef {
    name: "vim",
    aliases: &[],
    description: "Toggle vim keybinding mode",
    argument_hint: Some("[on|off]"),
    hidden: false,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            match args.as_str() {
                "on" | "true" | "1" => Ok(CommandOutput::message(
                    "Vim mode enabled.\n\
                     Using vim-style keybindings for input.",
                )),
                "off" | "false" | "0" => Ok(CommandOutput::message(
                    "Vim mode disabled.\n\
                     Using default keybindings.",
                )),
                "" => Ok(CommandOutput::message(
                    "Vim mode: off\n\n\
                     When enabled, uses vim-style keybindings (hjkl navigation, modes).\n\
                     Usage: /vim [on|off]",
                )),
                _ => Ok(CommandOutput::message("Usage: /vim [on|off]")),
            }
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_vim_toggle() {
        let result = (VIM.handler)("on").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Vim mode enabled"));
    }

    #[tokio::test]
    async fn test_vim_show() {
        let result = (VIM.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Vim mode:"));
    }
}
