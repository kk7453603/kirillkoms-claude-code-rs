use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::app::{AppMode, UsageInfo};
use crate::themes::Theme;

/// Render the status bar at the bottom of the screen.
pub fn render_status_bar(
    frame: &mut Frame,
    area: Rect,
    model: &str,
    usage: &UsageInfo,
    mode: AppMode,
    theme: &Theme,
) {
    let input_tokens = cc_cost::format::format_tokens(usage.input_tokens);
    let output_tokens = cc_cost::format::format_tokens(usage.output_tokens);
    let cost = cc_cost::format::format_cost(usage.cost_usd);

    let mode_indicator = match mode {
        AppMode::Input => "INPUT",
        AppMode::Scrolling => "SCROLL",
        AppMode::PermissionPrompt => "PERMISSION",
    };

    let left = format!(" {} | {}in {}out | {} ", model, input_tokens, output_tokens, cost);
    let right = format!(" {} ", mode_indicator);

    let left_width = left.len() as u16;
    let right_width = right.len() as u16;
    let padding = area
        .width
        .saturating_sub(left_width + right_width);

    let line = Line::from(vec![
        Span::styled(
            left,
            Style::default()
                .fg(theme.status_bar_fg)
                .bg(theme.status_bar_bg),
        ),
        Span::styled(
            " ".repeat(padding as usize),
            Style::default().bg(theme.status_bar_bg),
        ),
        Span::styled(
            right,
            Style::default()
                .fg(theme.status_bar_fg)
                .bg(theme.status_bar_bg)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let widget = Paragraph::new(line);
    frame.render_widget(widget, area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_status_bar_no_panic() {
        let usage = UsageInfo::default();
        let theme = Theme::dark();
        let backend = ratatui::backend::TestBackend::new(80, 1);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                render_status_bar(f, f.area(), "opus", &usage, AppMode::Input, &theme);
            })
            .unwrap();
    }
}
