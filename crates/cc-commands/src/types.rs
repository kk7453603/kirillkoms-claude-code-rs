use std::future::Future;
use std::pin::Pin;

pub type CommandResult = Result<CommandOutput, CommandError>;
pub type CommandFuture = Pin<Box<dyn Future<Output = CommandResult> + Send>>;

#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub message: Option<String>,
    pub should_continue: bool,
}

impl CommandOutput {
    pub fn message(msg: &str) -> Self {
        Self {
            message: Some(msg.to_string()),
            should_continue: true,
        }
    }

    pub fn silent() -> Self {
        Self {
            message: None,
            should_continue: true,
        }
    }

    pub fn exit() -> Self {
        Self {
            message: None,
            should_continue: false,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("{message}")]
    User { message: String },
    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

pub struct CommandDef {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub description: &'static str,
    pub argument_hint: Option<&'static str>,
    pub hidden: bool,
    pub handler: fn(args: &str) -> CommandFuture,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_output_message() {
        let out = CommandOutput::message("hello");
        assert_eq!(out.message, Some("hello".to_string()));
        assert!(out.should_continue);
    }

    #[test]
    fn test_command_output_silent() {
        let out = CommandOutput::silent();
        assert!(out.message.is_none());
        assert!(out.should_continue);
    }

    #[test]
    fn test_command_output_exit() {
        let out = CommandOutput::exit();
        assert!(out.message.is_none());
        assert!(!out.should_continue);
    }

    #[test]
    fn test_command_error_user() {
        let err = CommandError::User {
            message: "bad input".to_string(),
        };
        assert_eq!(err.to_string(), "bad input");
    }

    #[test]
    fn test_command_error_internal() {
        let err = CommandError::Internal(anyhow::anyhow!("something broke"));
        assert_eq!(err.to_string(), "Internal error: something broke");
    }
}
