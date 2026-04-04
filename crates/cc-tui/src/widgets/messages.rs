use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::app::{ActiveTool, ChatMessage, ContentBlock, MessageRole, ScrollState};
use crate::progress::Spinner;
use crate::themes::Theme;

/// Parameters for rendering the message list.
pub struct MessagesRenderState<'a> {
    pub messages: &'a [ChatMessage],
    pub streaming_text: &'a str,
    pub streaming_thinking: &'a str,
    pub active_tool: Option<&'a ActiveTool>,
    pub thinking: bool,
    pub spinner: &'a Spinner,
    pub scroll: &'a mut ScrollState,
    pub theme: &'a Theme,
}

/// Render the message list area.
pub fn render_messages(frame: &mut Frame, area: Rect, state: &mut MessagesRenderState<'_>) {
    let thinking = state.thinking;
    let width = area.width as usize;
    let mut lines: Vec<Line<'static>> = Vec::new();

    for msg in state.messages {
        render_message(&mut lines, msg, state.theme, width);
        lines.push(Line::from(""));
    }

    // Thinking indicator
    if thinking {
        let spinner_char = state.spinner.frame();
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {} ", spinner_char),
                Style::default().fg(state.theme.spinner_color),
            ),
            Span::styled(
                "Thinking...",
                Style::default()
                    .fg(state.theme.thinking_color)
                    .add_modifier(Modifier::ITALIC),
            ),
        ]));
    }

    // Streaming text (markdown)
    if !state.streaming_text.is_empty() {
        let md_width = width.saturating_sub(2);
        let md_lines =
            crate::markdown::render_markdown(state.streaming_text, state.theme, md_width as u16);
        for md_line in md_lines {
            let mut indented = vec![Span::raw("  ".to_string())];
            indented.extend(md_line.spans);
            lines.push(Line::from(indented));
        }
    }

    // Active tool with spinner
    if let Some(tool) = &state.active_tool {
        let spinner_char = state.spinner.frame();
        let summary = truncate_to_width(&tool.input_summary, width.saturating_sub(12));
        let mut tool_spans = vec![
            Span::styled(
                format!("  {} ", spinner_char),
                Style::default().fg(state.theme.spinner_color),
            ),
            Span::styled(
                format!("⚡ {} ", tool.name),
                Style::default()
                    .fg(state.theme.tool_use_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ];
        if !summary.is_empty() {
            tool_spans.push(Span::styled(
                summary,
                Style::default().fg(state.theme.dim_color),
            ));
        }
        lines.push(Line::from(tool_spans));
    }

    // Scroll: skip/take approach (no Wrap — we control line width ourselves)
    state
        .scroll
        .set_content_size(lines.len(), area.height as usize);
    state.scroll.follow_if_needed();

    let total = lines.len();
    let visible = area.height as usize;
    let skip = if total > visible {
        let from_bottom = state.scroll.offset.min(total.saturating_sub(visible));
        total - visible - from_bottom
    } else {
        0
    };

    let visible_lines: Vec<Line> = lines.into_iter().skip(skip).take(visible).collect();
    let paragraph = Paragraph::new(visible_lines);
    frame.render_widget(paragraph, area);
}

/// Render a single ChatMessage into lines, truncating to `max_width`.
fn render_message(
    lines: &mut Vec<Line<'static>>,
    msg: &ChatMessage,
    theme: &Theme,
    max_width: usize,
) {
    // Header: only for User and System messages (Claude Code style — no "Assistant:" label)
    match msg.role {
        MessageRole::User => {
            lines.push(Line::from(Span::styled(
                " ❯ You",
                Style::default()
                    .fg(theme.user_msg_color)
                    .add_modifier(Modifier::BOLD),
            )));
        }
        MessageRole::System => {
            // Don't show header for info messages (only for errors)
            let is_error = msg
                .blocks
                .first()
                .is_some_and(|b| matches!(b, ContentBlock::Text(t) if t.starts_with("Error:")));
            if is_error {
                lines.push(Line::from(Span::styled(
                    " ! Error",
                    Style::default()
                        .fg(theme.error_color)
                        .add_modifier(Modifier::BOLD),
                )));
            }
        }
        MessageRole::Assistant => {
            // No header — assistant text flows directly
        }
    }

    for block in &msg.blocks {
        match block {
            ContentBlock::Text(text) => {
                if msg.role == MessageRole::Assistant {
                    let md_width = max_width.saturating_sub(2);
                    let md_lines =
                        crate::markdown::render_markdown(text, theme, md_width as u16);
                    for md_line in md_lines {
                        let mut indented_spans = vec![Span::raw("  ".to_string())];
                        indented_spans.extend(md_line.spans);
                        lines.push(Line::from(indented_spans));
                    }
                } else {
                    let color = match msg.role {
                        MessageRole::User => theme.user_msg_color,
                        MessageRole::System => {
                            if text.starts_with("Error:") {
                                theme.error_color
                            } else {
                                theme.info_color
                            }
                        }
                        _ => theme.assistant_msg_color,
                    };
                    for line_str in text.lines() {
                        for wrapped in wrap_text(&format!("  {}", line_str), max_width) {
                            lines.push(Line::from(Span::styled(
                                wrapped,
                                Style::default().fg(color),
                            )));
                        }
                    }
                }
            }
            ContentBlock::Thinking(text) => {
                // Claude Code style: always collapsed, one-line indicator
                let total_lines = text.lines().count();
                let duration_hint = if total_lines > 20 {
                    "extended"
                } else {
                    "brief"
                };
                lines.push(Line::from(vec![
                    Span::styled(
                        "  ▸ ",
                        Style::default().fg(theme.thinking_color),
                    ),
                    Span::styled(
                        format!("Thinking ({} — {} lines)", duration_hint, total_lines),
                        Style::default()
                            .fg(theme.thinking_color)
                            .add_modifier(Modifier::DIM),
                    ),
                ]));
            }
            ContentBlock::ToolUse {
                name,
                input_summary,
                result,
                collapsed,
                diff_data,
            } => {
                let status = match result {
                    Some(r) if r.is_error => {
                        format!("✗ {}", truncate_to_width(&r.summary, max_width / 2))
                    }
                    Some(r) => {
                        format!("✓ {}", truncate_to_width(&r.summary, max_width / 2))
                    }
                    None => "running...".to_string(),
                };

                let status_color = match result {
                    Some(r) if r.is_error => theme.error_color,
                    Some(_) => theme.tool_result_color,
                    None => theme.spinner_color,
                };

                // Tool label: ⚡ Name input_label status
                let label = result
                    .as_ref()
                    .and_then(|r| r.file_path.as_deref())
                    .or(if input_summary.is_empty() {
                        None
                    } else {
                        Some(input_summary.as_str())
                    });

                let mut tool_line = format!("  ⚡ {} ", name);
                if let Some(l) = label {
                    let trunc = truncate_to_width(l, max_width.saturating_sub(tool_line.len() + 20));
                    tool_line.push_str(&trunc);
                    tool_line.push(' ');
                }

                lines.push(Line::from(vec![
                    Span::styled(
                        tool_line,
                        Style::default()
                            .fg(theme.tool_use_color)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(status, Style::default().fg(status_color)),
                ]));

                if !collapsed && let Some(r) = result {
                    if let Some((old, new)) = diff_data {
                        let diff_view = crate::diff_view::DiffView::new(old, new);
                        let diff_lines = diff_view.unified_diff_lines();
                        for diff_line in diff_lines.iter().take(20) {
                            use crate::diff_view::DiffLine;
                            let (prefix, content, color) = match diff_line {
                                DiffLine::Added(s) => ("+", s.as_str(), Color::Green),
                                DiffLine::Removed(s) => ("-", s.as_str(), Color::Red),
                                DiffLine::Context(s) => (" ", s.as_str(), theme.dim_color),
                                DiffLine::Header(s) => ("@", s.as_str(), theme.dim_color),
                            };
                            let modifier = match diff_line {
                                DiffLine::Context(_) | DiffLine::Header(_) => Modifier::DIM,
                                _ => Modifier::empty(),
                            };
                            lines.push(Line::from(Span::styled(
                                truncate_to_width(
                                    &format!("    {}{}", prefix, content),
                                    max_width,
                                ),
                                Style::default().fg(color).add_modifier(modifier),
                            )));
                        }
                    } else {
                        for line_str in r.summary.lines().take(10) {
                            lines.push(Line::from(Span::styled(
                                truncate_to_width(&format!("    {}", line_str), max_width),
                                Style::default().fg(theme.dim_color),
                            )));
                        }
                    }
                }
            }
        }
    }
}

/// Truncate a string to fit within `max_chars` display width.
fn truncate_to_width(s: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    let mut width = 0;
    let mut end = 0;
    for (i, ch) in s.char_indices() {
        let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1);
        if width + cw > max_chars.saturating_sub(3) && i + ch.len_utf8() < s.len() {
            // Need truncation
            return format!("{}...", &s[..i]);
        }
        width += cw;
        end = i + ch.len_utf8();
    }
    s[..end].to_string()
}

/// Wrap text at word boundaries to fit within max_width.
/// Returns multiple lines if the text is too long.
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![String::new()];
    }
    let display_width = |s: &str| -> usize {
        s.chars()
            .map(|c| unicode_width::UnicodeWidthChar::width(c).unwrap_or(1))
            .sum()
    };

    if display_width(text) <= max_width {
        return vec![text.to_string()];
    }

    let mut result = Vec::new();
    let mut current = String::new();
    let mut current_width = 0;

    for word in text.split_inclusive(|c: char| c.is_whitespace()) {
        let word_width = display_width(word);
        if current_width + word_width > max_width && !current.is_empty() {
            result.push(current.trim_end().to_string());
            current = format!("  {}", word.trim_start()); // indent continuation
            current_width = display_width(&current);
        } else {
            current.push_str(word);
            current_width += word_width;
        }
    }
    if !current.is_empty() {
        result.push(current);
    }
    if result.is_empty() {
        result.push(text.to_string());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_render_messages_no_panic() {
        let messages = vec![
            ChatMessage {
                role: MessageRole::User,
                blocks: vec![ContentBlock::Text("Hello".to_string())],
                timestamp: Instant::now(),
            },
            ChatMessage {
                role: MessageRole::Assistant,
                blocks: vec![ContentBlock::Text("Hi there!".to_string())],
                timestamp: Instant::now(),
            },
        ];
        let theme = Theme::dark();
        let spinner = Spinner::new("test");
        let mut scroll = ScrollState::default();

        let backend = ratatui::backend::TestBackend::new(80, 20);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                render_messages(
                    f,
                    f.area(),
                    &mut MessagesRenderState {
                        messages: &messages,
                        streaming_text: "",
                        streaming_thinking: "",
                        active_tool: None,
                        thinking: false,
                        spinner: &spinner,
                        scroll: &mut scroll,
                        theme: &theme,
                    },
                );
            })
            .unwrap();
    }

    #[test]
    fn test_render_message_lines() {
        let msg = ChatMessage {
            role: MessageRole::User,
            blocks: vec![ContentBlock::Text("test line".to_string())],
            timestamp: Instant::now(),
        };
        let theme = Theme::dark();
        let mut lines = Vec::new();
        render_message(&mut lines, &msg, &theme, 80);
        assert!(lines.len() >= 2);
    }

    #[test]
    fn test_truncate_to_width() {
        assert_eq!(truncate_to_width("short", 80), "short");
        let long = "a".repeat(100);
        let trunc = truncate_to_width(&long, 20);
        assert!(trunc.len() <= 23); // 20 + "..."
        assert!(trunc.ends_with("..."));
    }

    #[test]
    fn test_truncate_empty() {
        assert_eq!(truncate_to_width("anything", 0), "");
    }
}
