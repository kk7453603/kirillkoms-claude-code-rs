use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct WorktreeInfo {
    pub path: PathBuf,
    pub branch: String,
    pub is_main: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum WorktreeError {
    #[error("Git error: {0}")]
    Git(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Create a git worktree for the given branch name.
/// The worktree is created in a `.worktrees` subdirectory of the repo root.
pub async fn create_worktree(
    repo_root: &Path,
    branch_name: &str,
) -> Result<PathBuf, WorktreeError> {
    let worktree_dir = repo_root.join(".worktrees").join(branch_name);

    let output = tokio::process::Command::new("git")
        .args(["worktree", "add", "-b", branch_name])
        .arg(&worktree_dir)
        .current_dir(repo_root)
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(WorktreeError::Git(stderr.to_string()));
    }

    Ok(worktree_dir)
}

/// Remove a git worktree at the given path.
pub async fn remove_worktree(worktree_path: &Path) -> Result<(), WorktreeError> {
    let output = tokio::process::Command::new("git")
        .args(["worktree", "remove", "--force"])
        .arg(worktree_path)
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(WorktreeError::Git(stderr.to_string()));
    }

    Ok(())
}

/// List all git worktrees for the repository at `repo_root`.
pub async fn list_worktrees(repo_root: &Path) -> Result<Vec<WorktreeInfo>, WorktreeError> {
    let output = tokio::process::Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .current_dir(repo_root)
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(WorktreeError::Git(stderr.to_string()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut worktrees = Vec::new();
    let mut current_path: Option<PathBuf> = None;
    let mut current_branch = String::new();
    let mut is_bare = false;

    for line in stdout.lines() {
        if let Some(path_str) = line.strip_prefix("worktree ") {
            // Save previous worktree if any
            if let Some(path) = current_path.take()
                && !is_bare {
                    worktrees.push(WorktreeInfo {
                        is_main: worktrees.is_empty(),
                        path,
                        branch: std::mem::take(&mut current_branch),
                    });
                }
            current_path = Some(PathBuf::from(path_str));
            is_bare = false;
        } else if let Some(branch_ref) = line.strip_prefix("branch ") {
            current_branch = branch_ref
                .strip_prefix("refs/heads/")
                .unwrap_or(branch_ref)
                .to_string();
        } else if line == "bare" {
            is_bare = true;
        }
    }

    // Push the last one
    if let Some(path) = current_path
        && !is_bare {
            worktrees.push(WorktreeInfo {
                is_main: worktrees.is_empty(),
                path,
                branch: current_branch,
            });
        }

    Ok(worktrees)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worktree_info() {
        let info = WorktreeInfo {
            path: PathBuf::from("/tmp/wt"),
            branch: "feature".to_string(),
            is_main: false,
        };
        assert_eq!(info.branch, "feature");
        assert!(!info.is_main);
    }

    #[test]
    fn test_worktree_error_display() {
        let err = WorktreeError::Git("not a git repo".to_string());
        assert_eq!(err.to_string(), "Git error: not a git repo");
    }
}
