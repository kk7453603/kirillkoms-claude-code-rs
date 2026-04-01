use crate::types::*;

pub static INSTALL: CommandDef = CommandDef {
    name: "install",
    aliases: &[],
    description: "Install or update Claude Code",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async move {
            Ok(CommandOutput::message(
                "Claude Code installation manager.\n\n\
                 To install/update Claude Code, run:\n  \
                 npm install -g @anthropic-ai/claude-code\n\n\
                 Or use the /upgrade command to check for updates.",
            ))
        })
    },
};

pub static INSTALL_GITHUB_APP: CommandDef = CommandDef {
    name: "install-github-app",
    aliases: &[],
    description: "Install the Claude GitHub App",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async move {
            Ok(CommandOutput::message(
                "Install the Claude GitHub App to enable:\n  \
                 - Automated code review on pull requests\n  \
                 - Issue triage and labeling\n  \
                 - PR summary generation\n\n\
                 Visit https://github.com/apps/claude to install.",
            ))
        })
    },
};

pub static INSTALL_SLACK_APP: CommandDef = CommandDef {
    name: "install-slack-app",
    aliases: &[],
    description: "Install the Claude Slack app",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async move {
            Ok(CommandOutput::message(
                "Install the Claude Slack App to bring Claude into your Slack workspace.\n\n\
                 Visit https://claude.ai/slack to install.",
            ))
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_install() {
        let result = (INSTALL.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("install"));
    }

    #[tokio::test]
    async fn test_install_github_app() {
        let result = (INSTALL_GITHUB_APP.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("GitHub"));
    }

    #[tokio::test]
    async fn test_install_slack_app() {
        let result = (INSTALL_SLACK_APP.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Slack"));
    }
}
