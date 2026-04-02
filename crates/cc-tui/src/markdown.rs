use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use ratatui::prelude::*;
use ratatui::text::{Line, Span};

use crate::syntax::highlight_code;
use crate::themes::Theme;

/// Convert markdown text into styled ratatui Lines.
pub fn render_markdown(text: &str, theme: &Theme, width: u16) -> Vec<Line<'static>> {
    let mut renderer = MarkdownRenderer::new(theme, width);
    renderer.render(text);
    renderer.lines
}

struct MarkdownRenderer<'t> {
    theme: &'t Theme,
    width: u16,
    lines: Vec<Line<'static>>,

    // Current line being built
    spans: Vec<Span<'static>>,

    // Style stack
    bold: bool,
    italic: bool,
    in_code_inline: bool,

    // Code block state
    in_code_block: bool,
    code_block_lang: String,
    code_block_content: String,

    // Heading level
    heading_level: u8,

    // List state
    list_depth: usize,
    list_ordered: bool,
    list_item_index: u64,
}

impl<'t> MarkdownRenderer<'t> {
    fn new(theme: &'t Theme, width: u16) -> Self {
        Self {
            theme,
            width,
            lines: Vec::new(),
            spans: Vec::new(),
            bold: false,
            italic: false,
            in_code_inline: false,
            in_code_block: false,
            code_block_lang: String::new(),
            code_block_content: String::new(),
            heading_level: 0,
            list_depth: 0,
            list_ordered: false,
            list_item_index: 0,
        }
    }

    fn render(&mut self, text: &str) {
        let options = Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES;
        let parser = Parser::new_ext(text, options);

        for event in parser {
            match event {
                Event::Start(tag) => self.start_tag(tag),
                Event::End(tag) => self.end_tag(tag),
                Event::Text(text) => self.handle_text(&text),
                Event::Code(code) => self.handle_inline_code(&code),
                Event::SoftBreak => self.soft_break(),
                Event::HardBreak => self.hard_break(),
                Event::Rule => self.rule(),
                _ => {}
            }
        }

        // Flush remaining spans
        self.flush_line();
    }

