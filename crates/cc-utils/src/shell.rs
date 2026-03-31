use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use tokio::process::Command;

#[derive(Debug, Clone)]
pub struct ShellOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub timed_out: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum ShellError {
    #[error("Command timed out after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },
    #[error("Command failed: {0}")]
    ExecutionFailed(#[from] std::io::Error),
    #[error("Invalid command: {message}")]
    InvalidCommand { message: String },
}

/// Execute a shell command with timeout.
pub async fn execute_shell(
    command: &str,
    cwd: &Path,
    timeout: Duration,
    env: Option<&HashMap<String, String>>,
) -> Result<ShellOutput, ShellError> {
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(command).current_dir(cwd);

    if let Some(env_vars) = env {
        for (key, value) in env_vars {
            cmd.env(key, value);
        }
    }

    let result = tokio::time::timeout(timeout, cmd.output()).await;

    match result {
        Ok(Ok(output)) => Ok(ShellOutput {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
            timed_out: false,
        }),
        Ok(Err(e)) => Err(ShellError::ExecutionFailed(e)),
        Err(_) => Err(ShellError::Timeout {
            timeout_ms: timeout.as_millis() as u64,
        }),
    }
}

/// Execute a command and return combined output.
pub async fn execute_command(
    program: &str,
    args: &[&str],
    cwd: &Path,
) -> Result<ShellOutput, ShellError> {
    let output = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .output()
        .await?;

    Ok(ShellOutput {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code().unwrap_or(-1),
        timed_out: false,
    })
}

/// Check if a command is read-only (heuristic).
pub fn is_read_only_command(command: &str) -> bool {
    let trimmed = command.trim();
    let read_only_prefixes: &[&str] = &[
        "ls",
        "cat",
        "echo",
        "pwd",
        "which",
        "find",
        "grep",
        "head",
        "tail",
        "wc",
        "file",
        "stat",
        "readlink",
        "realpath",
        "type",
        "git status",
        "git log",
        "git diff",
        "git show",
        "git branch",
        "cargo check",
        "cargo test",
        "npm test",
        "npm run test",
        "env",
        "printenv",
        "whoami",
        "hostname",
        "uname",
        "date",
        "df",
        "du",
        "free",
        "id",
        "less",
        "more",
        "diff",
        "sort",
        "uniq",
        "tr",
        "cut",
        "tee",
        "test",
        "true",
        "false",
    ];

    for prefix in read_only_prefixes {
        if trimmed == *prefix {
            return true;
        }
        // Match "prefix " or "prefix\t" (command with arguments)
        if trimmed.starts_with(prefix) {
            let rest = &trimmed[prefix.len()..];
            if rest.starts_with(' ') || rest.starts_with('\t') {
                return true;
            }
        }
    }
    false
}

/// Check if a command is potentially destructive.
pub fn is_destructive_command(command: &str) -> bool {
    let trimmed = command.trim();
    let lower = trimmed.to_lowercase();

    let destructive_patterns: &[&str] = &[
        "rm ",
        "rm\t",
        "rmdir ",
        "rmdir\t",
        "git push --force",
        "git push -f",
        "git reset --hard",
        "git clean -f",
        "git checkout .",
        "git restore .",
        "drop table",
        "drop database",
        "truncate table",
        "delete from",
        "format ",
        "mkfs",
        "dd ",
        "dd\t",
        "> /dev/",
        "chmod -r",
        "chown -r",
        ":(){ :|:& };:",
    ];

    // Exact matches
    if trimmed == "rm" || trimmed == "rmdir" {
        return true;
    }

    for pattern in destructive_patterns {
        if lower.contains(pattern) {
            return true;
        }
    }
    false
}

/// Parse a shell command string into program and args.
pub fn parse_command(command: &str) -> Result<(String, Vec<String>), ShellError> {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return Err(ShellError::InvalidCommand {
            message: "Empty command".to_string(),
        });
    }

    let words = shell_words::split(trimmed).map_err(|e| ShellError::InvalidCommand {
        message: format!("Failed to parse command: {}", e),
    })?;

    if words.is_empty() {
        return Err(ShellError::InvalidCommand {
            message: "Empty command after parsing".to_string(),
        });
    }

    let program = words[0].clone();
    let args = words[1..].to_vec();
    Ok((program, args))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_read_only_basic_commands() {
        assert!(is_read_only_command("ls"));
        assert!(is_read_only_command("ls -la"));
        assert!(is_read_only_command("cat foo.txt"));
        assert!(is_read_only_command("echo hello"));
        assert!(is_read_only_command("pwd"));
        assert!(is_read_only_command("which git"));
        assert!(is_read_only_command("find . -name '*.rs'"));
        assert!(is_read_only_command("grep pattern file"));
        assert!(is_read_only_command("head -n 10 file"));
        assert!(is_read_only_command("tail -f log"));
        assert!(is_read_only_command("wc -l file"));
        assert!(is_read_only_command("file image.png"));
        assert!(is_read_only_command("stat file.txt"));
    }

