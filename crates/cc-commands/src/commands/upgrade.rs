use crate::types::*;

pub static UPGRADE: CommandDef = CommandDef {
    name: "upgrade",
    aliases: &["update"],
    description: "Check for updates",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async {
            let current = env!("CARGO_PKG_VERSION");
            let mut lines = vec![
                format!("Current version: {}", current),
            ];

            // Check if installed via cargo
            let cargo_check = cc_utils::shell::execute_command(
                "cargo",
                &["--version"],
                std::path::Path::new("/tmp"),
            )
            .await;

            if cargo_check.is_ok() {
                lines.push(String::new());
                lines.push("To update via cargo:".to_string());
                lines.push("  cargo install claude-code-rs --force".to_string());
            }

            lines.push(String::new());
            lines.push("Check releases at:".to_string());
            lines.push("  https://github.com/anthropics/claude-code/releases".to_string());

            Ok(CommandOutput::message(&lines.join("\n")))
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_upgrade() {
        let result = (UPGRADE.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Current version:"));
    }
}
