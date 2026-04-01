/// Truncate a string in the middle, replacing the removed section with "...".
/// If `max_len` is less than 4, just truncates from the end.
pub fn truncate_middle(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        return s.to_string();
    }
    if max_len < 4 {
        return s.chars().take(max_len).collect();
    }
    let keep_start = (max_len - 3) / 2;
    let keep_end = max_len - 3 - keep_start;
    let start: String = s.chars().take(keep_start).collect();
    let end: String = s.chars().skip(s.chars().count() - keep_end).collect();
    format!("{}...{}", start, end)
}

/// Indent every line in `text` by `spaces` number of spaces.
pub fn indent(text: &str, spaces: usize) -> String {
    let prefix = " ".repeat(spaces);
    text.lines()
        .map(|line| {
            if line.is_empty() {
                String::new()
            } else {
                format!("{}{}", prefix, line)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Remove common leading whitespace from all non-empty lines.
pub fn dedent(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let min_indent = lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);

    lines
        .iter()
        .map(|line| {
            if line.len() >= min_indent {
                &line[min_indent..]
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Count the number of lines in a string.
pub fn count_lines(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }
    text.lines().count()
}

/// Get the line at a given 0-based line number.
pub fn line_at(text: &str, line_number: usize) -> Option<&str> {
    text.lines().nth(line_number)
}

/// Convert text to a URL-friendly slug.
/// Lowercases, replaces non-alphanumeric chars with hyphens, collapses multiple hyphens.
pub fn slug(text: &str) -> String {
    let lowered = text.to_lowercase();
    let mut result = String::with_capacity(lowered.len());

    for c in lowered.chars() {
        if c.is_ascii_alphanumeric() {
            result.push(c);
        } else {
            result.push('-');
        }
    }

    // Collapse multiple hyphens
    let mut collapsed = String::with_capacity(result.len());
    let mut prev_hyphen = false;
    for c in result.chars() {
        if c == '-' {
            if !prev_hyphen {
                collapsed.push('-');
            }
            prev_hyphen = true;
        } else {
            collapsed.push(c);
            prev_hyphen = false;
        }
    }

    // Trim leading/trailing hyphens
    collapsed.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_middle_short() {
        assert_eq!(truncate_middle("hello", 10), "hello");
        assert_eq!(truncate_middle("hello world!", 10), "hel...rld!");
    }

    #[test]
    fn indent_basic() {
        assert_eq!(indent("a\nb\nc", 2), "  a\n  b\n  c");
        assert_eq!(indent("", 4), "");
    }

    #[test]
    fn dedent_basic() {
        let input = "    a\n    b\n    c";
        assert_eq!(dedent(input), "a\nb\nc");
    }

    #[test]
    fn count_lines_basic() {
        assert_eq!(count_lines("a\nb\nc"), 3);
        assert_eq!(count_lines(""), 0);
        assert_eq!(count_lines("single"), 1);
    }

    #[test]
    fn line_at_basic() {
        let text = "zero\none\ntwo";
        assert_eq!(line_at(text, 0), Some("zero"));
        assert_eq!(line_at(text, 1), Some("one"));
        assert_eq!(line_at(text, 2), Some("two"));
        assert_eq!(line_at(text, 3), None);
    }

    #[test]
    fn slug_basic() {
        assert_eq!(slug("Hello World!"), "hello-world");
        assert_eq!(slug("  foo  BAR  baz  "), "foo-bar-baz");
        assert_eq!(slug("already-slugged"), "already-slugged");
    }
}
