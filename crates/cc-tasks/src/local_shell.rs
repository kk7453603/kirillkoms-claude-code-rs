use super::types::TaskStatus;

/// A background shell task
#[derive(Debug, Clone)]
pub struct LocalShellTask {
    pub id: String,
    pub command: String,
    pub status: TaskStatus,
    pub output: Option<String>,
}

impl LocalShellTask {
    pub fn new(id: String, command: String) -> Self {
        Self {
            id,
            command,
            status: TaskStatus::Pending,
            output: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_shell_task() {
        let task = LocalShellTask::new("t1".to_string(), "echo hello".to_string());
        assert_eq!(task.id, "t1");
        assert_eq!(task.command, "echo hello");
        assert_eq!(task.status, TaskStatus::Pending);
        assert!(task.output.is_none());
    }
}
