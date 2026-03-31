/// Format a duration in human-readable form.
/// Examples: "45ms", "1.2s", "2m 30s", "1h 5m"
pub fn format_duration(ms: u64) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else if ms < 60_000 {
        let secs = ms as f64 / 1000.0;
        if ms % 1000 == 0 {
            format!("{}s", ms / 1000)
        } else {
            format!("{:.1}s", secs)
        }
    } else if ms < 3_600_000 {
        let minutes = ms / 60_000;
        let remaining_secs = (ms % 60_000) / 1000;
        if remaining_secs == 0 {
            format!("{}m", minutes)
        } else {
            format!("{}m {}s", minutes, remaining_secs)
        }
    } else {
        let hours = ms / 3_600_000;
        let remaining_mins = (ms % 3_600_000) / 60_000;
        if remaining_mins == 0 {
            format!("{}h", hours)
        } else {
            format!("{}h {}m", hours, remaining_mins)
        }
    }
}

/// Format a cost in USD.
/// Examples: "$0.01", "$1.23", "$0.00"
pub fn format_cost(usd: f64) -> String {
    format!("${:.2}", usd)
}

/// Format token count with human-readable suffixes.
/// Examples: "45", "1.2K", "1.5M"
pub fn format_tokens(count: u64) -> String {
    if count < 1000 {
        format!("{}", count)
    } else if count < 1_000_000 {
        let k = count as f64 / 1000.0;
        if count % 1000 == 0 {
            format!("{}K", count / 1000)
        } else {
            format!("{:.1}K", k)
        }
    } else {
        let m = count as f64 / 1_000_000.0;
        if count % 1_000_000 == 0 {
            format!("{}M", count / 1_000_000)
        } else {
            format!("{:.1}M", m)
        }
    }
}

/// Format file size in bytes with human-readable suffixes.
/// Examples: "45 B", "1.2 KB", "3.4 MB", "1.0 GB"
pub fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        let kb = bytes as f64 / 1024.0;
        format!("{:.1} KB", kb)
    } else if bytes < 1024 * 1024 * 1024 {
        let mb = bytes as f64 / (1024.0 * 1024.0);
        format!("{:.1} MB", mb)
    } else {
        let gb = bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        format!("{:.1} GB", gb)
    }
}

/// Truncate string with ellipsis if it exceeds max_len.
/// The resulting string (including "...") will be at most max_len characters.
pub fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        return s.to_string();
    }

    if max_len <= 3 {
        return ".".repeat(max_len);
    }

    let target = max_len - 3;
    // Find a valid char boundary
    let mut end = target;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }

    let mut result = s[..end].to_string();
    result.push_str("...");
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration_ms() {
        assert_eq!(format_duration(0), "0ms");
        assert_eq!(format_duration(45), "45ms");
        assert_eq!(format_duration(999), "999ms");
    }

    #[test]
    fn test_format_duration_seconds() {
        assert_eq!(format_duration(1000), "1s");
        assert_eq!(format_duration(1200), "1.2s");
        assert_eq!(format_duration(5000), "5s");
        assert_eq!(format_duration(59999), "60.0s");
    }

    #[test]
    fn test_format_duration_minutes() {
        assert_eq!(format_duration(60_000), "1m");
        assert_eq!(format_duration(150_000), "2m 30s");
        assert_eq!(format_duration(120_000), "2m");
    }

    #[test]
    fn test_format_duration_hours() {
        assert_eq!(format_duration(3_600_000), "1h");
        assert_eq!(format_duration(3_900_000), "1h 5m");
    }

    #[test]
    fn test_format_cost() {
        assert_eq!(format_cost(0.0), "$0.00");
        assert_eq!(format_cost(0.01), "$0.01");
        assert_eq!(format_cost(1.23), "$1.23");
        assert_eq!(format_cost(100.5), "$100.50");
    }

    #[test]
    fn test_format_tokens_small() {
        assert_eq!(format_tokens(0), "0");
        assert_eq!(format_tokens(45), "45");
        assert_eq!(format_tokens(999), "999");
    }

    #[test]
    fn test_format_tokens_thousands() {
        assert_eq!(format_tokens(1000), "1K");
        assert_eq!(format_tokens(1200), "1.2K");
        assert_eq!(format_tokens(1500), "1.5K");
        assert_eq!(format_tokens(999_999), "1000.0K");
    }

    #[test]
    fn test_format_tokens_millions() {
        assert_eq!(format_tokens(1_000_000), "1M");
        assert_eq!(format_tokens(1_500_000), "1.5M");
    }

    #[test]
    fn test_format_bytes_small() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(45), "45 B");
        assert_eq!(format_bytes(1023), "1023 B");
    }

    #[test]
    fn test_format_bytes_kb() {
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
    }

    #[test]
    fn test_format_bytes_mb() {
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(3 * 1024 * 1024 + 512 * 1024), "3.5 MB");
    }

    #[test]
    fn test_format_bytes_gb() {
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.0 GB");
    }

    #[test]
    fn test_truncate_str_no_truncation() {
        assert_eq!(truncate_str("hello", 10), "hello");
        assert_eq!(truncate_str("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_str_with_ellipsis() {
        assert_eq!(truncate_str("hello world", 8), "hello...");
    }

    #[test]
    fn test_truncate_str_very_short_max() {
        assert_eq!(truncate_str("hello", 3), "...");
        assert_eq!(truncate_str("hello", 2), "..");
        assert_eq!(truncate_str("hello", 1), ".");
        assert_eq!(truncate_str("hello", 0), "");
    }

    #[test]
    fn test_truncate_str_empty() {
        assert_eq!(truncate_str("", 5), "");
    }
}
