use ratatui::prelude::*;
use ratatui::text::{Line, Span};
use std::sync::LazyLock;
use syntect::highlighting::{ThemeSet, Style as SyntectStyle};
use syntect::parsing::SyntaxSet;
use syntect::easy::HighlightLines;

static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);
static THEME_SET: LazyLock<ThemeSet> = LazyLock::new(ThemeSet::load_defaults);

/// Highlight source code and return styled ratatui Lines.
/// Falls back to plain text if the language is not recognized.
pub fn highlight_code(code: &str, language: &str) -> Vec<Line<'static>> {
    // Try to find syntax definition
    let syntax = if language.is_empty() {
        SYNTAX_SET.find_syntax_plain_text()
    } else {
        SYNTAX_SET
            .find_syntax_by_token(language)
            .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text())
    };

    let theme = &THEME_SET.themes["base16-ocean.dark"];
    let mut highlighter = HighlightLines::new(syntax, theme);

    let mut lines = Vec::new();

    for line_str in code.lines() {
        match highlighter.highlight_line(line_str, &SYNTAX_SET) {
            Ok(ranges) => {
                let spans: Vec<Span<'static>> = ranges
                    .into_iter()
                    .map(|(style, text)| {
                        Span::styled(text.to_string(), syntect_to_ratatui_style(style))
                    })
                    .collect();
                lines.push(Line::from(spans));
            }
            Err(_) => {
                // Fallback: plain text
                lines.push(Line::from(Span::raw(line_str.to_string())));
            }
        }
    }

    // Handle empty code
    if lines.is_empty() && !code.is_empty() {
        lines.push(Line::from(Span::raw(code.to_string())));
    }

    lines
}

/// Convert syntect style to ratatui Style.
fn syntect_to_ratatui_style(style: SyntectStyle) -> Style {
    let fg = Color::Rgb(
        style.foreground.r,
        style.foreground.g,
        style.foreground.b,
    );
    Style::default().fg(fg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlight_rust() {
        let code = "fn main() {\n    println!(\"hello\");\n}";
        let lines = highlight_code(code, "rust");
        assert_eq!(lines.len(), 3);
        // Each line should have at least one span
        for line in &lines {
            assert!(!line.spans.is_empty());
        }
    }

    #[test]
    fn test_highlight_unknown_language() {
        let code = "some text here";
        let lines = highlight_code(code, "nonexistent_lang_xyz");
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn test_highlight_empty_language() {
        let code = "plain text";
        let lines = highlight_code(code, "");
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn test_highlight_python() {
        let code = "def hello():\n    print('world')";
        let lines = highlight_code(code, "python");
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn test_highlight_javascript() {
        let code = "const x = 42;";
        let lines = highlight_code(code, "js");
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn test_highlight_empty_code() {
        let lines = highlight_code("", "rust");
        assert!(lines.is_empty());
    }

    #[test]
    fn test_syntect_to_ratatui() {
        let style = SyntectStyle {
            foreground: syntect::highlighting::Color { r: 255, g: 128, b: 0, a: 255 },
            background: syntect::highlighting::Color { r: 0, g: 0, b: 0, a: 255 },
            font_style: syntect::highlighting::FontStyle::empty(),
        };
        let ratatui_style = syntect_to_ratatui_style(style);
        assert_eq!(ratatui_style.fg, Some(Color::Rgb(255, 128, 0)));
    }
}
