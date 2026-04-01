use cc_tools::trait_def::RenderedContent;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// Convert RenderedContent to ratatui spans
pub fn to_spans(content: &RenderedContent) -> Vec<Span<'static>> {
    match content {
        RenderedContent::Text(text) => {
            vec![Span::raw(text.clone())]
        }
        RenderedContent::Styled {
            text,
            bold,
            dim,
            color,
        } => {
            let mut style = Style::default();
            if *bold {
                style = style.add_modifier(Modifier::BOLD);
            }
            if *dim {
                style = style.add_modifier(Modifier::DIM);
            }
            if let Some(color_name) = color {
                style = style.fg(parse_color(color_name));
            }
            vec![Span::styled(text.clone(), style)]
        }
        RenderedContent::Diff {
            old,
            new,
            file_path,
        } => {
            let mut spans = Vec::new();
            if let Some(path) = file_path {
                spans.push(Span::styled(
                    format!("--- {path}\n"),
                    Style::default().fg(Color::DarkGray),
                ));
            }
            for line in old.lines() {
                spans.push(Span::styled(
                    format!("- {line}\n"),
                    Style::default().fg(Color::Red),
                ));
            }
            for line in new.lines() {
                spans.push(Span::styled(
                    format!("+ {line}\n"),
                    Style::default().fg(Color::Green),
                ));
            }
            spans
        }
        RenderedContent::Lines(lines) => {
            let mut spans = Vec::new();
            for line in lines {
                spans.extend(to_spans(line));
            }
            spans
        }
        RenderedContent::Empty => vec![],
    }
}

/// Convert to full Line
pub fn to_line(content: &RenderedContent) -> Line<'static> {
    Line::from(to_spans(content))
}

fn parse_color(name: &str) -> Color {
    match name.to_lowercase().as_str() {
        "red" => Color::Red,
        "green" => Color::Green,
        "blue" => Color::Blue,
        "yellow" => Color::Yellow,
        "cyan" => Color::Cyan,
        "magenta" => Color::Magenta,
        "white" => Color::White,
        "black" => Color::Black,
        "gray" | "grey" => Color::Gray,
        "darkgray" | "darkgrey" => Color::DarkGray,
        _ => Color::Reset,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_to_spans() {
        let content = RenderedContent::Text("hello".to_string());
        let spans = to_spans(&content);
        assert_eq!(spans.len(), 1);
    }

    #[test]
    fn test_empty_to_spans() {
        let content = RenderedContent::Empty;
        let spans = to_spans(&content);
        assert!(spans.is_empty());
    }

    #[test]
    fn test_styled_to_spans() {
        let content = RenderedContent::Styled {
            text: "bold text".to_string(),
            bold: true,
            dim: false,
            color: Some("red".to_string()),
        };
        let spans = to_spans(&content);
        assert_eq!(spans.len(), 1);
    }

    #[test]
    fn test_to_line() {
        let content = RenderedContent::Text("line".to_string());
        let line = to_line(&content);
        assert_eq!(line.spans.len(), 1);
    }

    #[test]
    fn test_parse_color() {
        assert_eq!(parse_color("red"), Color::Red);
        assert_eq!(parse_color("GREEN"), Color::Green);
        assert_eq!(parse_color("unknown"), Color::Reset);
    }
}
