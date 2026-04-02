use ratatui::prelude::*;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::app::AppMode;
use crate::input::TextInput;
use crate::themes::Theme;

/// Render the input area with optional autocomplete dropdown.
pub fn render_input_area(
    frame: &mut Frame,
    area: Rect,
    input: &TextInput,
    mode: AppMode,
    theme: &Theme,
    completions: &[String],
    completion_labels: &[String],
    completion_index: Option<usize>,
) {
    let border_color = match mode {
        AppMode::Input => theme.input_border_active,
        _ => theme.input_border_inactive,
    };

    let hint = if input.line_count() > 1 {
        " > (Shift+Enter: newline) "
    } else {
        match mode {
            AppMode::Input => " > ",
            AppMode::Scrolling => " SCROLL (i to type) ",
            AppMode::PermissionPrompt => " Permission Required ",
        }
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(hint)
        .border_style(Style::default().fg(border_color));

    let lines: Vec<Line> = input
        .lines()
        .iter()
        .map(|l| Line::from(l.as_str().to_string()))
        .collect();

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);

    // Render autocomplete dropdown ABOVE the input area
    if !completions.is_empty() && mode == AppMode::Input {
        let max_visible: usize = 8;
        let visible_count = completions.len().min(max_visible);
        let dropdown_height = visible_count as u16;

        // Use labels for display width calculation
        let labels = if completion_labels.len() == completions.len() {
            completion_labels
        } else {
            completions
        };

        let dropdown_width = labels
            .iter()
            .map(|c| c.len() as u16 + 3)
            .max()
            .unwrap_or(20)
            .clamp(15, area.width.saturating_sub(4));

        // Scroll offset: ensure selected item is always within visible window
        let scroll_off = if let Some(sel) = completion_index {
            if sel >= visible_count {
                sel - visible_count + 1
            } else {
                0
            }
        } else {
            0
        };

        let dropdown_y = area.y.saturating_sub(dropdown_height + 2);
        let dropdown_x = area.x + 1;

        // Build dropdown lines with scroll viewport, using labels for display
        let dropdown_lines: Vec<Line> = labels
            .iter()
            .enumerate()
            .skip(scroll_off)
            .take(visible_count)
            .map(|(i, label)| {
                let is_selected = Some(i) == completion_index;
                let style = if is_selected {
                    Style::default()
                        .bg(theme.input_border_active)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.assistant_msg_color)
                };
                Line::from(Span::styled(format!(" {} ", label), style))
            })
            .collect();

        let dropdown_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border_color));

        if dropdown_y > 0 {
            let total_height = dropdown_height + 2; // +2 for borders
            let total_width = (dropdown_width + 2).min(frame.area().width.saturating_sub(dropdown_x));
            let available_height = area.y.saturating_sub(dropdown_y);

            if available_height >= 3 {
                let render_height = total_height.min(available_height);
                let clamped = Rect::new(dropdown_x, dropdown_y, total_width, render_height);

                frame.render_widget(Clear, clamped);

                let inner_height = render_height.saturating_sub(2) as usize;
                let visible_lines: Vec<Line> = dropdown_lines
                    .into_iter()
                    .take(inner_height)
                    .collect();
                let widget = Paragraph::new(visible_lines).block(dropdown_block);
                frame.render_widget(widget, clamped);
            }
        }
    }

    // Show cursor in input mode
    if mode == AppMode::Input {
        let display_col: u16 = input.lines()[input.cursor_row()][..input.cursor_col()]
            .chars()
            .map(|c| unicode_width::UnicodeWidthChar::width(c).unwrap_or(1) as u16)
            .sum();
        let cursor_x = area.x + display_col + 1;
        let cursor_y = area.y + input.cursor_row() as u16 + 1;
        if cursor_x < area.x + area.width - 1 && cursor_y < area.y + area.height - 1 {
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_input_area_no_panic() {
        let input = TextInput::new();
        let theme = Theme::dark();
        let backend = ratatui::backend::TestBackend::new(80, 3);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                render_input_area(f, f.area(), &input, AppMode::Input, &theme, &[], &[], None);
            })
            .unwrap();
    }

    #[test]
    fn test_render_multiline_input() {
        let mut input = TextInput::new();
        input.insert_char('a');
        input.insert_newline();
        input.insert_char('b');

        let theme = Theme::dark();
        let backend = ratatui::backend::TestBackend::new(80, 5);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                render_input_area(f, f.area(), &input, AppMode::Input, &theme, &[], &[], None);
            })
            .unwrap();
    }

    #[test]
    fn test_render_with_completions() {
        let mut input = TextInput::new();
        for c in "/hel".chars() {
            input.insert_char(c);
        }
        let completions = vec!["help".to_string(), "history".to_string()];
        let theme = Theme::dark();
        let backend = ratatui::backend::TestBackend::new(80, 20);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                let area = Rect::new(0, 15, 80, 3);
                render_input_area(f, area, &input, AppMode::Input, &theme, &completions, &completions, Some(0));
            })
            .unwrap();
    }
}
