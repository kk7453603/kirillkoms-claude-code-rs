use std::collections::HashMap;
use crate::types::CommandDef;

pub struct CommandRegistry {
    commands: HashMap<String, &'static CommandDef>,
    aliases: HashMap<String, String>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
            aliases: HashMap::new(),
        }
    }

    pub fn register(&mut self, cmd: &'static CommandDef) {
        self.commands.insert(cmd.name.to_string(), cmd);
        for alias in cmd.aliases {
            self.aliases.insert(alias.to_string(), cmd.name.to_string());
        }
    }

    pub fn get(&self, name: &str) -> Option<&&'static CommandDef> {
        self.commands.get(name)
    }

    pub fn lookup(&self, name_or_alias: &str) -> Option<&&'static CommandDef> {
        if let Some(cmd) = self.commands.get(name_or_alias) {
            return Some(cmd);
        }
        if let Some(canonical) = self.aliases.get(name_or_alias) {
            return self.commands.get(canonical.as_str());
        }
        None
    }

    pub fn all_commands(&self) -> Vec<&&'static CommandDef> {
        self.commands.values().collect()
    }

    pub fn visible_commands(&self) -> Vec<&&'static CommandDef> {
        self.commands.values().filter(|cmd| !cmd.hidden).collect()
    }

    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        for cmd in crate::commands::all_commands() {
            registry.register(cmd);
        }
        registry
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands;

    #[test]
    fn test_register_and_get() {
        let mut reg = CommandRegistry::new();
        reg.register(&commands::help::HELP);
        assert!(reg.get("help").is_some());
        assert!(reg.get("nonexistent").is_none());
    }

    #[test]
    fn test_alias_lookup() {
        let mut reg = CommandRegistry::new();
        reg.register(&commands::help::HELP);
        // "h" and "?" are aliases for help
        let cmd = reg.lookup("h").expect("should find by alias");
        assert_eq!(cmd.name, "help");
        let cmd = reg.lookup("?").expect("should find by alias");
        assert_eq!(cmd.name, "help");
    }

    #[test]
    fn test_lookup_by_name() {
        let mut reg = CommandRegistry::new();
        reg.register(&commands::exit::EXIT);
        let cmd = reg.lookup("exit").expect("should find by name");
        assert_eq!(cmd.name, "exit");
    }

    #[test]
    fn test_lookup_alias_quit() {
        let mut reg = CommandRegistry::new();
        reg.register(&commands::exit::EXIT);
        let cmd = reg.lookup("quit").expect("should find by alias");
        assert_eq!(cmd.name, "exit");
    }

    #[test]
    fn test_with_defaults() {
        let reg = CommandRegistry::with_defaults();
        assert!(reg.get("help").is_some());
        assert!(reg.get("exit").is_some());
        assert!(reg.get("clear").is_some());
        assert!(reg.get("model").is_some());
        assert!(reg.all_commands().len() >= 50);
    }

    #[test]
    fn test_visible_commands_excludes_hidden() {
        let reg = CommandRegistry::with_defaults();
        let all = reg.all_commands().len();
        let visible = reg.visible_commands().len();
        // hooks and mcp are hidden, so visible < all
        assert!(visible < all);
        for cmd in reg.visible_commands() {
            assert!(!cmd.hidden);
        }
    }

    #[test]
    fn test_all_commands_count() {
        let cmds = commands::all_commands();
        assert_eq!(cmds.len(), 99);
    }
}
