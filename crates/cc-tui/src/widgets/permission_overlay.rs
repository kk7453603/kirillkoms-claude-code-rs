use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::app::PendingPermission;
use crate::themes::Theme;

/// Render a centered permission dialog overlay.
pub fn render_permission_overlay(
    frame: &mut Frame,
    area: Rect,
    perm: &PendingPermission,
    theme: &Theme,
) {
    let width = (area.width * 70 / 100).max(50).min(area.width.saturating_sub(4));
    let height = 10_u16.min(area.height.saturating_sub(4));
    let x = area.width.saturating_sub(width) / 2;
    let y = area.height.saturating_sub(height) / 2;

    let overlay_area = Rect::new(x, y, width, height);

    // Clear background completely
    frame.render_widget(Clear, overlay_area);

    let block = Block::default()
        .title(" ⚠ Permission Required ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.permission_border))
        .title_style(
            Style::default()
                .fg(theme.permission_highlight)
                .add_modifier(Modifier::BOLD),
        );

    let inner = block.inner(overlay_area);
    frame.render_widget(block, overlay_area);

    let max_text_width = inner.width.saturating_sub(4) as usize;

    // Truncate input summary to fit
    let input_display = if perm.input_summary.len() > max_text_width {
        format!("{}...", &perm.input_summary[..max_text_width.saturating_sub(3)])
    } else {
        perm.input_summary.clone()
    };

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Tool: ", Style::default().fg(theme.dim_color)),
            Span::styled(
                perm.tool_name.clone(),
                Style::default()
                    .fg(theme.tool_use_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(Span::styled(
            format!("  {}", truncate(&perm.message, max_text_width)),
            Style::default().fg(theme.assistant_msg_color),
        )),
    ];

    if !input_display.is_empty() {
        lines.push(Line::from(Span::styled(
            format!("  > {}", input_display),
            Style::default().fg(theme.dim_color),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(
            "  [y] Allow  ",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " [n] Deny  ",
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " [a] Always  ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    ]));

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else if max > 3 {
        format!("{}...", &s[..max - 3])
    } else {
        s[..max].to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_permission_overlay_no_panic() {
        let perm = PendingPermission {
            tool_name: "Bash".to_string(),
            message: "Run a command".to_string(),
            input_summary: "ls -la".to_string(),
        };
        let theme = Theme::dark();
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                render_permission_overlay(f, f.area(), &perm, &theme);
            })
            .unwrap();
    }

    #[test]
    fn test_long_input_truncated() {
        let perm = PendingPermission {
            tool_name: "NotebookEdit".to_string(),
            message: "Tool 'NotebookEdit' requires permission".to_string(),
            input_summary: "x".repeat(200),
        };
        let theme = Theme::dark();
        let backend = ratatui::backend::TestBackend::new(100, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                render_permission_overlay(f, f.area(), &perm, &theme);
            })
            .unwrap();
    }
}
