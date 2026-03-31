/// The kind of command (prompt-based or local).
#[derive(Debug, Clone)]
pub enum CommandKind {
    Prompt,
    Local,
}

/// Information about a registered command.
#[derive(Debug, Clone)]
pub struct CommandInfo {
    pub name: String,
    pub aliases: Vec<String>,
    pub description: String,
    pub argument_hint: Option<String>,
    pub hidden: bool,
    pub user_invocable: bool,
    pub kind: CommandKind,
    pub loaded_from: CommandSource,
}

/// Where a command was loaded from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandSource {
    Bundled,
    Skills,
    Plugin,
    Managed,
    Mcp,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_info_construction() {
        let cmd = CommandInfo {
            name: "help".to_string(),
            aliases: vec!["h".to_string(), "?".to_string()],
            description: "Show help information".to_string(),
            argument_hint: None,
            hidden: false,
            user_invocable: true,
            kind: CommandKind::Local,
            loaded_from: CommandSource::Bundled,
        };
        assert_eq!(cmd.name, "help");
        assert_eq!(cmd.aliases.len(), 2);
        assert!(!cmd.hidden);
        assert!(cmd.user_invocable);
        assert_eq!(cmd.loaded_from, CommandSource::Bundled);
    }

    #[test]
    fn command_info_prompt_kind() {
        let cmd = CommandInfo {
            name: "review".to_string(),
            aliases: vec![],
            description: "Review code changes".to_string(),
            argument_hint: Some("<file>".to_string()),
            hidden: false,
            user_invocable: true,
            kind: CommandKind::Prompt,
            loaded_from: CommandSource::Skills,
        };
        assert!(matches!(cmd.kind, CommandKind::Prompt));
        assert_eq!(cmd.argument_hint, Some("<file>".to_string()));
        assert_eq!(cmd.loaded_from, CommandSource::Skills);
    }

    #[test]
    fn command_info_hidden() {
        let cmd = CommandInfo {
            name: "debug".to_string(),
            aliases: vec![],
            description: "Internal debug command".to_string(),
            argument_hint: None,
            hidden: true,
            user_invocable: false,
            kind: CommandKind::Local,
            loaded_from: CommandSource::Bundled,
        };
        assert!(cmd.hidden);
        assert!(!cmd.user_invocable);
    }

    #[test]
    fn command_source_equality() {
        assert_eq!(CommandSource::Bundled, CommandSource::Bundled);
        assert_ne!(CommandSource::Bundled, CommandSource::Skills);
        assert_ne!(CommandSource::Plugin, CommandSource::Managed);
        assert_eq!(CommandSource::Mcp, CommandSource::Mcp);
    }

    #[test]
    fn command_source_all_variants() {
        let sources = [
            CommandSource::Bundled,
            CommandSource::Skills,
            CommandSource::Plugin,
            CommandSource::Managed,
            CommandSource::Mcp,
        ];
        for (i, a) in sources.iter().enumerate() {
            for (j, b) in sources.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b);
                }
            }
        }
    }

    #[test]
    fn command_kind_debug() {
        let prompt = CommandKind::Prompt;
        let local = CommandKind::Local;
        let _ = format!("{:?}", prompt);
        let _ = format!("{:?}", local);
    }

    #[test]
    fn command_info_clone() {
        let cmd = CommandInfo {
            name: "test".to_string(),
            aliases: vec!["t".to_string()],
            description: "A test command".to_string(),
            argument_hint: Some("<arg>".to_string()),
            hidden: false,
            user_invocable: true,
            kind: CommandKind::Local,
            loaded_from: CommandSource::Plugin,
        };
        let cloned = cmd.clone();
        assert_eq!(cloned.name, "test");
        assert_eq!(cloned.aliases, vec!["t".to_string()]);
        assert_eq!(cloned.loaded_from, CommandSource::Plugin);
    }
}
