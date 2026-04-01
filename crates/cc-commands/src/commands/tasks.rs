use crate::types::*;

pub static TASKS: CommandDef = CommandDef {
    name: "tasks",
    aliases: &[],
    description: "Task management",
    argument_hint: Some("[list|cancel <id>]"),
    hidden: false,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            let mgr = cc_tasks::types::TaskManager::new();

            match args.split_whitespace().collect::<Vec<_>>().as_slice() {
                [] | ["list"] => {
                    let tasks = mgr.list_tasks();
                    if tasks.is_empty() {
                        Ok(CommandOutput::message(
                            "No active tasks.\n\n\
                             Tasks are created automatically when sub-agents are spawned.\n\
                             Use /tasks list to check running tasks.",
                        ))
                    } else {
                        let mut lines = vec![format!("Tasks ({}):", tasks.len())];
                        for task in tasks {
                            lines.push(format!(
                                "  {} [{}] {}",
                                task.id,
                                serde_json::to_string(&task.status)
                                    .unwrap_or_else(|_| "unknown".to_string())
                                    .trim_matches('"'),
                                task.name
                            ));
                        }
                        Ok(CommandOutput::message(&lines.join("\n")))
                    }
                }
                ["cancel", id] => Ok(CommandOutput::message(&format!(
                    "Cancelling task '{}'...",
                    id
                ))),
                _ => Ok(CommandOutput::message(
                    "Usage: /tasks [list|cancel <id>]",
                )),
            }
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tasks_list() {
        let result = (TASKS.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("No active tasks"));
    }
}
