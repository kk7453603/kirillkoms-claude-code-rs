/// Format a cost in USD as a human-readable string.
///
/// Examples: `"$0.00"`, `"$0.01"`, `"$1.23"`, `"$0.0001"`.
pub fn format_cost(usd: f64) -> String {
    if usd == 0.0 {
        return "$0.00".to_string();
    }
    if usd < 0.005 {
        // Show more precision for very small amounts
        // Find the first significant digit and show 2 significant figures
        let formatted = format!("{:.4}", usd);
        // Trim trailing zeros after the significant digits, but keep at least 2 decimal places
        let trimmed = formatted.trim_end_matches('0');
        let trimmed = if trimmed.ends_with('.') {
            &formatted[..formatted.len().min(trimmed.len() + 2)]
        } else {
            trimmed
        };
        format!("${}", trimmed)
    } else {
        format!("${:.2}", usd)
    }
}

/// Format a token count in a human-readable way.
///
/// Examples: `"0"`, `"999"`, `"1.2K"`, `"45.2K"`, `"1.5M"`.
pub fn format_tokens(count: u64) -> String {
    if count < 1_000 {
        format!("{}", count)
    } else if count < 1_000_000 {
        let k = count as f64 / 1_000.0;
        if k >= 100.0 {
            format!("{:.0}K", k)
        } else if k >= 10.0 {
            format!("{:.1}K", k)
        } else {
            format!("{:.1}K", k)
        }
    } else {
        let m = count as f64 / 1_000_000.0;
        if m >= 100.0 {
            format!("{:.0}M", m)
        } else if m >= 10.0 {
            format!("{:.1}M", m)
        } else {
            format!("{:.1}M", m)
        }
    }
}

/// Format a duration in milliseconds as a human-readable string.
fn format_duration_ms(ms: u64) -> String {
    if ms < 1_000 {
        format!("{}ms", ms)
    } else if ms < 60_000 {
        let secs = ms as f64 / 1_000.0;
        format!("{:.1}s", secs)
    } else {
        let total_secs = ms / 1_000;
        let mins = total_secs / 60;
        let secs = total_secs % 60;
        if secs == 0 {
            format!("{}m", mins)
        } else {
            format!("{}m {}s", mins, secs)
        }
    }
}

/// Format a cost summary line.
///
/// Example output:
/// `"Total cost: $1.23 | Input: 45.2K tokens | Output: 12.1K tokens | Duration: 2m 30s"`
pub fn format_cost_summary(
    total_cost: f64,
    input_tokens: u64,
    output_tokens: u64,
    duration_ms: u64,
) -> String {
    format!(
        "Total cost: {} | Input: {} tokens | Output: {} tokens | Duration: {}",
        format_cost(total_cost),
        format_tokens(input_tokens),
        format_tokens(output_tokens),
        format_duration_ms(duration_ms),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- format_cost tests ---

    #[test]
    fn format_cost_zero() {
        assert_eq!(format_cost(0.0), "$0.00");
    }

    #[test]
    fn format_cost_small() {
        assert_eq!(format_cost(0.0001), "$0.0001");
    }

    #[test]
    fn format_cost_very_small() {
        assert_eq!(format_cost(0.0012), "$0.0012");
    }

    #[test]
    fn format_cost_penny() {
        assert_eq!(format_cost(0.01), "$0.01");
    }

    #[test]
    fn format_cost_normal() {
        assert_eq!(format_cost(1.23), "$1.23");
    }

    #[test]
    fn format_cost_large() {
        assert_eq!(format_cost(99.99), "$99.99");
    }

    #[test]
    fn format_cost_round_dollar() {
        assert_eq!(format_cost(5.0), "$5.00");
    }

    #[test]
    fn format_cost_near_threshold() {
        // 0.005 should use 2-decimal format
        assert_eq!(format_cost(0.005), "$0.01");
    }

    #[test]
    fn format_cost_just_below_threshold() {
        assert_eq!(format_cost(0.004), "$0.004");
    }

    // --- format_tokens tests ---

    #[test]
    fn format_tokens_zero() {
        assert_eq!(format_tokens(0), "0");
    }

    #[test]
    fn format_tokens_small() {
        assert_eq!(format_tokens(42), "42");
    }

    #[test]
    fn format_tokens_999() {
        assert_eq!(format_tokens(999), "999");
    }

    #[test]
    fn format_tokens_1k() {
        assert_eq!(format_tokens(1_000), "1.0K");
    }

    #[test]
    fn format_tokens_1234() {
        assert_eq!(format_tokens(1_234), "1.2K");
    }

    #[test]
    fn format_tokens_45200() {
        assert_eq!(format_tokens(45_200), "45.2K");
    }

    #[test]
    fn format_tokens_999999() {
        assert_eq!(format_tokens(999_999), "1000K");
    }

    #[test]
    fn format_tokens_1m() {
        assert_eq!(format_tokens(1_000_000), "1.0M");
    }

    #[test]
    fn format_tokens_1_5m() {
        assert_eq!(format_tokens(1_500_000), "1.5M");
    }

    #[test]
    fn format_tokens_100m() {
        assert_eq!(format_tokens(100_000_000), "100M");
    }

    #[test]
    fn format_tokens_150k() {
        assert_eq!(format_tokens(150_000), "150K");
    }

    // --- format_duration_ms tests ---

    #[test]
    fn duration_ms_small() {
        assert_eq!(format_duration_ms(500), "500ms");
    }

    #[test]
    fn duration_ms_seconds() {
        assert_eq!(format_duration_ms(5_000), "5.0s");
    }

    #[test]
    fn duration_ms_seconds_fraction() {
        assert_eq!(format_duration_ms(12_500), "12.5s");
    }

    #[test]
    fn duration_ms_minutes() {
        assert_eq!(format_duration_ms(150_000), "2m 30s");
    }

    #[test]
    fn duration_ms_exact_minutes() {
        assert_eq!(format_duration_ms(120_000), "2m");
    }

    #[test]
    fn duration_ms_zero() {
        assert_eq!(format_duration_ms(0), "0ms");
    }

    // --- format_cost_summary tests ---

    #[test]
    fn summary_basic() {
        let s = format_cost_summary(1.23, 45_200, 12_100, 150_000);
        assert_eq!(
            s,
            "Total cost: $1.23 | Input: 45.2K tokens | Output: 12.1K tokens | Duration: 2m 30s"
        );
    }

    #[test]
    fn summary_zero() {
        let s = format_cost_summary(0.0, 0, 0, 0);
        assert_eq!(
            s,
            "Total cost: $0.00 | Input: 0 tokens | Output: 0 tokens | Duration: 0ms"
        );
    }

    #[test]
    fn summary_large() {
        let s = format_cost_summary(42.50, 1_500_000, 500_000, 300_000);
        assert_eq!(
            s,
            "Total cost: $42.50 | Input: 1.5M tokens | Output: 500K tokens | Duration: 5m"
        );
    }

    #[test]
    fn summary_small_cost() {
        let s = format_cost_summary(0.0012, 100, 50, 800);
        assert_eq!(
            s,
            "Total cost: $0.0012 | Input: 100 tokens | Output: 50 tokens | Duration: 800ms"
        );
    }
}
