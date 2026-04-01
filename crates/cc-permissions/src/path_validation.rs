/// Filesystem path security validation.
///
/// Validates that paths are within allowed directories and detects sensitive files.
use std::path::{Component, Path, PathBuf};

/// Validate that a path is within the project root or one of the additional allowed directories.
///
/// The path and all directory roots are normalized before comparison.
pub fn is_path_allowed(path: &Path, project_root: &Path, additional_dirs: &[PathBuf]) -> bool {
    let normalized = normalize_path(path, project_root);

    let norm_root = normalize_path(project_root, project_root);
    if normalized.starts_with(&norm_root) {
        return true;
    }

    for dir in additional_dirs {
        let norm_dir = normalize_path(dir, project_root);
        if normalized.starts_with(&norm_dir) {
            return true;
        }
    }

    false
}

/// Normalize a path by resolving `.` and `..` components logically (without touching the filesystem).
///
/// If the path is relative, it is joined onto `cwd` first.
pub fn normalize_path(path: &Path, cwd: &Path) -> PathBuf {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    };

    let mut components = Vec::new();
    for component in absolute.components() {
        match component {
            Component::ParentDir => {
                // Pop the last normal component, but never go above root
                if !components.is_empty() {
                    let last = components.last().cloned();
                    if let Some(Component::Normal(_)) = last {
                        components.pop();
                    }
                }
            }
            Component::CurDir => {
                // Skip
            }
            other => {
                components.push(other);
            }
        }
    }

    if components.is_empty() {
        PathBuf::from("/")
    } else {
        components.iter().collect()
    }
}

