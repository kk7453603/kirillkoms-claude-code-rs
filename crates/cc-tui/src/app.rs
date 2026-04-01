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

/// Core application state that drives the TUI render loop.
pub struct App {
    pub mode: AppMode,
    pub input: TextInput,
    pub messages: MessageListState,
    pub theme: Theme,
    pub should_quit: bool,
    pub status_line: String,
}

impl App {
    pub fn new() -> Self {
        Self {
            mode: AppMode::Input,
            input: TextInput::new(),
            messages: MessageListState::new(),
            theme: Theme::default(),
            should_quit: false,
            status_line: String::new(),
        }
    }

    pub fn set_status(&mut self, msg: &str) {
        self.status_line = msg.to_string();
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }
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
}
