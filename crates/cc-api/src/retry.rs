use std::time::Duration;

use crate::errors::ApiError;

pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub backoff_factor: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(30),
            backoff_factor: 2.0,
        }
    }
}

/// Calculate delay for a given retry attempt (0-based).
pub fn retry_delay(config: &RetryConfig, attempt: u32) -> Duration {
    let delay_ms =
        config.initial_delay.as_millis() as f64 * config.backoff_factor.powi(attempt as i32);
    let delay = Duration::from_millis(delay_ms as u64);
    if delay > config.max_delay {
        config.max_delay
    } else {
        delay
    }
}

/// Execute an async operation with retry logic.
///
/// Retries the operation up to `config.max_retries` times if the error
/// is retryable (as determined by `ApiError::is_retryable`).
pub async fn with_retry<F, Fut, T>(config: &RetryConfig, operation: F) -> Result<T, ApiError>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, ApiError>>,
{
    let mut last_error: Option<ApiError> = None;

    for attempt in 0..=config.max_retries {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(err) => {
                if !err.is_retryable() || attempt == config.max_retries {
                    return Err(err);
                }
                let delay = retry_delay(config, attempt);
                tracing::warn!(
                    attempt = attempt,
                    delay_ms = delay.as_millis() as u64,
                    error = %err,
                    "Retrying after error"
                );
                tokio::time::sleep(delay).await;
                last_error = Some(err);
            }
        }
    }

    // This should be unreachable, but just in case:
    Err(last_error.unwrap_or(ApiError::ConnectionError {
        message: "Retry loop exhausted without result".to_string(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_delay, Duration::from_millis(500));
        assert_eq!(config.max_delay, Duration::from_secs(30));
        assert!((config.backoff_factor - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn retry_delay_first_attempt() {
        let config = RetryConfig::default();
        let delay = retry_delay(&config, 0);
        assert_eq!(delay, Duration::from_millis(500));
    }

    #[test]
    fn retry_delay_second_attempt() {
        let config = RetryConfig::default();
        let delay = retry_delay(&config, 1);
        assert_eq!(delay, Duration::from_millis(1000));
    }

    #[test]
    fn retry_delay_third_attempt() {
        let config = RetryConfig::default();
        let delay = retry_delay(&config, 2);
        assert_eq!(delay, Duration::from_millis(2000));
    }

    #[test]
    fn retry_delay_capped_at_max() {
        let config = RetryConfig {
            max_delay: Duration::from_secs(5),
            ..RetryConfig::default()
        };
        // attempt 10: 500 * 2^10 = 512000ms = 512s, should be capped at 5s
        let delay = retry_delay(&config, 10);
        assert_eq!(delay, Duration::from_secs(5));
    }

    #[test]
    fn retry_delay_custom_config() {
        let config = RetryConfig {
            initial_delay: Duration::from_millis(100),
            backoff_factor: 3.0,
            max_delay: Duration::from_secs(60),
            max_retries: 5,
        };
        // attempt 0: 100ms
        assert_eq!(retry_delay(&config, 0), Duration::from_millis(100));
        // attempt 1: 300ms
        assert_eq!(retry_delay(&config, 1), Duration::from_millis(300));
        // attempt 2: 900ms
        assert_eq!(retry_delay(&config, 2), Duration::from_millis(900));
    }

    #[tokio::test]
    async fn with_retry_succeeds_first_try() {
        let config = RetryConfig {
            max_retries: 3,
            initial_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(10),
            backoff_factor: 2.0,
        };

        let result = with_retry(&config, || async { Ok::<_, ApiError>(42) }).await;
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn with_retry_succeeds_after_retries() {
        let config = RetryConfig {
            max_retries: 3,
            initial_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(10),
            backoff_factor: 2.0,
        };

        let attempt = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let attempt_clone = attempt.clone();

        let result = with_retry(&config, move || {
            let attempt = attempt_clone.clone();
            async move {
                let n = attempt.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                if n < 2 {
                    Err(ApiError::ConnectionError {
                        message: "connection refused".into(),
                    })
                } else {
                    Ok(42)
                }
            }
        })
        .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempt.load(std::sync::atomic::Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn with_retry_fails_non_retryable() {
        let config = RetryConfig {
            max_retries: 3,
            initial_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(10),
            backoff_factor: 2.0,
        };

        let attempt = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let attempt_clone = attempt.clone();

        let result = with_retry(&config, move || {
            let attempt = attempt_clone.clone();
            async move {
                attempt.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Err::<i32, _>(ApiError::AuthError {
                    message: "bad key".into(),
                })
            }
        })
        .await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ApiError::AuthError { .. }));
        // Should not retry non-retryable errors
        assert_eq!(attempt.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn with_retry_exhausts_retries() {
        let config = RetryConfig {
            max_retries: 2,
            initial_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(10),
            backoff_factor: 2.0,
        };

        let attempt = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let attempt_clone = attempt.clone();

        let result = with_retry(&config, move || {
            let attempt = attempt_clone.clone();
            async move {
                attempt.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Err::<i32, _>(ApiError::Timeout)
            }
        })
        .await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ApiError::Timeout));
        // initial attempt + 2 retries = 3 total
        assert_eq!(attempt.load(std::sync::atomic::Ordering::SeqCst), 3);
    }
}
