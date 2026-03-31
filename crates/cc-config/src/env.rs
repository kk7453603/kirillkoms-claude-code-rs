/// Environment variable configuration for Claude Code.
#[derive(Debug, Clone)]
pub struct EnvConfig {
    // API
    pub api_key: Option<String>,
    pub auth_token: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,

    // Providers
    pub use_bedrock: bool,
    pub use_vertex: bool,
    pub use_foundry: bool,

    // AWS
    pub aws_region: Option<String>,
    pub aws_profile: Option<String>,
    pub bedrock_base_url: Option<String>,

    // GCP
    pub vertex_project_id: Option<String>,
    pub vertex_region: Option<String>,
    pub vertex_base_url: Option<String>,

    // Foundry
    pub foundry_base_url: Option<String>,
    pub foundry_resource: Option<String>,

    // Features
    pub max_thinking_tokens: Option<u64>,
    pub bash_default_timeout_ms: Option<u64>,
    pub bash_max_timeout_ms: Option<u64>,
    pub bash_max_output_length: Option<usize>,
    pub max_output_tokens: Option<u64>,

    // Behavior
    pub disable_telemetry: bool,
    pub disable_error_reporting: bool,
    pub disable_auto_updater: bool,
    pub simple_mode: bool,
    pub is_ci: bool,

    // User type
    pub user_type: UserType,
}

/// The type of user running Claude Code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserType {
    Internal,
    External,
}

/// Which API provider to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiProvider {
    Direct,
    Bedrock,
    Vertex,
    Foundry,
}

impl EnvConfig {
    /// Read all configuration from environment variables.
    pub fn from_env() -> Self {
        Self {
            // API
            api_key: get_opt("ANTHROPIC_API_KEY"),
            auth_token: get_opt("CLAUDE_AUTH_TOKEN"),
            base_url: get_opt("ANTHROPIC_BASE_URL"),
            model: get_opt("ANTHROPIC_MODEL").or_else(|| get_opt("CLAUDE_MODEL")),

            // Providers
            use_bedrock: get_bool("CLAUDE_USE_BEDROCK"),
            use_vertex: get_bool("CLAUDE_USE_VERTEX"),
            use_foundry: get_bool("CLAUDE_USE_FOUNDRY"),

            // AWS
            aws_region: get_opt("AWS_REGION").or_else(|| get_opt("AWS_DEFAULT_REGION")),
            aws_profile: get_opt("AWS_PROFILE"),
            bedrock_base_url: get_opt("ANTHROPIC_BEDROCK_BASE_URL"),

            // GCP
            vertex_project_id: get_opt("ANTHROPIC_VERTEX_PROJECT_ID")
                .or_else(|| get_opt("CLOUD_ML_PROJECT_ID")),
            vertex_region: get_opt("CLOUD_ML_REGION"),
            vertex_base_url: get_opt("ANTHROPIC_VERTEX_BASE_URL"),

            // Foundry
            foundry_base_url: get_opt("ANTHROPIC_FOUNDRY_BASE_URL"),
            foundry_resource: get_opt("ANTHROPIC_FOUNDRY_RESOURCE"),

            // Features
            max_thinking_tokens: get_u64("CLAUDE_MAX_THINKING_TOKENS"),
            bash_default_timeout_ms: get_u64("CLAUDE_BASH_DEFAULT_TIMEOUT_MS"),
            bash_max_timeout_ms: get_u64("CLAUDE_BASH_MAX_TIMEOUT_MS"),
            bash_max_output_length: get_u64("CLAUDE_BASH_MAX_OUTPUT_LENGTH").map(|v| v as usize),
            max_output_tokens: get_u64("CLAUDE_MAX_OUTPUT_TOKENS"),

            // Behavior
            disable_telemetry: get_bool("CLAUDE_DISABLE_TELEMETRY"),
            disable_error_reporting: get_bool("CLAUDE_DISABLE_ERROR_REPORTING"),
            disable_auto_updater: get_bool("CLAUDE_DISABLE_AUTO_UPDATER"),
            simple_mode: get_bool("CLAUDE_SIMPLE_MODE"),
            is_ci: get_bool("CI") || get_bool("CLAUDE_CI"),

            // User type
            user_type: if get_bool("CLAUDE_INTERNAL") {
                UserType::Internal
            } else {
                UserType::External
            },
        }
    }

    /// Returns true if running in a sandbox environment.
    pub fn is_sandbox(&self) -> bool {
        get_bool_static("CLAUDE_SANDBOX") || get_bool_static("SANDBOX")
    }