    #[test]
    fn test_is_read_only_git_commands() {
        assert!(is_read_only_command("git status"));
        assert!(is_read_only_command("git log --oneline"));
        assert!(is_read_only_command("git diff HEAD"));
        assert!(is_read_only_command("git show HEAD"));
        assert!(is_read_only_command("git branch -a"));
    }

    #[test]
    fn test_is_read_only_returns_false() {
        assert!(!is_read_only_command("rm -rf /"));
        assert!(!is_read_only_command("git push"));
        assert!(!is_read_only_command("cargo build"));
        assert!(!is_read_only_command("mkdir foo"));
        assert!(!is_read_only_command("mv a b"));
        assert!(!is_read_only_command("cp a b"));
    }

    #[test]
    fn test_is_read_only_with_whitespace() {
        assert!(is_read_only_command("  ls -la  "));
        assert!(is_read_only_command("  git status  "));
    }

    #[test]
    fn test_is_destructive_rm() {
        assert!(is_destructive_command("rm file.txt"));
        assert!(is_destructive_command("rm -rf /"));
        assert!(is_destructive_command("rm"));
        assert!(is_destructive_command("rmdir empty_dir"));
    }

    #[test]
    fn test_is_destructive_git() {
        assert!(is_destructive_command("git push --force"));
        assert!(is_destructive_command("git push -f origin main"));
        assert!(is_destructive_command("git reset --hard HEAD~1"));
        assert!(is_destructive_command("git clean -fd"));
    }

    #[test]
    fn test_is_destructive_sql() {
        assert!(is_destructive_command("DROP TABLE users"));
        assert!(is_destructive_command("DELETE FROM users WHERE 1=1"));
        assert!(is_destructive_command("TRUNCATE TABLE logs"));
    }

    #[test]
    fn test_is_destructive_returns_false() {
        assert!(!is_destructive_command("ls -la"));
        assert!(!is_destructive_command("git status"));
        assert!(!is_destructive_command("cat file"));
    }

    #[test]
    fn test_parse_command_simple() {
        let (prog, args) = parse_command("ls -la").unwrap();
        assert_eq!(prog, "ls");
        assert_eq!(args, vec!["-la"]);
    }

    #[test]
    fn test_parse_command_with_quotes() {
        let (prog, args) = parse_command("echo 'hello world'").unwrap();
        assert_eq!(prog, "echo");
        assert_eq!(args, vec!["hello world"]);
    }

    #[test]
    fn test_parse_command_empty() {
        assert!(parse_command("").is_err());
        assert!(parse_command("   ").is_err());
    }

    #[test]
    fn test_parse_command_no_args() {
        let (prog, args) = parse_command("pwd").unwrap();
        assert_eq!(prog, "pwd");
        assert!(args.is_empty());
    }

    #[test]
    fn test_parse_command_double_quotes() {
        let (prog, args) = parse_command(r#"echo "hello world""#).unwrap();
        assert_eq!(prog, "echo");
        assert_eq!(args, vec!["hello world"]);
    }

    #[tokio::test]
    async fn test_execute_command_simple() {
        let output = execute_command("echo", &["hello"], Path::new("/tmp"))
            .await
            .unwrap();
        assert_eq!(output.stdout.trim(), "hello");
        assert_eq!(output.exit_code, 0);
        assert!(!output.timed_out);
    }

    #[tokio::test]
    async fn test_execute_shell_simple() {
        let output = execute_shell(
            "echo test",
            Path::new("/tmp"),
            Duration::from_secs(5),
            None,
        )
        .await
        .unwrap();
        assert_eq!(output.stdout.trim(), "test");
        assert_eq!(output.exit_code, 0);
    }

    #[tokio::test]
    async fn test_execute_shell_with_env() {
        let mut env = HashMap::new();
        env.insert("MY_VAR".to_string(), "my_value".to_string());
        let output = execute_shell(
            "echo $MY_VAR",
            Path::new("/tmp"),
            Duration::from_secs(5),
            Some(&env),
        )
        .await
        .unwrap();
        assert_eq!(output.stdout.trim(), "my_value");
    }

    #[tokio::test]
    async fn test_execute_shell_nonzero_exit() {
        let output = execute_shell(
            "exit 42",
            Path::new("/tmp"),
            Duration::from_secs(5),
            None,
        )
        .await
        .unwrap();
        assert_eq!(output.exit_code, 42);
    }
}
