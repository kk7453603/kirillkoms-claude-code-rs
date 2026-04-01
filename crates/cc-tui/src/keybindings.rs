use crossterm::event::KeyCode;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Submit,
    Cancel,
    ScrollUp,
    ScrollDown,
    History,
    Clear,
    Quit,
    Accept,
    Reject,
    ToggleVim,
}

pub struct KeyBindings {
    bindings: HashMap<KeyCode, Action>,
}

impl KeyBindings {
    pub fn default_bindings() -> Self {
        let mut bindings = HashMap::new();
        bindings.insert(KeyCode::Enter, Action::Submit);
        bindings.insert(KeyCode::Esc, Action::Cancel);
        bindings.insert(KeyCode::Up, Action::ScrollUp);
        bindings.insert(KeyCode::Down, Action::ScrollDown);
        bindings.insert(KeyCode::Tab, Action::History);
        bindings.insert(KeyCode::F(5), Action::Clear);
        bindings.insert(KeyCode::F(10), Action::Quit);
        bindings.insert(KeyCode::Char('y'), Action::Accept);
        bindings.insert(KeyCode::Char('n'), Action::Reject);
        bindings.insert(KeyCode::F(6), Action::ToggleVim);
        Self { bindings }
    }

    pub fn get_action(&self, key: KeyCode) -> Option<Action> {
        self.bindings.get(&key).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_bindings() {
        let kb = KeyBindings::default_bindings();
        assert_eq!(kb.get_action(KeyCode::Enter), Some(Action::Submit));
        assert_eq!(kb.get_action(KeyCode::Esc), Some(Action::Cancel));
        assert_eq!(kb.get_action(KeyCode::Up), Some(Action::ScrollUp));
        assert_eq!(kb.get_action(KeyCode::Down), Some(Action::ScrollDown));
    }

    #[test]
    fn test_unknown_key() {
        let kb = KeyBindings::default_bindings();
        assert_eq!(kb.get_action(KeyCode::F(12)), None);
    }

    #[test]
    fn test_quit_binding() {
        let kb = KeyBindings::default_bindings();
        assert_eq!(kb.get_action(KeyCode::F(10)), Some(Action::Quit));
    }
}
