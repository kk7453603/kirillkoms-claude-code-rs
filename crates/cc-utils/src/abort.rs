use tokio_util::sync::CancellationToken;

/// Create a linked cancellation token (child of parent).
/// When the parent is cancelled, the child will also be cancelled.
/// The child can also be cancelled independently without affecting the parent.
pub fn linked_token(parent: &CancellationToken) -> CancellationToken {
    parent.child_token()
}

/// Run a future with cancellation support.
/// Returns `Ok(value)` if the future completes before cancellation,
/// or `Err(Cancelled)` if the token is cancelled first.
pub async fn cancellable<F, T>(token: &CancellationToken, future: F) -> Result<T, Cancelled>
where
    F: std::future::Future<Output = T>,
{
    tokio::select! {
        result = future => Ok(result),
        _ = token.cancelled() => Err(Cancelled),
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Operation cancelled")]
pub struct Cancelled;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linked_token_parent_cancels_child() {
        let parent = CancellationToken::new();
        let child = linked_token(&parent);

        assert!(!child.is_cancelled());
        parent.cancel();
        assert!(child.is_cancelled());
    }

    #[test]
    fn test_linked_token_child_independent() {
        let parent = CancellationToken::new();
        let child = linked_token(&parent);

        child.cancel();
        assert!(child.is_cancelled());
        assert!(!parent.is_cancelled());
    }

    #[tokio::test]
    async fn test_cancellable_completes() {
        let token = CancellationToken::new();
        let result = cancellable(&token, async { 42 }).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_cancellable_cancelled() {
        let token = CancellationToken::new();
        token.cancel();

        let result = cancellable(&token, std::future::pending::<i32>()).await;
        assert!(result.is_err());
        assert_eq!(format!("{}", result.unwrap_err()), "Operation cancelled");
    }

    #[tokio::test]
    async fn test_cancellable_cancel_during_execution() {
        let token = CancellationToken::new();
        let token_clone = token.clone();

        // Spawn a task that cancels after a short delay
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            token_clone.cancel();
        });

        let result = cancellable(&token, std::future::pending::<i32>()).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_cancelled_error_display() {
        let err = Cancelled;
        assert_eq!(err.to_string(), "Operation cancelled");
    }
}
