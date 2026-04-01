use std::path::Path;

/// Find a process by name, returning its PID.
/// Searches `/proc` on Linux or uses `pgrep` as fallback.
pub async fn find_process(name: &str) -> Option<u32> {
    // Try pgrep first (works on Linux and macOS)
    let output = tokio::process::Command::new("pgrep")
        .arg("-f")
        .arg(name)
        .output()
        .await
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .next()
        .and_then(|line| line.trim().parse::<u32>().ok())
}

/// Kill a process by PID.
pub async fn kill_process(pid: u32) -> Result<(), std::io::Error> {
    let output = tokio::process::Command::new("kill")
        .arg(pid.to_string())
        .output()
        .await?;

    if output.status.success() {
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!(
                "Failed to kill process {}: {}",
                pid,
                String::from_utf8_lossy(&output.stderr)
            ),
        ))
    }
}

/// Check if a process with the given PID is currently running.
/// Uses the `/proc` filesystem on Linux, or `kill -0` as fallback.
pub fn is_process_running(pid: u32) -> bool {
    let proc_path = format!("/proc/{}", pid);
    if Path::new(&proc_path).exists() {
        return true;
    }
    // Fallback: use kill -0 (sends no signal, just checks existence)
    std::process::Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_process_is_running() {
        let pid = std::process::id();
        assert!(is_process_running(pid));
    }

    #[test]
    fn nonexistent_process_not_running() {
        // PID 4294967 is very unlikely to exist
        assert!(!is_process_running(4_294_967));
    }

    #[tokio::test]
    async fn find_process_nonexistent() {
        let result = find_process("this_process_should_not_exist_12345").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn kill_nonexistent_process() {
        let result = kill_process(4_294_967).await;
        assert!(result.is_err());
    }

    #[test]
    fn is_process_running_pid_1() {
        // PID 1 (init/systemd) should always be running on Linux
        if cfg!(target_os = "linux") {
            assert!(is_process_running(1));
        }
    }
}
