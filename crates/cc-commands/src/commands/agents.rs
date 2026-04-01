use crate::types::*;

pub static AGENTS: CommandDef = CommandDef {
    name: "agents",
    aliases: &[],
    description: "List agent definitions",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async {
            let lines = vec![
                "Available Agents:".to_string(),
                String::new(),
                "  main           - Primary coding agent with full tool access".to_string(),
                "  task           - Sub-agent for parallel task execution".to_string(),
                "  review         - Code review specialist".to_string(),
                "  commit         - Git commit message generator".to_string(),
                String::new(),
                "Agents are spawned automatically when needed.".to_string(),
                "Use /tasks to manage running sub-agents.".to_string(),
            ];
            Ok(CommandOutput::message(&lines.join("\n")))
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agents_list() {
        let result = (AGENTS.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Available Agents:"));
        assert!(msg.contains("main"));
        assert!(result.should_continue);
    }
}
