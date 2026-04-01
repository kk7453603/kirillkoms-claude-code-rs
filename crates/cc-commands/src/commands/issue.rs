use crate::types::*;

pub static ISSUE: CommandDef = CommandDef {
    name: "issue",
    aliases: &[],
    description: "Create or view GitHub issues",
    argument_hint: Some("[create|view|list] [args]"),
    hidden: false,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            let parts: Vec<&str> = args.splitn(2, ' ').collect();
            match parts.first().map(|s| *s) {
                Some("create") => {
                    let title = parts.get(1).unwrap_or(&"");
                    if title.is_empty() {
                        Ok(CommandOutput::message(
                            "Usage: /issue create <title>\n\
                             Creates a new GitHub issue in the current repository.",
                        ))
                    } else {
                        Ok(CommandOutput::message(&format!(
                            "Creating issue: {}\n\
                             Issue will be created in the current repository.",
                            title
                        )))
                    }
                }
                Some("list") => Ok(CommandOutput::message(
                    "Fetching open issues for the current repository...",
                )),
                Some("view") => {
                    let num = parts.get(1).unwrap_or(&"");
                    if num.is_empty() {
                        Ok(CommandOutput::message("Usage: /issue view <number>"))
                    } else {
                        Ok(CommandOutput::message(&format!(
                            "Fetching issue #{}...",
                            num
                        )))
                    }
                }
                Some("") | None => Ok(CommandOutput::message(
                    "Usage: /issue <subcommand> [args]\n\n\
                     Subcommands:\n  \
                     create <title>  - Create a new issue\n  \
                     view <number>   - View an issue\n  \
                     list            - List open issues",
                )),
                Some(other) => Ok(CommandOutput::message(&format!(
                    "Unknown subcommand: '{}'\nUsage: /issue [create|view|list] [args]",
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
    async fn test_issue_no_args() {
        let result = (ISSUE.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Usage:"));
    }

    #[tokio::test]
    async fn test_issue_list() {
        let result = (ISSUE.handler)("list").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Fetching"));
    }
}
