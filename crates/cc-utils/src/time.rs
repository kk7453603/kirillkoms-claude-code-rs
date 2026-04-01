use chrono::{DateTime, Utc};

/// Format a duration in seconds as a human-readable string.
/// Examples: "0.5s", "45s", "2m 30s", "1h 5m", "2d 3h"
pub fn human_duration(secs: f64) -> String {
    if secs < 0.0 {
        return "0s".to_string();
    }
    if secs < 1.0 {
        return format!("{:.1}s", secs);
    }
    if secs < 60.0 {
        let whole = secs as u64;
        if (secs - whole as f64).abs() < 0.05 {
            return format!("{}s", whole);
        }
        return format!("{:.1}s", secs);
    }
    let total_secs = secs as u64;
    if total_secs < 3600 {
        let minutes = total_secs / 60;
        let remaining = total_secs % 60;
        if remaining == 0 {
            return format!("{}m", minutes);
        }
        return format!("{}m {}s", minutes, remaining);
    }
    if total_secs < 86400 {
        let hours = total_secs / 3600;
        let remaining_mins = (total_secs % 3600) / 60;
        if remaining_mins == 0 {
            return format!("{}h", hours);
        }
        return format!("{}h {}m", hours, remaining_mins);
    }
    let days = total_secs / 86400;
    let remaining_hours = (total_secs % 86400) / 3600;
    if remaining_hours == 0 {
        return format!("{}d", days);
    }
    format!("{}d {}h", days, remaining_hours)
}

/// Return a human-readable relative time from an ISO 8601 timestamp.
/// Examples: "just now", "2 minutes ago", "3 hours ago", "1 day ago"
pub fn relative_time(timestamp: &str) -> String {
    let parsed = match timestamp.parse::<DateTime<Utc>>() {
        Ok(dt) => dt,
        Err(_) => return timestamp.to_string(),
    };

    let now = Utc::now();
    let duration = now.signed_duration_since(parsed);

    if duration.num_seconds() < 0 {
        return "in the future".to_string();
    }

    let secs = duration.num_seconds();
    if secs < 60 {
        return "just now".to_string();
    }
    let mins = secs / 60;
    if mins < 60 {
        return if mins == 1 {
            "1 minute ago".to_string()
        } else {
            format!("{} minutes ago", mins)
        };
    }
    let hours = mins / 60;
    if hours < 24 {
        return if hours == 1 {
            "1 hour ago".to_string()
        } else {
            format!("{} hours ago", hours)
        };
    }
    let days = hours / 24;
    if days < 30 {
        return if days == 1 {
            "1 day ago".to_string()
        } else {
            format!("{} days ago", days)
        };
    }
    let months = days / 30;
    if months < 12 {
        return if months == 1 {
            "1 month ago".to_string()
        } else {
            format!("{} months ago", months)
        };
    }
    let years = months / 12;
    if years == 1 {
        "1 year ago".to_string()
    } else {
        format!("{} years ago", years)
    }
}

/// Return the current time as an ISO 8601 string.
pub fn iso_now() -> String {
    Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_duration_basic() {
        assert_eq!(human_duration(0.5), "0.5s");
        assert_eq!(human_duration(5.0), "5s");
        assert_eq!(human_duration(65.0), "1m 5s");
        assert_eq!(human_duration(3661.0), "1h 1m");
        assert_eq!(human_duration(90000.0), "1d 1h");
    }

    #[test]
    fn human_duration_negative() {
        assert_eq!(human_duration(-1.0), "0s");
    }

    #[test]
    fn relative_time_invalid() {
        assert_eq!(relative_time("not a date"), "not a date");
    }

    #[test]
    fn iso_now_format() {
        let now = iso_now();
        // Should be parseable back
        assert!(now.parse::<DateTime<Utc>>().is_ok());
    }

    #[test]
    fn human_duration_exact_minutes() {
        assert_eq!(human_duration(60.0), "1m");
        assert_eq!(human_duration(120.0), "2m");
    }
}
