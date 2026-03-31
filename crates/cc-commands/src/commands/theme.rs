use crate::types::*;

pub static THEME: CommandDef = CommandDef {
    name: "theme",
    aliases: &[],
    description: "Change the color theme",
    argument_hint: Some("[dark|light]"),
    hidden: false,
    handler: |args| {
        let msg = match args.trim() {
            "dark" => "Theme set to dark.".to_string(),
            "light" => "Theme set to light.".to_string(),
            "" => "Current theme: dark\nAvailable: dark, light".to_string(),
            other => format!("Unknown theme: {other}"),
        };
        Box::pin(async move { Ok(CommandOutput::message(&msg)) })
    },
};
