use regex::Regex;

/// Strip all ANSI escape codes from text.
pub fn strip_ansi(text: &str) -> String {
    // Matches ANSI escape sequences: ESC[ ... final_byte and OSC sequences
    let re = Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]|\x1b\].*?\x07|\x1b\[[0-9;]*m").unwrap();
    re.replace_all(text, "").to_string()
}

/// Compute the visible width of a string, ignoring ANSI escape codes.
pub fn visible_width(text: &str) -> usize {
    strip_ansi(text).chars().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_ansi_basic() {
        assert_eq!(strip_ansi("\x1b[31mred\x1b[0m"), "red");
        assert_eq!(strip_ansi("\x1b[1;32mbold green\x1b[0m"), "bold green");
    }

    #[test]
    fn strip_ansi_no_codes() {
        assert_eq!(strip_ansi("plain text"), "plain text");
    }

    #[test]
    fn strip_ansi_empty() {
        assert_eq!(strip_ansi(""), "");
    }

    #[test]
    fn visible_width_with_ansi() {
        assert_eq!(visible_width("\x1b[31mhello\x1b[0m"), 5);
        assert_eq!(visible_width("hello"), 5);
    }

    #[test]
    fn visible_width_empty() {
        assert_eq!(visible_width(""), 0);
    }
}
