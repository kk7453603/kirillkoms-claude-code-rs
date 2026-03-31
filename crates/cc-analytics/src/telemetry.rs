/// Initialize telemetry (tracing subscriber).
///
/// This is a helper that will be called from cc-cli. Currently a placeholder
/// that can be extended to set up tracing subscribers, OpenTelemetry, etc.
pub fn init_telemetry(_verbose: bool) {
    // Placeholder: in a full implementation this would configure
    // a tracing subscriber with appropriate filtering.
}

/// Check if telemetry is enabled.
///
/// Telemetry is considered enabled when:
/// - `DISABLE_TELEMETRY` is not set to "1" or "true"
/// - `CLAUDE_CODE_ENABLE_TELEMETRY` is set to "1" or "true"
///
/// Both conditions must be met.
pub fn is_telemetry_enabled() -> bool {
    let not_disabled = std::env::var("DISABLE_TELEMETRY")
        .map(|v| v != "1" && v != "true")
        .unwrap_or(true);

    let explicitly_enabled = std::env::var("CLAUDE_CODE_ENABLE_TELEMETRY")
        .map(|v| v == "1" || v == "true")
        .unwrap_or(false);

    not_disabled && explicitly_enabled
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_telemetry_does_not_panic() {
        init_telemetry(false);
        init_telemetry(true);
    }

    #[test]
    fn test_telemetry_disabled_by_default() {
        // Without CLAUDE_CODE_ENABLE_TELEMETRY set, telemetry should be off.
        // SAFETY: test-only env var manipulation; tests may run serially
        // for correctness when modifying env vars.
        unsafe {
            std::env::remove_var("CLAUDE_CODE_ENABLE_TELEMETRY");
            std::env::remove_var("DISABLE_TELEMETRY");
        }
        assert!(!is_telemetry_enabled());
    }

    #[test]
    fn test_telemetry_enabled_when_opted_in() {
        // SAFETY: test-only env var manipulation
        unsafe {
            std::env::set_var("CLAUDE_CODE_ENABLE_TELEMETRY", "1");
            std::env::remove_var("DISABLE_TELEMETRY");
        }
        let result = is_telemetry_enabled();
        unsafe {
            std::env::remove_var("CLAUDE_CODE_ENABLE_TELEMETRY");
        }
        assert!(result);
    }

    #[test]
    fn test_telemetry_disabled_by_disable_var() {
        // SAFETY: test-only env var manipulation
        unsafe {
            std::env::set_var("CLAUDE_CODE_ENABLE_TELEMETRY", "1");
            std::env::set_var("DISABLE_TELEMETRY", "1");
        }
        let result = is_telemetry_enabled();
        unsafe {
            std::env::remove_var("CLAUDE_CODE_ENABLE_TELEMETRY");
            std::env::remove_var("DISABLE_TELEMETRY");
        }
        assert!(!result);
    }
}
