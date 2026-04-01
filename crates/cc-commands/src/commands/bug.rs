use crate::types::*;

pub static BUG: CommandDef = CommandDef {
    name: "bug",
    aliases: &["report-bug"],
    description: "Report a bug",
    argument_hint: Some("[description]"),
    hidden: false,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            if args.is_empty() {
                Ok(CommandOutput::message(
                    "To report a bug:\n\n\
                     1. Visit https://github.com/anthropics/claude-code/issues/new\n\
                     2. Or use: /bug <description>\n\n\
                     Please include:\n  \
                     - Steps to reproduce\n  \
                     - Expected vs actual behavior\n  \
                     - OS and version (/version)",
                ))
            } else {
                let version = env!("CARGO_PKG_VERSION");
                let platform = format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH);
                Ok(CommandOutput::message(&format!(
                    "Bug report prepared:\n\n\
                     Description: {}\n\
                     Version: {}\n\
                     Platform: {}\n\n\
                     Submit at: https://github.com/anthropics/claude-code/issues/new",
                    args, version, platform
                )))
            }
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bug_no_args() {
        let result = (BUG.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("report a bug"));
    }

    #[tokio::test]
    async fn test_bug_with_description() {
        let result = (BUG.handler)("something broke").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("something broke"));
        assert!(msg.contains("Version:"));
    }
}
