use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use crate::themes::Theme;

pub fn render_btw_overlay(frame: &mut Frame, area: Rect, text: &str, theme: &Theme) {
    let width = (area.width * 60 / 100).max(40).min(area.width - 4);
    let height = (area.height * 50 / 100).max(10).min(area.height - 4);
    let x = (area.width - width) / 2;
    let y = (area.height - height) / 2;
    let overlay = Rect::new(x, y, width, height);

    frame.render_widget(Clear, overlay);
    let block = Block::default()
        .title(" /btw ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.info_color))
        .title_style(Style::default().fg(theme.info_color).add_modifier(Modifier::BOLD));
    let inner = block.inner(overlay);
    frame.render_widget(block, overlay);

    // Render text with wrapping
    let paragraph = Paragraph::new(text.to_string()).wrap(Wrap { trim: true });
    frame.render_widget(paragraph, inner);
}
