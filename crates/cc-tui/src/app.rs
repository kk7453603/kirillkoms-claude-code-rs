use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::input::TextInput;
use crate::message_list::MessageListState;
use crate::themes::Theme;

/// The main application state for the TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Normal,
    Input,
    PermissionPrompt,
    Scrolling,
}

/// A message to display in the TUI message list.
#[derive(Debug, Clone)]
pub struct DisplayMessage {
    pub role: String,
    pub content: String,
    pub tool_name: Option<String>,
}

/// Core application state that drives the TUI render loop.
pub struct App {
    pub mode: AppMode,
    pub input: TextInput,
    pub messages: MessageListState,
    pub display_messages: Vec<DisplayMessage>,
    pub theme: Theme,
    pub should_quit: bool,
    pub status_line: String,
    pub scroll_offset: usize,
}

impl App {
    pub fn new() -> Self {
        Self {
            mode: AppMode::Input,
            input: TextInput::new(),
            messages: MessageListState::new(),
            display_messages: Vec::new(),
            theme: Theme::default(),
            should_quit: false,
            status_line: String::new(),
            scroll_offset: 0,
        }
    }

    pub fn set_status(&mut self, msg: &str) {
        self.status_line = msg.to_string();
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    /// Add a display message to the message list.
    pub fn add_message(&mut self, role: &str, content: &str) {
        self.display_messages.push(DisplayMessage {
            role: role.to_string(),
            content: content.to_string(),
            tool_name: None,
        });
    }

    /// Add a display message with a tool name.
    pub fn add_tool_message(&mut self, role: &str, content: &str, tool_name: &str) {
        self.display_messages.push(DisplayMessage {
            role: role.to_string(),
            content: content.to_string(),
            tool_name: Some(tool_name.to_string()),
        });
    }

    /// Handle a key event and return the resulting action.
    pub fn handle_key_event(&mut self, key: KeyEvent) -> AppAction {
        // Ctrl+C always quits
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return AppAction::Quit;
        }

        match self.mode {
            AppMode::Input => self.handle_input_key(key),
            AppMode::Normal | AppMode::Scrolling => self.handle_scroll_key(key),
            AppMode::PermissionPrompt => AppAction::Continue,
        }
    }

    fn handle_input_key(&mut self, key: KeyEvent) -> AppAction {
        match key.code {
            KeyCode::Enter => {
                let content = self.input.content().to_string();
                if content.is_empty() {
                    return AppAction::Continue;
                }
                self.input.clear();
                AppAction::Submit(content)
            }
            KeyCode::Char(c) => {
                self.input.insert_char(c);
                AppAction::Continue
            }
            KeyCode::Backspace => {
                self.input.delete_char();
                AppAction::Continue
            }
            KeyCode::Left => {
                self.input.move_left();
                AppAction::Continue
            }
            KeyCode::Right => {
                self.input.move_right();
                AppAction::Continue
            }
            KeyCode::Up => {
                self.scroll_offset = self.scroll_offset.saturating_add(1);
                AppAction::Continue
            }
            KeyCode::Down => {
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
                AppAction::Continue
            }
            KeyCode::Esc => AppAction::Quit,
            _ => AppAction::Continue,
        }
    }

    fn handle_scroll_key(&mut self, key: KeyEvent) -> AppAction {
        match key.code {
            KeyCode::Up => {
                self.scroll_offset = self.scroll_offset.saturating_add(1);
                AppAction::Continue
            }
            KeyCode::Down => {
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
                AppAction::Continue
            }
            KeyCode::Char('i') => {
                self.mode = AppMode::Input;
                AppAction::Continue
            }
            KeyCode::Esc | KeyCode::Char('q') => AppAction::Quit,
            _ => AppAction::Continue,
        }
    }

    /// Draw the application UI.
    pub fn draw(&self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),   // messages
                Constraint::Length(3), // input
                Constraint::Length(1), // status bar
            ])
            .split(frame.area());

        // Messages area
        let messages: Vec<Line> = self
            .display_messages
            .iter()
            .flat_map(|m| {
                let role_style = match m.role.as_str() {
                    "user" => Style::default()
                        .fg(self.theme.user_msg_color)
                        .add_modifier(Modifier::BOLD),
                    "assistant" => Style::default()
                        .fg(self.theme.assistant_msg_color)
                        .add_modifier(Modifier::BOLD),
                    "tool" => Style::default().fg(self.theme.tool_use_color),
                    _ => Style::default(),
                };

                let header = if let Some(ref tool) = m.tool_name {
                    format!("{} [{}]:", m.role, tool)
                } else {
                    format!("{}:", m.role)
                };

                vec![
                    Line::from(Span::styled(header, role_style)),
                    Line::from(m.content.clone()),
                    Line::from(""),
                ]
            })
            .collect();

        let messages_widget = Paragraph::new(messages)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Claude Code")
                    .border_style(Style::default().fg(self.theme.border_color)),
            )
            .scroll((self.scroll_offset as u16, 0));
        frame.render_widget(messages_widget, chunks[0]);

        // Input area
        let input_title = match self.mode {
            AppMode::Input => "Input (Esc to quit)",
            AppMode::Normal | AppMode::Scrolling => "-- SCROLL -- (i to type)",
            AppMode::PermissionPrompt => "Permission Required",
        };
        let input_widget = Paragraph::new(self.input.content())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(input_title)
                    .border_style(Style::default().fg(self.theme.border_color)),
            );
        frame.render_widget(input_widget, chunks[1]);

        // Status bar
        let status = Paragraph::new(self.status_line.as_str())
            .style(Style::default().fg(self.theme.dim_color));
        frame.render_widget(status, chunks[2]);

        // Cursor position in input mode
        if self.mode == AppMode::Input {
            frame.set_cursor_position((
                chunks[1].x + self.input.cursor_position() as u16 + 1,
                chunks[1].y + 1,
            ));
        }
    }
}

