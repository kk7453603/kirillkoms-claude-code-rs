use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
    pub id: String,
    pub name: String,
    pub status: TaskStatus,
    pub created_at: String,
    pub description: Option<String>,
    pub output: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TaskManager {
    tasks: Vec<TaskInfo>,
}

impl TaskManager {
    pub fn new() -> Self {
        Self { tasks: vec![] }
    }

    pub fn add_task(&mut self, task: TaskInfo) -> &TaskInfo {
        self.tasks.push(task);
        self.tasks.last().unwrap()
    }

    pub fn get_task(&self, id: &str) -> Option<&TaskInfo> {
        self.tasks.iter().find(|t| t.id == id)
    }

    pub fn update_status(&mut self, id: &str, status: TaskStatus) -> bool {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
            task.status = status;
            true
        } else {
            false
        }
    }

    pub fn list_tasks(&self) -> &[TaskInfo] {
        &self.tasks
    }

    pub fn set_output(&mut self, id: &str, output: String) -> bool {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
            task.output = Some(output);
            true
        } else {
            false
        }
    }

    pub fn tasks_by_status(&self, status: TaskStatus) -> Vec<&TaskInfo> {
        self.tasks.iter().filter(|t| t.status == status).collect()
    }

    pub fn remove_task(&mut self, id: &str) -> Option<TaskInfo> {
        if let Some(pos) = self.tasks.iter().position(|t| t.id == id) {
            Some(self.tasks.remove(pos))
        } else {
            None
        }
    }
}

impl Default for TaskManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_task(id: &str, name: &str) -> TaskInfo {
        TaskInfo {
            id: id.to_string(),
            name: name.to_string(),
            status: TaskStatus::Pending,
            created_at: "2025-01-01T00:00:00Z".to_string(),
            description: None,
            output: None,
        }
    }

    #[test]
    fn test_add_and_get_task() {
        let mut mgr = TaskManager::new();
        mgr.add_task(make_task("1", "test task"));

        let task = mgr.get_task("1").unwrap();
        assert_eq!(task.name, "test task");
        assert_eq!(task.status, TaskStatus::Pending);
    }

    #[test]
    fn test_get_nonexistent_task() {
        let mgr = TaskManager::new();
        assert!(mgr.get_task("nope").is_none());
    }

    #[test]
    fn test_update_status() {
        let mut mgr = TaskManager::new();
        mgr.add_task(make_task("1", "task"));

        assert!(mgr.update_status("1", TaskStatus::Running));
        assert_eq!(mgr.get_task("1").unwrap().status, TaskStatus::Running);

        assert!(mgr.update_status("1", TaskStatus::Completed));
        assert_eq!(mgr.get_task("1").unwrap().status, TaskStatus::Completed);
    }

    #[test]
    fn test_update_status_nonexistent() {
        let mut mgr = TaskManager::new();
        assert!(!mgr.update_status("nope", TaskStatus::Failed));
    }

    #[test]
    fn test_list_tasks() {
        let mut mgr = TaskManager::new();
        assert!(mgr.list_tasks().is_empty());

        mgr.add_task(make_task("1", "a"));
        mgr.add_task(make_task("2", "b"));
        assert_eq!(mgr.list_tasks().len(), 2);
    }

    #[test]
    fn test_remove_task() {
        let mut mgr = TaskManager::new();
        mgr.add_task(make_task("1", "task"));

        let removed = mgr.remove_task("1").unwrap();
        assert_eq!(removed.id, "1");
        assert!(mgr.get_task("1").is_none());
        assert!(mgr.list_tasks().is_empty());
    }

    #[test]
    fn test_remove_nonexistent() {
        let mut mgr = TaskManager::new();
        assert!(mgr.remove_task("nope").is_none());
    }

    #[test]
    fn test_task_status_serialization() {
        let status = TaskStatus::Completed;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"completed\"");

        let deserialized: TaskStatus = serde_json::from_str("\"failed\"").unwrap();
        assert_eq!(deserialized, TaskStatus::Failed);
    }

    #[test]
    fn test_task_info_serialization() {
        let task = make_task("abc", "my task");
        let json = serde_json::to_string(&task).unwrap();
        let roundtrip: TaskInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip.id, "abc");
        assert_eq!(roundtrip.status, TaskStatus::Pending);
    }
}
