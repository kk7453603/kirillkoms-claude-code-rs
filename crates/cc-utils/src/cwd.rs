use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone)]
pub struct WorkingDirectory {
    inner: Arc<RwLock<PathBuf>>,
    original: PathBuf,
}

impl WorkingDirectory {
    pub fn new(path: PathBuf) -> Self {
        Self {
            inner: Arc::new(RwLock::new(path.clone())),
            original: path,
        }
    }

    pub fn current(&self) -> PathBuf {
        self.inner.read().unwrap().clone()
    }

    pub fn original(&self) -> &Path {
        &self.original
    }

    pub fn set(&self, path: PathBuf) {
        let mut guard = self.inner.write().unwrap();
        *guard = path;
    }

    /// Restore to original working directory.
    pub fn reset(&self) {
        self.set(self.original.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let wd = WorkingDirectory::new(PathBuf::from("/home/user"));
        assert_eq!(wd.current(), PathBuf::from("/home/user"));
        assert_eq!(wd.original(), Path::new("/home/user"));
    }

    #[test]
    fn test_set() {
        let wd = WorkingDirectory::new(PathBuf::from("/home/user"));
        wd.set(PathBuf::from("/tmp"));
        assert_eq!(wd.current(), PathBuf::from("/tmp"));
        // original should not change
        assert_eq!(wd.original(), Path::new("/home/user"));
    }

    #[test]
    fn test_reset() {
        let wd = WorkingDirectory::new(PathBuf::from("/home/user"));
        wd.set(PathBuf::from("/tmp"));
        assert_eq!(wd.current(), PathBuf::from("/tmp"));

        wd.reset();
        assert_eq!(wd.current(), PathBuf::from("/home/user"));
    }

    #[test]
    fn test_clone_shares_state() {
        let wd1 = WorkingDirectory::new(PathBuf::from("/home/user"));
        let wd2 = wd1.clone();

        wd1.set(PathBuf::from("/tmp"));
        assert_eq!(wd2.current(), PathBuf::from("/tmp"));
    }

    #[test]
    fn test_thread_safety() {
        let wd = WorkingDirectory::new(PathBuf::from("/start"));
        let wd_clone = wd.clone();

        let handle = std::thread::spawn(move || {
            wd_clone.set(PathBuf::from("/from_thread"));
        });

        handle.join().unwrap();
        assert_eq!(wd.current(), PathBuf::from("/from_thread"));
    }
}
