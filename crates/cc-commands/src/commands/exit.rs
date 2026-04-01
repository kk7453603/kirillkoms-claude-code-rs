use crate::types::*;

pub static EXIT: CommandDef = CommandDef {
    name: "exit",
    aliases: &["quit", "q"],
    description: "Exit the application",
    argument_hint: None,
    hidden: false,
    handler: |_args| Box::pin(async { Ok(CommandOutput::exit()) }),
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_exit() {
        let result = (EXIT.handler)("").await.unwrap();
        assert!(!result.should_continue);
        assert!(result.message.is_none());
    }
}
