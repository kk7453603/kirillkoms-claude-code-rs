use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::app::SessionInfo;
use crate::themes::Theme;

/// Render the welcome banner at the top of the screen.
pub fn render_banner(frame: &mut Frame, area: Rect, info: &SessionInfo, theme: &Theme) {
    let title = format!(" Claude Code v{}", info.version);
    let mut meta_parts = vec![format!("Model: {}", info.model)];
    if let Some(ref branch) = info.git_branch {
        meta_parts.push(format!("Branch: {}", branch));
    }
    let meta = format!(" {}", meta_parts.join(" | "));

    let text = vec![
        Line::from(Span::styled(
            title,
            Style::default()
                .fg(theme.banner_title_color)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            meta,
            Style::default().fg(theme.banner_info_color),
        )),
    ];

    let widget = Paragraph::new(text);
    frame.render_widget(widget, area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_banner_no_panic() {
        // Just verify the function can be called without panicking
        let info = SessionInfo {
            model: "opus".to_string(),
            cwd: "/tmp".to_string(),
            git_branch: Some("main".to_string()),
            session_id: "abc".to_string(),
            version: "0.1.0".to_string(),
        };
        let theme = Theme::dark();
        let backend = ratatui::backend::TestBackend::new(80, 5);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                render_banner(f, f.area(), &info, &theme);
            })
            .unwrap();
    }
}
