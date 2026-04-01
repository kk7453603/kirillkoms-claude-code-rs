use std::path::{Component, Path, PathBuf};

/// Expand `~` at the beginning of a path to the user's home directory.
pub fn expand_tilde(path: &str) -> PathBuf {
    if path == "~" {
        return home_dir_or_root();
    }
    if let Some(rest) = path.strip_prefix("~/") {
        return home_dir_or_root().join(rest);
    }
    PathBuf::from(path)
}

fn home_dir_or_root() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/"))
}

/// Compute a relative path from `base` to `path`.
/// If the paths share no common prefix, returns `path` as-is.
pub fn relative_path(path: &Path, base: &Path) -> PathBuf {
    // Normalize both paths
    let norm_path = normalize_path(path);
    let norm_base = normalize_path(base);

    // Find common prefix length
    let path_components: Vec<_> = norm_path.components().collect();
    let base_components: Vec<_> = norm_base.components().collect();

    let common_len = path_components
        .iter()
        .zip(base_components.iter())
        .take_while(|(a, b)| a == b)
        .count();

    if common_len == 0 {
        return norm_path;
    }

    let mut result = PathBuf::new();

    // Add ".." for each remaining component in base
    for _ in common_len..base_components.len() {
        result.push("..");
    }

    // Add remaining components from path
    for component in &path_components[common_len..] {
        result.push(component.as_os_str());
    }

    if result.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        result
    }
}

/// Check if a path component is hidden (starts with `.`).
pub fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map_or(false, |name| name.starts_with('.'))
}

/// Normalize a path by resolving `.` and `..` components without touching the filesystem.
pub fn normalize_path(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();

    for component in path.components() {
        match component {
            Component::ParentDir => {
                // Only pop if we have a normal component to pop
                if result
                    .components()
                    .last()
                    .map_or(false, |c| matches!(c, Component::Normal(_)))
                {
                    result.pop();
                } else {
                    result.push("..");
                }
            }
            Component::CurDir => {
                // Skip `.`
            }
            other => {
                result.push(other.as_os_str());
            }
        }
    }

    if result.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_tilde_home() {
        let expanded = expand_tilde("~/Documents");
        assert!(expanded.to_str().unwrap().ends_with("Documents"));
        assert!(!expanded.to_str().unwrap().starts_with('~'));
    }

    #[test]
    fn expand_tilde_no_tilde() {
        let expanded = expand_tilde("/usr/local/bin");
        assert_eq!(expanded, PathBuf::from("/usr/local/bin"));
    }

    #[test]
    fn expand_tilde_just_tilde() {
        let expanded = expand_tilde("~");
        assert!(!expanded.to_str().unwrap().contains('~'));
    }

    #[test]
    fn relative_path_basic() {
        let rel = relative_path(Path::new("/a/b/c"), Path::new("/a/b"));
        assert_eq!(rel, PathBuf::from("c"));
    }

    #[test]
    fn relative_path_up() {
        let rel = relative_path(Path::new("/a/b"), Path::new("/a/b/c"));
        assert_eq!(rel, PathBuf::from(".."));
    }

    #[test]
    fn is_hidden_dotfile() {
        assert!(is_hidden(Path::new(".gitignore")));
        assert!(is_hidden(Path::new("/home/user/.config")));
        assert!(!is_hidden(Path::new("README.md")));
        assert!(!is_hidden(Path::new("/home/user/Documents")));
    }

    #[test]
    fn normalize_path_dots() {
        assert_eq!(
            normalize_path(Path::new("/a/b/../c/./d")),
            PathBuf::from("/a/c/d")
        );
        assert_eq!(normalize_path(Path::new(".")), PathBuf::from("."));
    }
}
