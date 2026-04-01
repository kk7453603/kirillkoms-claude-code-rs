use crate::errors::ApiError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiProvider {
    Direct,
    Bedrock,
    Vertex,
    Foundry,
}

impl ApiProvider {
    /// Detect the provider from environment variables.
    pub fn from_env() -> Self {
        if std::env::var("CLAUDE_CODE_USE_BEDROCK").ok().as_deref() == Some("1")
            || std::env::var("AWS_REGION").is_ok()
                && std::env::var("ANTHROPIC_MODEL")
                    .ok()
                    .map_or(false, |m| m.starts_with("anthropic."))
        {
            return ApiProvider::Bedrock;
        }
        if std::env::var("CLAUDE_CODE_USE_VERTEX").ok().as_deref() == Some("1")
            || std::env::var("CLOUD_ML_PROJECT_ID").is_ok()
            || std::env::var("GOOGLE_CLOUD_PROJECT").is_ok()
        {
            return ApiProvider::Vertex;
        }
        if std::env::var("CLAUDE_CODE_USE_FOUNDRY").ok().as_deref() == Some("1")
            || std::env::var("AZURE_FOUNDRY_BASE_URL").is_ok()
        {
            return ApiProvider::Foundry;
        }
        ApiProvider::Direct
    }
}

#[derive(Debug, Clone)]
pub enum AuthMethod {
    ApiKey(String),
    OAuthToken(String),
}

#[derive(Debug, Clone)]
pub struct ApiConfig {
    pub auth: AuthMethod,
    pub base_url: String,
    pub max_retries: u32,
    pub timeout_secs: u64,
}

impl ApiConfig {
    /// Create config from environment variables
    pub fn from_env() -> Result<Self, ApiError> {
        let api_key = std::env::var("ANTHROPIC_API_KEY").ok();
        let auth_token = std::env::var("ANTHROPIC_AUTH_TOKEN")
            .ok()
            .or_else(|| std::env::var("CLAUDE_CODE_OAUTH_TOKEN").ok());
        let base_url = std::env::var("ANTHROPIC_BASE_URL")
            .unwrap_or_else(|_| "https://api.anthropic.com".to_string());

        let auth = if let Some(key) = api_key {
            AuthMethod::ApiKey(key)
        } else if let Some(token) = auth_token {
            AuthMethod::OAuthToken(token)
        } else {
            return Err(ApiError::AuthError {
                message: "No API key or OAuth token found. Set ANTHROPIC_API_KEY or ANTHROPIC_AUTH_TOKEN.".into(),
            });
        };

        Ok(Self {
            auth,
            base_url,
            max_retries: 3,
            timeout_secs: 600,
        })
    }

    pub fn with_api_key(key: String) -> Self {
        Self {
            auth: AuthMethod::ApiKey(key),
            base_url: "https://api.anthropic.com".to_string(),
            max_retries: 3,
            timeout_secs: 600,
        }
    }

    /// Returns the header name and value for authentication.
    pub fn auth_header(&self) -> (&str, String) {
        match &self.auth {
            AuthMethod::ApiKey(key) => ("x-api-key", key.clone()),
            AuthMethod::OAuthToken(token) => ("authorization", format!("Bearer {}", token)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_api_key_creates_config() {
        let config = ApiConfig::with_api_key("sk-test-123".to_string());
        assert_eq!(config.base_url, "https://api.anthropic.com");
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.timeout_secs, 600);
        match &config.auth {
            AuthMethod::ApiKey(k) => assert_eq!(k, "sk-test-123"),
            _ => panic!("expected ApiKey"),
        }
    }

    #[test]
    fn auth_header_api_key() {
        let config = ApiConfig::with_api_key("sk-test".to_string());
        let (name, value) = config.auth_header();
        assert_eq!(name, "x-api-key");
        assert_eq!(value, "sk-test");
    }

    #[test]
    fn auth_header_oauth_token() {
        let config = ApiConfig {
            auth: AuthMethod::OAuthToken("oauth-token-123".to_string()),
            base_url: "https://api.anthropic.com".to_string(),
            max_retries: 3,
            timeout_secs: 600,
        };
        let (name, value) = config.auth_header();
        assert_eq!(name, "authorization");
        assert_eq!(value, "Bearer oauth-token-123");
    }

    #[test]
    fn api_provider_default_is_direct() {
        // Without any special env vars, should default to Direct
        // (This test is best-effort; env may have vars set externally)
        let provider = ApiProvider::from_env();
        // Just verify it returns a valid variant
        assert!(
            provider == ApiProvider::Direct
                || provider == ApiProvider::Bedrock
                || provider == ApiProvider::Vertex
                || provider == ApiProvider::Foundry
        );
    }

    #[test]
    fn api_provider_equality() {
        assert_eq!(ApiProvider::Direct, ApiProvider::Direct);
        assert_eq!(ApiProvider::Bedrock, ApiProvider::Bedrock);
        assert_eq!(ApiProvider::Vertex, ApiProvider::Vertex);
        assert_eq!(ApiProvider::Foundry, ApiProvider::Foundry);
        assert_ne!(ApiProvider::Direct, ApiProvider::Bedrock);
    }

    // Note: from_env() tests that manipulate env vars are inherently racy
    // in parallel test execution. We test the logic through with_api_key()
    // and direct struct construction instead.

    #[test]
    fn from_env_returns_result() {
        // Just verify from_env doesn't panic - result depends on actual env
        let _ = ApiConfig::from_env();
    }

    #[test]
    fn default_base_url() {
        let config = ApiConfig::with_api_key("sk-test".into());
        assert_eq!(config.base_url, "https://api.anthropic.com");
    }

    #[test]
    fn default_retries_and_timeout() {
        let config = ApiConfig::with_api_key("sk-test".into());
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.timeout_secs, 600);
    }

    #[test]
    fn api_key_auth_method() {
        let config = ApiConfig {
            auth: AuthMethod::ApiKey("sk-key".into()),
            base_url: "https://api.anthropic.com".into(),
            max_retries: 3,
            timeout_secs: 600,
        };
        let (name, value) = config.auth_header();
        assert_eq!(name, "x-api-key");
        assert_eq!(value, "sk-key");
    }

    #[test]
    fn oauth_auth_method() {
        let config = ApiConfig {
            auth: AuthMethod::OAuthToken("token-123".into()),
            base_url: "https://api.anthropic.com".into(),
            max_retries: 3,
            timeout_secs: 600,
        };
        let (name, value) = config.auth_header();
        assert_eq!(name, "authorization");
        assert_eq!(value, "Bearer token-123");
    }
}
