use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct FileState {
    pub content: String,
    pub modified_time: SystemTime,
    pub line_count: usize,
}

#[derive(Debug)]
pub struct FileStateCache {
    cache: HashMap<PathBuf, FileState>,
    max_entries: usize,
}

impl FileStateCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            cache: HashMap::new(),
            max_entries,
        }
    }

    pub fn get(&self, path: &Path) -> Option<&FileState> {
        self.cache.get(path)
    }

    pub fn insert(&mut self, path: PathBuf, state: FileState) {
        // If we're at capacity and this is a new key, evict the oldest entry
        if self.cache.len() >= self.max_entries && !self.cache.contains_key(&path) {
            // Evict the entry with the oldest modified_time
            if let Some(oldest_key) = self
                .cache
                .iter()
                .min_by_key(|(_, v)| v.modified_time)
                .map(|(k, _)| k.clone())
            {
                self.cache.remove(&oldest_key);
            }
        }
        self.cache.insert(path, state);
    }

    pub fn remove(&mut self, path: &Path) -> Option<FileState> {
        self.cache.remove(path)
    }

    /// Check if the cached state is stale by comparing modification times
    /// with the actual file on disk. Returns Ok(true) if the file has been
    /// modified since it was cached, Ok(false) if it hasn't, or an error
    /// if the file metadata cannot be read (e.g., file was deleted).
    pub fn is_stale(&self, path: &Path) -> Result<bool, std::io::Error> {
        match self.cache.get(path) {
            None => {
                // Not in cache at all; consider it stale
                Ok(true)
            }
            Some(state) => {
                let metadata = std::fs::metadata(path)?;
                let disk_modified = metadata.modified()?;
                Ok(disk_modified > state.modified_time)
            }
        }
    }

    pub fn clear(&mut self) {
        self.cache.clear();
    }

    pub fn len(&self) -> usize {
        self.cache.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn make_state(content: &str, time: SystemTime) -> FileState {
        FileState {
            content: content.to_string(),
            modified_time: time,
            line_count: content.lines().count(),
        }
    }

    #[test]
    fn test_new_cache_is_empty() {
        let cache = FileStateCache::new(10);
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_insert_and_get() {
        let mut cache = FileStateCache::new(10);
        let path = PathBuf::from("/tmp/test.txt");
        let state = make_state("hello\nworld", SystemTime::now());
        cache.insert(path.clone(), state);

        assert_eq!(cache.len(), 1);
        let retrieved = cache.get(&path).unwrap();
        assert_eq!(retrieved.content, "hello\nworld");
        assert_eq!(retrieved.line_count, 2);
    }

    #[test]
    fn test_get_missing() {
        let cache = FileStateCache::new(10);
        assert!(cache.get(Path::new("/nonexistent")).is_none());
    }

    #[test]
    fn test_remove() {
        let mut cache = FileStateCache::new(10);
        let path = PathBuf::from("/tmp/test.txt");
        cache.insert(path.clone(), make_state("content", SystemTime::now()));
        assert_eq!(cache.len(), 1);

        let removed = cache.remove(&path);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().content, "content");
        assert!(cache.is_empty());
    }

    #[test]
    fn test_remove_missing() {
        let mut cache = FileStateCache::new(10);
        assert!(cache.remove(Path::new("/nonexistent")).is_none());
    }

    #[test]
    fn test_clear() {
        let mut cache = FileStateCache::new(10);
        cache.insert(
            PathBuf::from("/a"),
            make_state("a", SystemTime::now()),
        );
        cache.insert(
            PathBuf::from("/b"),
            make_state("b", SystemTime::now()),
        );
        assert_eq!(cache.len(), 2);
        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_max_entries_eviction() {
        let mut cache = FileStateCache::new(2);
        let now = SystemTime::now();

        // Insert oldest first
        cache.insert(
            PathBuf::from("/a"),
            make_state("a", now - Duration::from_secs(10)),
        );
        cache.insert(
            PathBuf::from("/b"),
            make_state("b", now - Duration::from_secs(5)),
        );
        assert_eq!(cache.len(), 2);

        // Adding a third should evict the oldest (/a)
        cache.insert(PathBuf::from("/c"), make_state("c", now));
        assert_eq!(cache.len(), 2);
        assert!(cache.get(Path::new("/a")).is_none());
        assert!(cache.get(Path::new("/b")).is_some());
        assert!(cache.get(Path::new("/c")).is_some());
    }

    #[test]
    fn test_insert_same_key_no_eviction() {
        let mut cache = FileStateCache::new(2);
        let now = SystemTime::now();
        cache.insert(PathBuf::from("/a"), make_state("a1", now));
        cache.insert(PathBuf::from("/b"), make_state("b1", now));

        // Updating an existing key should not evict
        cache.insert(PathBuf::from("/a"), make_state("a2", now));
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.get(Path::new("/a")).unwrap().content, "a2");
    }

    #[test]
    fn test_is_stale_with_real_file() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();

        let mut cache = FileStateCache::new(10);
        // Cache with a time in the past
        let old_time = SystemTime::now() - Duration::from_secs(100);
        cache.insert(path.clone(), make_state("old", old_time));

        // File was written "now", cached time is in the past => stale
        assert!(cache.is_stale(&path).unwrap());
    }

    #[test]
    fn test_is_stale_not_in_cache() {
        let cache = FileStateCache::new(10);
        assert!(cache.is_stale(Path::new("/some/path")).unwrap());
    }

    #[test]
    fn test_is_stale_file_deleted() {
        let mut cache = FileStateCache::new(10);
        let path = PathBuf::from("/tmp/definitely_does_not_exist_abc123xyz");
        cache.insert(path.clone(), make_state("gone", SystemTime::now()));
        // File doesn't exist, should return an IO error
        assert!(cache.is_stale(&path).is_err());
    }
}
