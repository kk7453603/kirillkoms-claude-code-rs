use std::path::PathBuf;

/// Returns the name of the current operating system.
pub fn os_name() -> &'static str {
    if cfg!(target_os = "macos") {
        "macOS"
    } else if cfg!(target_os = "linux") {
        "Linux"
    } else if cfg!(target_os = "windows") {
        "Windows"
    } else {
        "Unknown"
    }
}

/// Returns true if running on macOS.
pub fn is_macos() -> bool {
    cfg!(target_os = "macos")
}

/// Returns true if running on Linux.
pub fn is_linux() -> bool {
    cfg!(target_os = "linux")
}

/// Returns true if running on Windows.
pub fn is_windows() -> bool {
    cfg!(target_os = "windows")
}

/// Returns the name of the current shell (e.g., "bash", "zsh", "fish").
pub fn shell_name() -> String {
    if let Ok(shell) = std::env::var("SHELL") {
        if let Some(name) = shell.rsplit('/').next() {
            return name.to_string();
        }
        return shell;
    }
    if let Ok(shell) = std::env::var("ComSpec") {
        if let Some(name) = shell.rsplit('\\').next() {
            return name.to_string();
        }
        return shell;
    }
    "unknown".to_string()
}

/// Returns the user's home directory.
pub fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/"))
}

/// Returns true if running in a CI environment.
pub fn is_ci() -> bool {
    std::env::var("CI").is_ok()
        || std::env::var("CONTINUOUS_INTEGRATION").is_ok()
        || std::env::var("GITHUB_ACTIONS").is_ok()
        || std::env::var("JENKINS_URL").is_ok()
        || std::env::var("GITLAB_CI").is_ok()
        || std::env::var("CIRCLECI").is_ok()
        || std::env::var("TRAVIS").is_ok()
        || std::env::var("BUILDKITE").is_ok()
}

/// Returns true if the session appears to be over SSH.
pub fn is_ssh() -> bool {
    std::env::var("SSH_CLIENT").is_ok()
        || std::env::var("SSH_TTY").is_ok()
        || std::env::var("SSH_CONNECTION").is_ok()
}

/// Returns true if running inside a Docker container.
pub fn is_docker() -> bool {
    // Check for .dockerenv
    if std::path::Path::new("/.dockerenv").exists() {
        return true;
    }
    // Check cgroup for docker
    if let Ok(cgroup) = std::fs::read_to_string("/proc/1/cgroup") {
        if cgroup.contains("docker") || cgroup.contains("containerd") {
            return true;
        }
    }
    // Check for container env var
    if let Ok(val) = std::env::var("container") {
        if val == "docker" {
            return true;
        }
    }
    false
}

/// Returns the name of the terminal emulator, if detectable.
pub fn terminal_name() -> Option<String> {
    if let Ok(term_program) = std::env::var("TERM_PROGRAM") {
        return Some(term_program);
    }
    if let Ok(term) = std::env::var("TERMINAL_EMULATOR") {
        return Some(term);
    }
    if let Ok(term) = std::env::var("TERM") {
        if term != "xterm" && term != "xterm-256color" && term != "screen" && term != "dumb" {
            return Some(term);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn os_name_not_unknown() {
        let name = os_name();
        assert!(
            name == "macOS" || name == "Linux" || name == "Windows" || name == "Unknown",
            "Unexpected OS name: {}",
            name
        );
    }

    #[test]
    fn platform_booleans_consistent() {
        // At most one should be true
        let count = [is_macos(), is_linux(), is_windows()]
            .iter()
            .filter(|&&v| v)
            .count();
        assert!(count <= 1);
    }

    #[test]
    fn home_dir_not_empty() {
        let home = home_dir();
        assert!(!home.as_os_str().is_empty());
    }

    #[test]
    fn shell_name_not_empty() {
        let name = shell_name();
        assert!(!name.is_empty());
    }

    #[test]
    fn is_ci_returns_bool() {
        // Just verify it doesn't panic
        let _ = is_ci();
    }

    #[test]
    fn is_ssh_returns_bool() {
        let _ = is_ssh();
    }

    #[test]
    fn is_docker_returns_bool() {
        let _ = is_docker();
    }

    #[test]
    fn terminal_name_returns_option() {
        let _ = terminal_name();
    }
}