/// Actions that can result from handling a key event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppAction {
    /// No state change needed beyond what handle_key_event already did.
    Continue,
    /// User submitted input text.
    Submit(String),
    /// User wants to quit the application.
    Quit,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_new() {
        let app = App::new();
        assert_eq!(app.mode, AppMode::Input);
        assert!(!app.should_quit);
        assert!(app.input.is_empty());
        assert!(app.display_messages.is_empty());
        assert_eq!(app.scroll_offset, 0);
    }

    #[test]
    fn test_app_quit() {
        let mut app = App::new();
        app.quit();
        assert!(app.should_quit);
    }

    #[test]
    fn test_set_status() {
        let mut app = App::new();
        app.set_status("Working...");
        assert_eq!(app.status_line, "Working...");
    }

    #[test]
    fn test_add_message() {
        let mut app = App::new();
        app.add_message("user", "Hello!");
        assert_eq!(app.display_messages.len(), 1);
        assert_eq!(app.display_messages[0].role, "user");
        assert_eq!(app.display_messages[0].content, "Hello!");
        assert!(app.display_messages[0].tool_name.is_none());
    }

    #[test]
    fn test_add_tool_message() {
        let mut app = App::new();
        app.add_tool_message("tool", "file.rs contents", "Read");
        assert_eq!(app.display_messages.len(), 1);
        assert_eq!(app.display_messages[0].tool_name.as_deref(), Some("Read"));
    }

    #[test]
    fn test_handle_char_input() {
        let mut app = App::new();
        let action = app.handle_key_event(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE));
        assert_eq!(action, AppAction::Continue);
        assert_eq!(app.input.content(), "h");
    }

    #[test]
    fn test_handle_enter_submits() {
        let mut app = App::new();
        app.handle_key_event(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE));
        app.handle_key_event(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
        let action = app.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(action, AppAction::Submit("hi".to_string()));
        assert!(app.input.is_empty());
    }

    #[test]
    fn test_handle_enter_empty_continues() {
        let mut app = App::new();
        let action = app.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(action, AppAction::Continue);
    }

    #[test]
    fn test_handle_backspace() {
        let mut app = App::new();
        app.handle_key_event(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
        app.handle_key_event(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE));
        app.handle_key_event(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!(app.input.content(), "a");
    }

    #[test]
    fn test_handle_esc_quits() {
        let mut app = App::new();
        let action = app.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(action, AppAction::Quit);
    }

    #[test]
    fn test_handle_ctrl_c_quits() {
        let mut app = App::new();
        let action = app.handle_key_event(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert_eq!(action, AppAction::Quit);
    }

    #[test]
    fn test_handle_up_scrolls() {
        let mut app = App::new();
        app.handle_key_event(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(app.scroll_offset, 1);
    }

    #[test]
    fn test_handle_down_scrolls() {
        let mut app = App::new();
        app.scroll_offset = 5;
        app.handle_key_event(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.scroll_offset, 4);
    }

    #[test]
    fn test_handle_down_at_zero() {
        let mut app = App::new();
        app.handle_key_event(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.scroll_offset, 0);
    }

    #[test]
    fn test_handle_left_right() {
        let mut app = App::new();
        app.handle_key_event(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
        app.handle_key_event(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE));
        assert_eq!(app.input.cursor_position(), 2);
        app.handle_key_event(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
        assert_eq!(app.input.cursor_position(), 1);
        app.handle_key_event(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
        assert_eq!(app.input.cursor_position(), 2);
    }

    #[test]
    fn test_normal_mode_scroll() {
        let mut app = App::new();
        app.mode = AppMode::Normal;
        app.handle_key_event(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(app.scroll_offset, 1);
    }

    #[test]
    fn test_normal_mode_i_switches_to_input() {
        let mut app = App::new();
        app.mode = AppMode::Normal;
        app.handle_key_event(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
        assert_eq!(app.mode, AppMode::Input);
    }

    #[test]
    fn test_normal_mode_q_quits() {
        let mut app = App::new();
        app.mode = AppMode::Normal;
        let action = app.handle_key_event(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert_eq!(action, AppAction::Quit);
    }

    #[test]
    fn test_permission_prompt_mode_continues() {
        let mut app = App::new();
        app.mode = AppMode::PermissionPrompt;
        let action = app.handle_key_event(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        assert_eq!(action, AppAction::Continue);
    }

    #[test]
    fn test_display_message_debug() {
        let msg = DisplayMessage {
            role: "user".to_string(),
            content: "test".to_string(),
            tool_name: None,
        };
        let debug = format!("{:?}", msg);
        assert!(debug.contains("user"));
    }

    #[test]
    fn test_app_action_variants() {
        let _a = AppAction::Continue;
        let _b = AppAction::Submit("test".to_string());
        let _c = AppAction::Quit;
    }

    #[test]
    fn test_default_app() {
        let app = App::default();
        assert_eq!(app.mode, AppMode::Input);
    }
}
