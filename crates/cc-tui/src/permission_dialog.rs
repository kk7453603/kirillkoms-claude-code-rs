/// State for the permission approval dialog.
#[derive(Debug, Clone)]
pub struct PermissionDialog {
    pub tool_name: String,
    pub description: String,
    pub selected: PermissionChoice,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionChoice {
    Accept,
    Reject,
}

impl PermissionDialog {
    pub fn new(tool_name: &str, description: &str) -> Self {
        Self {
            tool_name: tool_name.to_string(),
            description: description.to_string(),
            selected: PermissionChoice::Accept,
        }
    }

    pub fn toggle(&mut self) {
        self.selected = match self.selected {
            PermissionChoice::Accept => PermissionChoice::Reject,
            PermissionChoice::Reject => PermissionChoice::Accept,
        };
    }

    pub fn is_accepted(&self) -> bool {
        self.selected == PermissionChoice::Accept
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dialog_defaults_to_accept() {
        let dialog = PermissionDialog::new("Bash", "Run a command");
        assert!(dialog.is_accepted());
    }

    #[test]
    fn test_toggle() {
        let mut dialog = PermissionDialog::new("Bash", "Run a command");
        dialog.toggle();
        assert!(!dialog.is_accepted());
        dialog.toggle();
        assert!(dialog.is_accepted());
    }
}
