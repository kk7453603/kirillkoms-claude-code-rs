/// A teammate task that delegates work to a remote agent or service.
#[derive(Debug, Clone)]
pub struct TeammateTask {
    pub id: String,
    pub prompt: String,
    pub status: super::types::TaskStatus,
    pub result: Option<String>,
}

impl TeammateTask {
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
    fn test_new_teammate_task() {
        let task = TeammateTask::new("tm1".to_string(), "Deploy to staging".to_string());
        assert_eq!(task.id, "tm1");
        assert_eq!(task.prompt, "Deploy to staging");
        assert_eq!(task.status, TaskStatus::Pending);
        assert!(task.result.is_none());
    }
}
