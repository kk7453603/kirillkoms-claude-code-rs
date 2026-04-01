use regex::Regex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeBlock {
    pub language: Option<String>,
    pub code: String,
}

/// Strip markdown formatting, returning plain text.
/// Removes headers, bold, italic, code fences, links, images, etc.
pub fn strip_markdown(text: &str) -> String {
    let mut result = text.to_string();

    // Remove code blocks (fenced) - replace with just the code content
    let code_fence_re = Regex::new(r"```[^\n]*\n([\s\S]*?)```").unwrap();
    result = code_fence_re.replace_all(&result, "$1").to_string();

    // Remove inline code backticks
    let inline_code_re = Regex::new(r"`([^`]+)`").unwrap();
    result = inline_code_re.replace_all(&result, "$1").to_string();

    // Remove headers (# ... )
    let header_re = Regex::new(r"(?m)^#{1,6}\s+").unwrap();
    result = header_re.replace_all(&result, "").to_string();

    // Remove bold **text** or __text__
    let bold_re = Regex::new(r"\*\*(.+?)\*\*|__(.+?)__").unwrap();
    result = bold_re
        .replace_all(&result, |caps: &regex::Captures| {
            caps.get(1)
                .or_else(|| caps.get(2))
                .map_or("", |m| m.as_str())
                .to_string()
        })
        .to_string();

    // Remove italic *text* or _text_
    let italic_re = Regex::new(r"\*(.+?)\*|_(.+?)_").unwrap();
    result = italic_re
        .replace_all(&result, |caps: &regex::Captures| {
            caps.get(1)
                .or_else(|| caps.get(2))
                .map_or("", |m| m.as_str())
                .to_string()
        })
        .to_string();

    // Remove images ![alt](url) -> alt (must come before links)
    let img_re = Regex::new(r"!\[([^\]]*)\]\([^)]+\)").unwrap();
    result = img_re.replace_all(&result, "$1").to_string();

    // Remove links [text](url) -> text
    let link_re = Regex::new(r"\[([^\]]+)\]\([^)]+\)").unwrap();
    result = link_re.replace_all(&result, "$1").to_string();

    // Remove horizontal rules
    let hr_re = Regex::new(r"(?m)^[-*_]{3,}\s*$").unwrap();
    result = hr_re.replace_all(&result, "").to_string();

    // Remove blockquote markers
    let bq_re = Regex::new(r"(?m)^>\s?").unwrap();
    result = bq_re.replace_all(&result, "").to_string();

    result
}

/// Extract fenced code blocks from markdown text.
pub fn extract_code_blocks(text: &str) -> Vec<CodeBlock> {
    let re = Regex::new(r"```(\w*)\n([\s\S]*?)```").unwrap();
    let mut blocks = Vec::new();

    for caps in re.captures_iter(text) {
        let language = caps
            .get(1)
            .map(|m| m.as_str().to_string())
            .and_then(|s| if s.is_empty() { None } else { Some(s) });
        let code = caps.get(2).map_or("", |m| m.as_str()).to_string();
        blocks.push(CodeBlock { language, code });
    }

    blocks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_markdown_headers() {
        assert_eq!(strip_markdown("# Hello"), "Hello");
        assert_eq!(strip_markdown("## World"), "World");
        assert_eq!(strip_markdown("### Heading 3"), "Heading 3");
    }

    #[test]
    fn strip_markdown_bold_italic() {
        assert_eq!(strip_markdown("**bold**"), "bold");
        assert_eq!(strip_markdown("*italic*"), "italic");
        assert_eq!(strip_markdown("__also bold__"), "also bold");
    }

    #[test]
    fn strip_markdown_links() {
        assert_eq!(strip_markdown("[text](https://example.com)"), "text");
        assert_eq!(strip_markdown("![alt](image.png)"), "alt");
    }

    #[test]
    fn extract_code_blocks_basic() {
        let md = "Some text\n```rust\nfn main() {}\n```\nMore text\n```\nplain code\n```";
        let blocks = extract_code_blocks(md);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].language, Some("rust".to_string()));
        assert_eq!(blocks[0].code, "fn main() {}\n");
        assert_eq!(blocks[1].language, None);
        assert_eq!(blocks[1].code, "plain code\n");
    }

    #[test]
    fn extract_code_blocks_empty_input() {
        let blocks = extract_code_blocks("no code blocks here");
        assert!(blocks.is_empty());
    }
}
