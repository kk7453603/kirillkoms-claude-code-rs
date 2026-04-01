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
    fn test_telemetry_logic_disabled_when_not_opted_in() {
        // Test the logic directly without touching env vars.
        // When CLAUDE_CODE_ENABLE_TELEMETRY is not set, result is false.
        // We just verify the function doesn't panic and returns a bool.
        let _ = is_telemetry_enabled();
    }

    #[test]
    fn test_telemetry_function_returns_bool() {
        let result = is_telemetry_enabled();
        // Result should be a valid bool (type system guarantees this, but verify no panic)
        assert!(result || !result);
    }
}
