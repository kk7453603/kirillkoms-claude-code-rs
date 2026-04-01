/// A local sub-agent task that runs in the same process.
#[derive(Debug, Clone)]
pub struct LocalAgentTask {
    pub id: String,
    pub prompt: String,
    pub status: super::types::TaskStatus,
    pub result: Option<String>,
}

impl LocalAgentTask {
    pub fn new(id: String, prompt: String) -> Self {
        Self {
            id,
            prompt,
            status: super::types::TaskStatus::Pending,
            result: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::TaskStatus;

    #[test]
    fn test_new_agent_task() {
        let task = LocalAgentTask::new("a1".to_string(), "Fix the bug".to_string());
        assert_eq!(task.id, "a1");
        assert_eq!(task.prompt, "Fix the bug");
        assert_eq!(task.status, TaskStatus::Pending);
        assert!(task.result.is_none());
    }
}
