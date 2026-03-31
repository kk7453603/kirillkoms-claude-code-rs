use std::path::{Path, PathBuf};

use crate::shell::execute_command;

#[derive(Debug, thiserror::Error)]
pub enum GitError {
    #[error("Not a git repository")]
    NotARepo,
    #[error("Git command failed: {message}")]
    CommandFailed { message: String },
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Get current branch name.
pub async fn current_branch(cwd: &Path) -> Result<String, GitError> {
    let output = execute_command("git", &["rev-parse", "--abbrev-ref", "HEAD"], cwd)
        .await
        .map_err(|e| GitError::CommandFailed {
            message: e.to_string(),
        })?;

    if output.exit_code != 0 {
        return Err(GitError::CommandFailed {
            message: output.stderr.trim().to_string(),
        });
    }

    Ok(output.stdout.trim().to_string())
}

/// Get short git status.
pub async fn git_status(cwd: &Path) -> Result<String, GitError> {
    let output = execute_command("git", &["status", "--short"], cwd)
        .await
        .map_err(|e| GitError::CommandFailed {
            message: e.to_string(),
        })?;

    if output.exit_code != 0 {
        return Err(GitError::CommandFailed {
            message: output.stderr.trim().to_string(),
        });
    }

    Ok(output.stdout)
}

/// Get git diff (staged + unstaged).
pub async fn git_diff(cwd: &Path) -> Result<String, GitError> {
    let output = execute_command("git", &["diff", "HEAD"], cwd)
        .await
        .map_err(|e| GitError::CommandFailed {
            message: e.to_string(),
        })?;

    if output.exit_code != 0 {
        // If HEAD doesn't exist (fresh repo), try plain diff
        let output2 = execute_command("git", &["diff"], cwd)
            .await
            .map_err(|e| GitError::CommandFailed {
                message: e.to_string(),
            })?;
        return Ok(output2.stdout);
    }

    Ok(output.stdout)
}

/// Get recent commit log (last N).
pub async fn git_log(cwd: &Path, count: usize) -> Result<String, GitError> {
    let count_str = format!("-{}", count);
    let output = execute_command(
        "git",
        &["log", "--oneline", &count_str],
        cwd,
    )
    .await
    .map_err(|e| GitError::CommandFailed {
        message: e.to_string(),
    })?;

    if output.exit_code != 0 {
        return Err(GitError::CommandFailed {
            message: output.stderr.trim().to_string(),
        });
    }

    Ok(output.stdout)
}

/// Check if directory is a git repository.
pub async fn is_git_repo(cwd: &Path) -> bool {
    let result = execute_command("git", &["rev-parse", "--is-inside-work-tree"], cwd).await;
    match result {
        Ok(output) => output.exit_code == 0 && output.stdout.trim() == "true",
        Err(_) => false,
    }
}

/// Get repository root.
pub async fn repo_root(cwd: &Path) -> Result<PathBuf, GitError> {
    let output = execute_command("git", &["rev-parse", "--show-toplevel"], cwd)
        .await
        .map_err(|e| GitError::CommandFailed {
            message: e.to_string(),
        })?;

    if output.exit_code != 0 {
        return Err(GitError::NotARepo);
    }

    Ok(PathBuf::from(output.stdout.trim()))
}

/// Get list of changed files.
pub async fn changed_files(cwd: &Path) -> Result<Vec<String>, GitError> {
    let output = execute_command(
        "git",
        &["status", "--short", "--porcelain"],
        cwd,
    )
    .await
    .map_err(|e| GitError::CommandFailed {
        message: e.to_string(),
    })?;

    if output.exit_code != 0 {
        return Err(GitError::CommandFailed {
            message: output.stderr.trim().to_string(),
        });
    }

    let files: Vec<String> = output
        .stdout
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            // Status output format: "XY filename" where XY is 2 chars + space
            if line.len() > 3 {
                line[3..].to_string()
            } else {
                line.trim().to_string()
            }
        })
        .collect();

    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_is_git_repo_current_dir() {
        // The project root should be a git repo
        let project_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        assert!(is_git_repo(project_root).await);
    }

    #[tokio::test]
    async fn test_is_git_repo_tmp() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(!is_git_repo(tmp.path()).await);
    }

    #[tokio::test]
    async fn test_current_branch() {
        let project_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        let branch = current_branch(project_root).await;
        // Should succeed if we're in a git repo
        assert!(branch.is_ok());
        assert!(!branch.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_repo_root() {
        let project_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        let root = repo_root(project_root).await.unwrap();
        assert!(root.exists());
    }

    #[tokio::test]
    async fn test_repo_root_not_a_repo() {
        let tmp = tempfile::tempdir().unwrap();
        let result = repo_root(tmp.path()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_git_status() {
        let project_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        let status = git_status(project_root).await;
        assert!(status.is_ok());
    }

    #[tokio::test]
    async fn test_git_log() {
        let project_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        let log = git_log(project_root, 5).await;
        assert!(log.is_ok());
    }

    #[tokio::test]
    async fn test_changed_files() {
        let project_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        let files = changed_files(project_root).await;
        assert!(files.is_ok());
    }
}