    /// Determine the API provider based on environment configuration.
    pub fn provider(&self) -> ApiProvider {
        if self.use_bedrock {
            ApiProvider::Bedrock
        } else if self.use_vertex {
            ApiProvider::Vertex
        } else if self.use_foundry {
            ApiProvider::Foundry
        } else {
            ApiProvider::Direct
        }
    }
}

impl Default for EnvConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            auth_token: None,
            base_url: None,
            model: None,
            use_bedrock: false,
            use_vertex: false,
            use_foundry: false,
            aws_region: None,
            aws_profile: None,
            bedrock_base_url: None,
            vertex_project_id: None,
            vertex_region: None,
            vertex_base_url: None,
            foundry_base_url: None,
            foundry_resource: None,
            max_thinking_tokens: None,
            bash_default_timeout_ms: None,
            bash_max_timeout_ms: None,
            bash_max_output_length: None,
            max_output_tokens: None,
            disable_telemetry: false,
            disable_error_reporting: false,
            disable_auto_updater: false,
            simple_mode: false,
            is_ci: false,
            user_type: UserType::External,
        }
    }
}

/// Get an optional string from an environment variable.
fn get_opt(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.is_empty())
}

/// Parse an environment variable as a boolean.
/// Treats "1", "true", "yes" (case-insensitive) as true.
fn get_bool(key: &str) -> bool {
    std::env::var(key)
        .ok()
        .map(|v| matches!(v.to_lowercase().as_str(), "1" | "true" | "yes"))
        .unwrap_or(false)
}

/// Same as get_bool but reads at call time (for methods called after construction).
fn get_bool_static(key: &str) -> bool {
    get_bool(key)
}