/// Check if a path appears to reference a sensitive file or directory.
///
/// Sensitive paths include credentials, private keys, environment files, and
/// other security-relevant locations.
pub fn is_sensitive_path(path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    let path_lower = path_str.to_lowercase();

    // Check file name
    if let Some(file_name) = path.file_name() {
        let name = file_name.to_string_lossy();
        let name_lower = name.to_lowercase();

        // Exact file name matches
        let sensitive_filenames = [
            ".env",
            ".env.local",
            ".env.production",
            ".env.development",
            ".env.staging",
            ".env.test",
            "credentials.json",
            "credentials.yaml",
            "credentials.yml",
            "service-account.json",
            "serviceaccount.json",
            "secrets.json",
            "secrets.yaml",
            "secrets.yml",
            ".netrc",
            ".npmrc",
            ".pypirc",
            "id_rsa",
            "id_ed25519",
            "id_ecdsa",
            "id_dsa",
            ".htpasswd",
            ".pgpass",
            "shadow",
            "passwd",
            "master.key",
            "token.json",
        ];

        for &sensitive in &sensitive_filenames {
            if name_lower == sensitive {
                return true;
            }
        }

        // File name patterns
        if name_lower.ends_with(".pem")
            || name_lower.ends_with(".key")
            || name_lower.ends_with(".p12")
            || name_lower.ends_with(".pfx")
            || name_lower.ends_with(".keystore")
            || name_lower.ends_with(".jks")
        {
            return true;
        }

        // .env with any suffix
        if name_lower.starts_with(".env.") || name_lower == ".env" {
            return true;
        }
    }

    // Check path components for sensitive directories
    let sensitive_dirs = [".ssh", ".aws", ".gnupg", ".gpg", ".kube", ".docker"];
    for component in path.components() {
        if let Component::Normal(os_str) = component {
            let comp = os_str.to_string_lossy().to_lowercase();
            for &dir in &sensitive_dirs {
                if comp == dir {
                    return true;
                }
            }
        }
    }

    // Check for well-known absolute sensitive paths
    let sensitive_prefixes = ["/etc/shadow", "/etc/passwd", "/etc/sudoers"];
    for &prefix in &sensitive_prefixes {
        if path_lower.starts_with(prefix) {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- is_path_allowed tests ----

    #[test]
    fn test_path_within_project_root() {
        let root = Path::new("/home/user/project");
        let path = Path::new("/home/user/project/src/main.rs");
        assert!(is_path_allowed(path, root, &[]));
    }

    #[test]
    fn test_path_is_project_root() {
        let root = Path::new("/home/user/project");
        assert!(is_path_allowed(root, root, &[]));
    }

    #[test]
    fn test_path_outside_project_root() {
        let root = Path::new("/home/user/project");
        let path = Path::new("/home/user/other/file.txt");
        assert!(!is_path_allowed(path, root, &[]));
    }

    #[test]
    fn test_path_in_additional_dir() {
        let root = Path::new("/home/user/project");
        let additional = vec![PathBuf::from("/tmp/allowed")];
        let path = Path::new("/tmp/allowed/file.txt");
        assert!(is_path_allowed(path, root, &additional));
    }

    #[test]
    fn test_path_traversal_blocked() {
        let root = Path::new("/home/user/project");
        let path = Path::new("/home/user/project/../other/file.txt");
        assert!(!is_path_allowed(path, root, &[]));
    }

    #[test]
    fn test_path_with_dot_components() {
        let root = Path::new("/home/user/project");
        let path = Path::new("/home/user/project/./src/../src/main.rs");
        assert!(is_path_allowed(path, root, &[]));
    }

    #[test]
    fn test_relative_path_resolved_against_root() {
        let root = Path::new("/home/user/project");
        let path = Path::new("src/main.rs");
        assert!(is_path_allowed(path, root, &[]));
    }

    #[test]
    fn test_relative_traversal_outside() {
        let root = Path::new("/home/user/project");
        let path = Path::new("../../etc/passwd");
        assert!(!is_path_allowed(path, root, &[]));
    }

    // ---- normalize_path tests ----

    #[test]
    fn test_normalize_absolute() {
        let result = normalize_path(Path::new("/a/b/c"), Path::new("/cwd"));
        assert_eq!(result, PathBuf::from("/a/b/c"));
    }

    #[test]
    fn test_normalize_relative() {
        let result = normalize_path(Path::new("src/main.rs"), Path::new("/home/user/project"));
        assert_eq!(result, PathBuf::from("/home/user/project/src/main.rs"));
    }

    #[test]
    fn test_normalize_parent_dir() {
        let result = normalize_path(Path::new("/a/b/../c"), Path::new("/"));
        assert_eq!(result, PathBuf::from("/a/c"));
    }

    #[test]
    fn test_normalize_current_dir() {
        let result = normalize_path(Path::new("/a/./b/./c"), Path::new("/"));
        assert_eq!(result, PathBuf::from("/a/b/c"));
    }

    #[test]
    fn test_normalize_multiple_parents() {
        let result = normalize_path(Path::new("/a/b/c/../../d"), Path::new("/"));
        assert_eq!(result, PathBuf::from("/a/d"));
    }

    #[test]
    fn test_normalize_parent_at_root() {
        // Going above root should stay at root
        let result = normalize_path(Path::new("/a/../.."), Path::new("/"));
        assert_eq!(result, PathBuf::from("/"));
    }

    // ---- is_sensitive_path tests ----

    #[test]
    fn test_env_file_sensitive() {
        assert!(is_sensitive_path(Path::new(".env")));
        assert!(is_sensitive_path(Path::new("/project/.env")));
        assert!(is_sensitive_path(Path::new(".env.local")));
        assert!(is_sensitive_path(Path::new(".env.production")));
        assert!(is_sensitive_path(Path::new(".env.staging")));
    }

    #[test]
    fn test_credentials_sensitive() {
        assert!(is_sensitive_path(Path::new("credentials.json")));
        assert!(is_sensitive_path(Path::new("/app/credentials.yaml")));
        assert!(is_sensitive_path(Path::new("secrets.yml")));
    }

    #[test]
    fn test_ssh_dir_sensitive() {
        assert!(is_sensitive_path(Path::new("/home/user/.ssh/id_rsa")));
        assert!(is_sensitive_path(Path::new("/home/user/.ssh/config")));
        assert!(is_sensitive_path(Path::new(".ssh/authorized_keys")));
    }

    #[test]
    fn test_aws_dir_sensitive() {
        assert!(is_sensitive_path(Path::new("/home/user/.aws/credentials")));
        assert!(is_sensitive_path(Path::new(".aws/config")));
    }

    #[test]
    fn test_private_key_files_sensitive() {
        assert!(is_sensitive_path(Path::new("id_rsa")));
        assert!(is_sensitive_path(Path::new("id_ed25519")));
        assert!(is_sensitive_path(Path::new("server.key")));
        assert!(is_sensitive_path(Path::new("cert.pem")));
        assert!(is_sensitive_path(Path::new("keystore.p12")));
    }

    #[test]
    fn test_system_files_sensitive() {
        assert!(is_sensitive_path(Path::new("/etc/shadow")));
        assert!(is_sensitive_path(Path::new("/etc/passwd")));
        assert!(is_sensitive_path(Path::new("/etc/sudoers")));
    }

    #[test]
    fn test_normal_files_not_sensitive() {
        assert!(!is_sensitive_path(Path::new("src/main.rs")));
        assert!(!is_sensitive_path(Path::new(
            "/home/user/project/README.md"
        )));
        assert!(!is_sensitive_path(Path::new("Cargo.toml")));
        assert!(!is_sensitive_path(Path::new("package.json")));
        assert!(!is_sensitive_path(Path::new(".gitignore")));
    }

    #[test]
    fn test_kube_dir_sensitive() {
        assert!(is_sensitive_path(Path::new("/home/user/.kube/config")));
    }

    #[test]
    fn test_docker_dir_sensitive() {
        assert!(is_sensitive_path(Path::new(
            "/home/user/.docker/config.json"
        )));
    }

    #[test]
    fn test_gnupg_dir_sensitive() {
        assert!(is_sensitive_path(Path::new(
            "/home/user/.gnupg/trustdb.gpg"
        )));
    }

    #[test]
    fn test_htpasswd_sensitive() {
        assert!(is_sensitive_path(Path::new(".htpasswd")));
    }

    #[test]
    fn test_netrc_sensitive() {
        assert!(is_sensitive_path(Path::new(".netrc")));
    }

    #[test]
    fn test_npmrc_sensitive() {
        assert!(is_sensitive_path(Path::new(".npmrc")));
    }

    #[test]
    fn test_master_key_sensitive() {
        assert!(is_sensitive_path(Path::new("master.key")));
        assert!(is_sensitive_path(Path::new("config/master.key")));
    }

    #[test]
    fn test_case_insensitive_filenames() {
        // Our check lowercases, so mixed case should still match
        assert!(is_sensitive_path(Path::new("Credentials.JSON")));
        assert!(is_sensitive_path(Path::new("SECRETS.YML")));
    }

    #[test]
    fn test_env_with_unknown_suffix() {
        // .env.anything should be sensitive
        assert!(is_sensitive_path(Path::new(".env.custom")));
        assert!(is_sensitive_path(Path::new(".env.backup")));
    }
}
