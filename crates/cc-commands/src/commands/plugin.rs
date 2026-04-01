use crate::types::*;

pub static PLUGIN: CommandDef = CommandDef {
    name: "plugin",
    aliases: &["plugins"],
    description: "Plugin management",
    argument_hint: Some("[list|install|remove]"),
    hidden: false,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            match args.split_whitespace().collect::<Vec<_>>().as_slice() {
                [] | ["list"] => Ok(CommandOutput::message(
                    "Installed plugins: (none)\n\n\
                     Plugins extend Claude Code with additional capabilities.\n\
                     Install: /plugin install <name>\n\
                     Remove:  /plugin remove <name>",
                )),
                ["install", name] => Ok(CommandOutput::message(&format!(
                    "Plugin '{}' installation is not yet supported in this version.\n\
                     Check back for updates.",
                    name
                ))),
                ["remove", name] => Ok(CommandOutput::message(&format!(
                    "Plugin '{}' not found in installed plugins.",
                    name
                ))),
                _ => Ok(CommandOutput::message(
                    "Usage: /plugin [list|install|remove] [name]",
                )),
            }
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_plugin_list() {
        let result = (PLUGIN.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Installed plugins:"));
    }
}