    fn start_tag(&mut self, tag: Tag) {
        match tag {
            Tag::Heading { level, .. } => {
                self.flush_line();
                self.heading_level = level as u8;
            }
            Tag::Paragraph => {
                // Space before paragraph (unless at start)
                if !self.lines.is_empty() {
                    self.lines.push(Line::from(""));
                }
            }
            Tag::Emphasis => {
                self.italic = true;
            }
            Tag::Strong => {
                self.bold = true;
            }
            Tag::CodeBlock(kind) => {
                self.flush_line();
                self.in_code_block = true;
                self.code_block_content.clear();
                self.code_block_lang = match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(lang) => lang.to_string(),
                    pulldown_cmark::CodeBlockKind::Indented => String::new(),
                };
            }
            Tag::List(start) => {
                if self.list_depth == 0 && !self.lines.is_empty() {
                    self.lines.push(Line::from(""));
                }
                self.list_depth += 1;
                self.list_ordered = start.is_some();
                self.list_item_index = start.unwrap_or(1);
            }
            Tag::Item => {
                self.flush_line();
                let indent = "  ".repeat(self.list_depth.saturating_sub(1));
                let bullet = if self.list_ordered {
                    let idx = self.list_item_index;
                    self.list_item_index += 1;
                    format!("{}{}. ", indent, idx)
                } else {
                    format!("{}• ", indent)
                };
                self.spans.push(Span::styled(
                    bullet,
                    Style::default().fg(self.theme.dim_color),
                ));
            }
            Tag::BlockQuote(_) => {
                self.flush_line();
                self.spans.push(Span::styled(
                    "│ ",
                    Style::default().fg(self.theme.dim_color),
                ));
            }
            _ => {}
        }
    }

    fn end_tag(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Heading(_) => {
                // Flush heading line with appropriate styling
                let level = self.heading_level;
                let spans = std::mem::take(&mut self.spans);

                let prefix = match level {
                    1 => "# ",
                    2 => "## ",
                    3 => "### ",
                    _ => "#### ",
                };

                let mut heading_spans = vec![Span::styled(
                    prefix.to_string(),
                    Style::default()
                        .fg(self.theme.heading_color)
                        .add_modifier(Modifier::BOLD),
                )];

                for span in spans {
                    heading_spans.push(Span::styled(
                        span.content.to_string(),
                        Style::default()
                            .fg(self.theme.heading_color)
                            .add_modifier(Modifier::BOLD),
                    ));
                }

                self.lines.push(Line::from(heading_spans));
                self.heading_level = 0;
            }
            TagEnd::Paragraph => {
                self.flush_line();
            }
            TagEnd::Emphasis => {
                self.italic = false;
            }
            TagEnd::Strong => {
                self.bold = false;
            }
            TagEnd::CodeBlock => {
                self.in_code_block = false;
                let lang = std::mem::take(&mut self.code_block_lang);
                let code = std::mem::take(&mut self.code_block_content);

                // Try syntax highlighting
                let highlighted = highlight_code(&code, &lang);

                // Add code block with border
                let block_width = (self.width as usize).saturating_sub(4);
                let lang_label = if lang.is_empty() {
                    String::new()
                } else {
                    format!(" {} ", lang)
                };

                // Top border
                let top_border = format!(
                    "  ┌{}{}┐",
                    lang_label,
                    "─".repeat(block_width.saturating_sub(lang_label.len()))
                );
                self.lines.push(Line::from(Span::styled(
                    top_border,
                    Style::default().fg(self.theme.code_border),
                )));

                // Code lines
                for line in highlighted {
                    let mut code_line_spans = vec![Span::styled(
                        "  │ ",
                        Style::default().fg(self.theme.code_border),
                    )];
                    code_line_spans.extend(line.spans);
                    self.lines.push(Line::from(code_line_spans));
                }

                // Bottom border
                let bottom_border = format!("  └{}┘", "─".repeat(block_width));
                self.lines.push(Line::from(Span::styled(
                    bottom_border,
                    Style::default().fg(self.theme.code_border),
                )));
            }
            TagEnd::List(_) => {
                self.list_depth = self.list_depth.saturating_sub(1);
                if self.list_depth == 0 {
                    self.flush_line();
                }
            }
            TagEnd::Item => {
                self.flush_line();
            }
            TagEnd::BlockQuote(_) => {
                self.flush_line();
            }
            _ => {}
        }
    }

    fn handle_text(&mut self, text: &str) {
        if self.in_code_block {
            self.code_block_content.push_str(text);
            return;
        }

        let style = self.current_style();
        self.spans
            .push(Span::styled(text.to_string(), style));
    }

    fn handle_inline_code(&mut self, code: &str) {
        self.in_code_inline = true;
        self.spans.push(Span::styled(
            format!("`{}`", code),
            Style::default()
                .fg(self.theme.tool_result_color)
                .add_modifier(Modifier::BOLD),
        ));
        self.in_code_inline = false;
    }

    fn soft_break(&mut self) {
        self.spans.push(Span::raw(" "));
    }

    fn hard_break(&mut self) {
        self.flush_line();
    }

    fn rule(&mut self) {
        self.flush_line();
        let rule = "─".repeat(self.width as usize);
        self.lines.push(Line::from(Span::styled(
            rule,
            Style::default().fg(self.theme.dim_color),
        )));
    }

    fn current_style(&self) -> Style {
        let mut style = Style::default();
        if self.bold {
            style = style
                .fg(self.theme.bold_color)
                .add_modifier(Modifier::BOLD);
        }
        if self.italic {
            style = style.add_modifier(Modifier::ITALIC);
        }
        style
    }

    fn flush_line(&mut self) {
        if !self.spans.is_empty() {
            let spans = std::mem::take(&mut self.spans);
            self.lines.push(Line::from(spans));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dark() -> Theme {
        Theme::dark()
    }

    #[test]
    fn test_plain_text() {
        let lines = render_markdown("Hello world", &dark(), 80);
        assert!(!lines.is_empty());
        let text: String = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.to_string())
            .collect();
        assert!(text.contains("Hello world"));
    }

    #[test]
    fn test_heading() {
        let lines = render_markdown("# Title", &dark(), 80);
        assert!(!lines.is_empty());
        let text: String = lines[0]
            .spans
            .iter()
            .map(|s| s.content.to_string())
            .collect();
        assert!(text.contains("# "));
        assert!(text.contains("Title"));
    }

    #[test]
    fn test_bold() {
        let lines = render_markdown("**bold text**", &dark(), 80);
        assert!(!lines.is_empty());
        let has_bold = lines.iter().any(|l| {
            l.spans.iter().any(|s| {
                s.content.contains("bold text")
                    && s.style
                        .add_modifier
                        .contains(Modifier::BOLD)
            })
        });
        assert!(has_bold);
    }

    #[test]
    fn test_italic() {
        let lines = render_markdown("*italic text*", &dark(), 80);
        assert!(!lines.is_empty());
        let has_italic = lines.iter().any(|l| {
            l.spans.iter().any(|s| {
                s.content.contains("italic text")
                    && s.style
                        .add_modifier
                        .contains(Modifier::ITALIC)
            })
        });
        assert!(has_italic);
    }

    #[test]
    fn test_inline_code() {
        let lines = render_markdown("Use `cargo build`", &dark(), 80);
        assert!(!lines.is_empty());
        let has_code = lines
            .iter()
            .any(|l| l.spans.iter().any(|s| s.content.contains("`cargo build`")));
        assert!(has_code);
    }

    #[test]
    fn test_code_block() {
        let md = "```rust\nfn main() {}\n```";
        let lines = render_markdown(md, &dark(), 80);
        // Should have top border, code line(s), bottom border
        assert!(lines.len() >= 3);
        let all_text: String = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.to_string())
            .collect::<Vec<_>>()
            .join("");
        // Syntect may tokenize "fn main() {}" differently
        assert!(all_text.contains("fn"));
        assert!(all_text.contains("main"));
        assert!(all_text.contains("┌"));
        assert!(all_text.contains("┘"));
    }

    #[test]
    fn test_unordered_list() {
        let md = "- item one\n- item two";
        let lines = render_markdown(md, &dark(), 80);
        let all_text: String = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.to_string())
            .collect::<Vec<_>>()
            .join("");
        assert!(all_text.contains("•"));
        assert!(all_text.contains("item one"));
        assert!(all_text.contains("item two"));
    }

    #[test]
    fn test_ordered_list() {
        let md = "1. first\n2. second";
        let lines = render_markdown(md, &dark(), 80);
        let all_text: String = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.to_string())
            .collect::<Vec<_>>()
            .join("");
        assert!(all_text.contains("1."));
        assert!(all_text.contains("first"));
    }

    #[test]
    fn test_horizontal_rule() {
        let lines = render_markdown("---", &dark(), 80);
        let has_rule = lines
            .iter()
            .any(|l| l.spans.iter().any(|s| s.content.contains("─")));
        assert!(has_rule);
    }

    #[test]
    fn test_empty_input() {
        let lines = render_markdown("", &dark(), 80);
        assert!(lines.is_empty());
    }

    #[test]
    fn test_multiple_paragraphs() {
        let md = "First paragraph.\n\nSecond paragraph.";
        let lines = render_markdown(md, &dark(), 80);
        // Should have empty line between paragraphs
        let has_empty = lines.iter().any(|l| l.spans.is_empty());
        assert!(has_empty);
    }

    #[test]
    fn test_code_block_without_lang() {
        let md = "```\nplain code\n```";
        let lines = render_markdown(md, &dark(), 80);
        let all_text: String = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.to_string())
            .collect::<Vec<_>>()
            .join("");
        assert!(all_text.contains("plain code"));
    }
}