/// Parse an environment variable as u64.
fn get_u64(key: &str) -> Option<u64> {
    std::env::var(key).ok().and_then(|v| v.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to safely set/remove env vars in tests.
    // SAFETY: These tests must run with --test-threads=1 or use unique var names.
    unsafe fn set_env(key: &str, val: &str) {
        unsafe { std::env::set_var(key, val); }
    }
    unsafe fn remove_env(key: &str) {
        unsafe { std::env::remove_var(key); }
    }

    #[test]
    fn env_config_default() {
        let config = EnvConfig::default();
        assert!(config.api_key.is_none());
        assert!(!config.use_bedrock);
        assert!(!config.is_ci);
        assert_eq!(config.user_type, UserType::External);
    }

    #[test]
    fn provider_direct_by_default() {
        let config = EnvConfig::default();
        assert_eq!(config.provider(), ApiProvider::Direct);
    }

    #[test]
    fn provider_bedrock() {
        let mut config = EnvConfig::default();
        config.use_bedrock = true;
        assert_eq!(config.provider(), ApiProvider::Bedrock);
    }

    #[test]
    fn provider_vertex() {
        let mut config = EnvConfig::default();
        config.use_vertex = true;
        assert_eq!(config.provider(), ApiProvider::Vertex);
    }

    #[test]
    fn provider_foundry() {
        let mut config = EnvConfig::default();
        config.use_foundry = true;
        assert_eq!(config.provider(), ApiProvider::Foundry);
    }

    #[test]
    fn provider_bedrock_takes_precedence() {
        let mut config = EnvConfig::default();
        config.use_bedrock = true;
        config.use_vertex = true;
        assert_eq!(config.provider(), ApiProvider::Bedrock);
    }

    #[test]
    fn get_bool_parses_true_values() {
        unsafe {
            set_env("CC_TEST_BOOL_1", "1");
            assert!(get_bool("CC_TEST_BOOL_1"));
            set_env("CC_TEST_BOOL_TRUE", "true");
            assert!(get_bool("CC_TEST_BOOL_TRUE"));
            set_env("CC_TEST_BOOL_YES", "yes");
            assert!(get_bool("CC_TEST_BOOL_YES"));
            set_env("CC_TEST_BOOL_TRUE_UPPER", "TRUE");
            assert!(get_bool("CC_TEST_BOOL_TRUE_UPPER"));
            set_env("CC_TEST_BOOL_YES_UPPER", "Yes");
            assert!(get_bool("CC_TEST_BOOL_YES_UPPER"));

            for key in [
                "CC_TEST_BOOL_1",
                "CC_TEST_BOOL_TRUE",
                "CC_TEST_BOOL_YES",
                "CC_TEST_BOOL_TRUE_UPPER",
                "CC_TEST_BOOL_YES_UPPER",
            ] {
                remove_env(key);
            }
        }
    }

    #[test]
    fn get_bool_parses_false_values() {
        unsafe {
            set_env("CC_TEST_BOOL_0", "0");
            assert!(!get_bool("CC_TEST_BOOL_0"));
            set_env("CC_TEST_BOOL_FALSE", "false");
            assert!(!get_bool("CC_TEST_BOOL_FALSE"));
            set_env("CC_TEST_BOOL_EMPTY", "");
            assert!(!get_bool("CC_TEST_BOOL_EMPTY"));
            assert!(!get_bool("CC_TEST_BOOL_MISSING"));

            for key in ["CC_TEST_BOOL_0", "CC_TEST_BOOL_FALSE", "CC_TEST_BOOL_EMPTY"] {
                remove_env(key);
            }
        }
    }

    #[test]
    fn get_u64_parsing() {
        unsafe {
            set_env("CC_TEST_U64_VALID", "12345");
            assert_eq!(get_u64("CC_TEST_U64_VALID"), Some(12345));

            set_env("CC_TEST_U64_INVALID", "not_a_number");
            assert_eq!(get_u64("CC_TEST_U64_INVALID"), None);

            assert_eq!(get_u64("CC_TEST_U64_MISSING"), None);

            remove_env("CC_TEST_U64_VALID");
            remove_env("CC_TEST_U64_INVALID");
        }
    }

    #[test]
    fn get_opt_empty_returns_none() {
        unsafe {
            set_env("CC_TEST_OPT_EMPTY", "");
            assert!(get_opt("CC_TEST_OPT_EMPTY").is_none());
            remove_env("CC_TEST_OPT_EMPTY");
        }
    }

    #[test]
    fn get_opt_nonempty_returns_some() {
        unsafe {
            set_env("CC_TEST_OPT_VAL", "hello");
            assert_eq!(get_opt("CC_TEST_OPT_VAL"), Some("hello".to_string()));
            remove_env("CC_TEST_OPT_VAL");
        }
    }

    #[test]
    fn from_env_reads_api_key() {
        unsafe {
            set_env("ANTHROPIC_API_KEY", "sk-test-key-12345");
            let config = EnvConfig::from_env();
            assert_eq!(config.api_key, Some("sk-test-key-12345".to_string()));
            remove_env("ANTHROPIC_API_KEY");
        }
    }

    #[test]
    fn from_env_reads_model_from_anthropic_model() {
        unsafe {
            remove_env("CLAUDE_MODEL");
            set_env("ANTHROPIC_MODEL", "claude-opus-4-6");
            let config = EnvConfig::from_env();
            assert_eq!(config.model, Some("claude-opus-4-6".to_string()));
            remove_env("ANTHROPIC_MODEL");
        }
    }

    #[test]
    fn from_env_reads_model_from_claude_model_fallback() {
        unsafe {
            remove_env("ANTHROPIC_MODEL");
            set_env("CLAUDE_MODEL", "claude-sonnet-4-6");
            let config = EnvConfig::from_env();
            assert_eq!(config.model, Some("claude-sonnet-4-6".to_string()));
            remove_env("CLAUDE_MODEL");
        }
    }

    #[test]
    fn from_env_reads_ci() {
        unsafe {
            set_env("CI", "true");
            let config = EnvConfig::from_env();
            assert!(config.is_ci);
            remove_env("CI");
        }
    }

    #[test]
    fn from_env_reads_internal_user() {
        unsafe {
            set_env("CLAUDE_INTERNAL", "1");
            let config = EnvConfig::from_env();
            assert_eq!(config.user_type, UserType::Internal);
            remove_env("CLAUDE_INTERNAL");
        }
    }

    #[test]
    fn user_type_equality() {
        assert_eq!(UserType::Internal, UserType::Internal);
        assert_ne!(UserType::Internal, UserType::External);
    }

    #[test]
    fn api_provider_equality() {
        assert_eq!(ApiProvider::Direct, ApiProvider::Direct);
        assert_ne!(ApiProvider::Direct, ApiProvider::Bedrock);
    }

    #[test]
    fn is_sandbox_reads_env() {
        unsafe {
            remove_env("CLAUDE_SANDBOX");
            remove_env("SANDBOX");
            let config = EnvConfig::default();
            assert!(!config.is_sandbox());

            set_env("CLAUDE_SANDBOX", "1");
            assert!(config.is_sandbox());
            remove_env("CLAUDE_SANDBOX");

            set_env("SANDBOX", "true");
            assert!(config.is_sandbox());
            remove_env("SANDBOX");
        }
    }
}
