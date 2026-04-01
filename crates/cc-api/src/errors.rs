use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Authentication failed: {message}")]
    AuthError { message: String },
    #[error("Rate limited (retry after {retry_after_ms:?}ms)")]
    RateLimited {
        message: String,
        retry_after_ms: Option<u64>,
    },
    #[error("Overloaded: {message}")]
    Overloaded { message: String },
    #[error("Prompt too long: {message}")]
    PromptTooLong { message: String },
    #[error("Invalid request: {message}")]
    InvalidRequest { message: String },
    #[error("Server error ({status}): {message}")]
    ServerError { status: u16, message: String },
    #[error("Connection error: {message}")]
    ConnectionError { message: String },
    #[error("Timeout")]
    Timeout,
    #[error("Stream error: {message}")]
    StreamError { message: String },
    #[error("Cancelled")]
    Cancelled,
}

impl ApiError {
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::RateLimited { .. } => true,
            Self::Overloaded { .. } => true,
            Self::ServerError { status, .. } => *status >= 500,
            Self::ConnectionError { .. } => true,
            Self::Timeout => true,
            _ => false,
        }
    }

    pub fn from_status(status: u16, body: &str) -> Self {
        // Try to parse as JSON error body
        #[derive(serde::Deserialize)]
        struct ErrorEnvelope {
            error: ErrorInner,
        }
        #[derive(serde::Deserialize)]
        struct ErrorInner {
            #[serde(rename = "type")]
            error_type: String,
            message: String,
        }

        let (error_type, message) = match serde_json::from_str::<ErrorEnvelope>(body) {
            Ok(envelope) => (envelope.error.error_type, envelope.error.message),
            Err(_) => ("unknown".to_string(), body.to_string()),
        };

        match status {
            401 => ApiError::AuthError { message },
            429 => ApiError::RateLimited {
                message,
                retry_after_ms: None,
            },
            413 => ApiError::PromptTooLong { message },
            400 => ApiError::InvalidRequest { message },
            529 => ApiError::Overloaded { message },
            s if s >= 500 => ApiError::ServerError {
                status: s,
                message,
            },
            _ => {
                if error_type == "overloaded_error" {
                    ApiError::Overloaded { message }
                } else {
                    ApiError::InvalidRequest { message }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_retryable_rate_limited() {
        let err = ApiError::RateLimited {
            message: "rate limited".into(),
            retry_after_ms: Some(1000),
        };
        assert!(err.is_retryable());
    }

    #[test]
    fn is_retryable_overloaded() {
        let err = ApiError::Overloaded {
            message: "overloaded".into(),
        };
        assert!(err.is_retryable());
    }

    #[test]
    fn is_retryable_server_error_500() {
        let err = ApiError::ServerError {
            status: 500,
            message: "internal".into(),
        };
        assert!(err.is_retryable());
    }

    #[test]
    fn is_retryable_server_error_502() {
        let err = ApiError::ServerError {
            status: 502,
            message: "bad gateway".into(),
        };
        assert!(err.is_retryable());
    }

    #[test]
    fn is_retryable_connection_error() {
        let err = ApiError::ConnectionError {
            message: "connection refused".into(),
        };
        assert!(err.is_retryable());
    }

    #[test]
    fn is_retryable_timeout() {
        let err = ApiError::Timeout;
        assert!(err.is_retryable());
    }

    #[test]
    fn not_retryable_auth_error() {
        let err = ApiError::AuthError {
            message: "bad key".into(),
        };
        assert!(!err.is_retryable());
    }

    #[test]
    fn not_retryable_invalid_request() {
        let err = ApiError::InvalidRequest {
            message: "bad request".into(),
        };
        assert!(!err.is_retryable());
    }

    #[test]
    fn not_retryable_prompt_too_long() {
        let err = ApiError::PromptTooLong {
            message: "too long".into(),
        };
        assert!(!err.is_retryable());
    }

    #[test]
    fn not_retryable_cancelled() {
        let err = ApiError::Cancelled;
        assert!(!err.is_retryable());
    }

    #[test]
    fn not_retryable_stream_error() {
        let err = ApiError::StreamError {
            message: "stream broke".into(),
        };
        assert!(!err.is_retryable());
    }

    #[test]
    fn from_status_401() {
        let err = ApiError::from_status(
            401,
            r#"{"error":{"type":"authentication_error","message":"Invalid API key"}}"#,
        );
        assert!(matches!(err, ApiError::AuthError { .. }));
        assert!(err.to_string().contains("Invalid API key"));
    }

    #[test]
    fn from_status_429() {
        let err = ApiError::from_status(
            429,
            r#"{"error":{"type":"rate_limit_error","message":"Too many requests"}}"#,
        );
        assert!(matches!(
            err,
            ApiError::RateLimited {
                retry_after_ms: None,
                ..
            }
        ));
    }

    #[test]
    fn from_status_413() {
        let err = ApiError::from_status(
            413,
            r#"{"error":{"type":"request_too_large","message":"Prompt too long"}}"#,
        );
        assert!(matches!(err, ApiError::PromptTooLong { .. }));
    }

    #[test]
    fn from_status_400() {
        let err = ApiError::from_status(
            400,
            r#"{"error":{"type":"invalid_request_error","message":"Bad param"}}"#,
        );
        assert!(matches!(err, ApiError::InvalidRequest { .. }));
    }

    #[test]
    fn from_status_529() {
        let err = ApiError::from_status(
            529,
            r#"{"error":{"type":"overloaded_error","message":"API overloaded"}}"#,
        );
        assert!(matches!(err, ApiError::Overloaded { .. }));
    }

    #[test]
    fn from_status_500() {
        let err = ApiError::from_status(
            500,
            r#"{"error":{"type":"api_error","message":"Internal error"}}"#,
        );
        assert!(matches!(err, ApiError::ServerError { status: 500, .. }));
    }

    #[test]
    fn from_status_non_json_body() {
        let err = ApiError::from_status(500, "Internal Server Error");
        assert!(matches!(err, ApiError::ServerError { status: 500, .. }));
        assert!(err.to_string().contains("Internal Server Error"));
    }

    #[test]
    fn from_status_unknown_with_overloaded_type() {
        let err = ApiError::from_status(
            422,
            r#"{"error":{"type":"overloaded_error","message":"overloaded"}}"#,
        );
        assert!(matches!(err, ApiError::Overloaded { .. }));
    }

    #[test]
    fn from_status_unknown_without_overloaded_type() {
        let err = ApiError::from_status(
            422,
            r#"{"error":{"type":"something_else","message":"unknown"}}"#,
        );
        assert!(matches!(err, ApiError::InvalidRequest { .. }));
    }

    #[test]
    fn error_display_messages() {
        let err = ApiError::AuthError {
            message: "bad key".into(),
        };
        assert_eq!(err.to_string(), "Authentication failed: bad key");

        let err = ApiError::RateLimited {
            message: "slow down".into(),
            retry_after_ms: Some(5000),
        };
        assert_eq!(err.to_string(), "Rate limited (retry after Some(5000)ms)");

        let err = ApiError::Timeout;
        assert_eq!(err.to_string(), "Timeout");

        let err = ApiError::Cancelled;
        assert_eq!(err.to_string(), "Cancelled");
    }
}
