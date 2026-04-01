use thiserror::Error;

/// Top-level error type for the Claude Code agent.
#[derive(Error, Debug)]
pub enum CcError {
    #[error("API error: {message}")]
    Api {
        message: String,
        status_code: Option<u16>,
        retryable: bool,
    },

    #[error("Tool error: {message}")]
    Tool { tool_name: String, message: String },

    #[error("Permission denied: {message}")]
    PermissionDenied { tool_name: String, message: String },

    #[error("Prompt too long: {message}")]
    PromptTooLong { message: String },

    #[error("Rate limited: {message}")]
    RateLimited {
        message: String,
        retry_after_ms: Option<u64>,
    },

    #[error("Overloaded: {message}")]
    Overloaded { message: String },

    #[error("Configuration error: {message}")]
    Config { message: String },

    #[error("Session error: {message}")]
    Session { message: String },

    #[error("MCP error: {message}")]
    Mcp { message: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Cancelled")]
    Cancelled,
}

/// Convenience type alias for results using `CcError`.
pub type CcResult<T> = Result<T, CcError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_error_display() {
        let err = CcError::Api {
            message: "Server error".to_string(),
            status_code: Some(500),
            retryable: true,
        };
        assert_eq!(err.to_string(), "API error: Server error");
    }

    #[test]
    fn tool_error_display() {
        let err = CcError::Tool {
            tool_name: "bash".to_string(),
            message: "Command failed".to_string(),
        };
        assert_eq!(err.to_string(), "Tool error: Command failed");
    }

    #[test]
    fn permission_denied_display() {
        let err = CcError::PermissionDenied {
            tool_name: "write_file".to_string(),
            message: "Not allowed".to_string(),
        };
        assert_eq!(err.to_string(), "Permission denied: Not allowed");
    }

    #[test]
    fn rate_limited_with_retry() {
        let err = CcError::RateLimited {
            message: "Too many requests".to_string(),
            retry_after_ms: Some(5000),
        };
        assert_eq!(err.to_string(), "Rate limited: Too many requests");
        match err {
            CcError::RateLimited { retry_after_ms, .. } => {
                assert_eq!(retry_after_ms, Some(5000));
            }
            _ => panic!("expected RateLimited"),
        }
    }

    #[test]
    fn from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let cc_err: CcError = io_err.into();
        assert!(matches!(cc_err, CcError::Io(_)));
        assert!(cc_err.to_string().contains("file not found"));
    }

    #[test]
    fn from_json_error() {
        let result: Result<serde_json::Value, _> = serde_json::from_str("not json");
        let json_err = result.unwrap_err();
        let cc_err: CcError = json_err.into();
        assert!(matches!(cc_err, CcError::Json(_)));
    }

    #[test]
    fn cancelled_display() {
        let err = CcError::Cancelled;
        assert_eq!(err.to_string(), "Cancelled");
    }

    #[test]
    fn cc_result_ok() {
        let result: CcResult<i32> = Ok(42);
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn cc_result_err() {
        let result: CcResult<i32> = Err(CcError::Config {
            message: "bad config".to_string(),
        });
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Configuration error: bad config"
        );
    }

    #[test]
    fn all_variants_display() {
        let errors: Vec<CcError> = vec![
            CcError::Api {
                message: "m".into(),
                status_code: None,
                retryable: false,
            },
            CcError::Tool {
                tool_name: "t".into(),
                message: "m".into(),
            },
            CcError::PermissionDenied {
                tool_name: "t".into(),
                message: "m".into(),
            },
            CcError::PromptTooLong {
                message: "m".into(),
            },
            CcError::RateLimited {
                message: "m".into(),
                retry_after_ms: None,
            },
            CcError::Overloaded {
                message: "m".into(),
            },
            CcError::Config {
                message: "m".into(),
            },
            CcError::Session {
                message: "m".into(),
            },
            CcError::Mcp {
                message: "m".into(),
            },
            CcError::Cancelled,
        ];
        for err in errors {
            assert!(!err.to_string().is_empty());
        }
    }
}
