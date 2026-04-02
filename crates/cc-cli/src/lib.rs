use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "claude-code", version, about = "Claude Code CLI Agent")]
pub struct CliArgs {
    /// Initial prompt to send
    #[arg(short, long)]
    pub prompt: Option<String>,

    /// Model to use
    #[arg(short, long, env = "ANTHROPIC_MODEL")]
    pub model: Option<String>,

    /// Run in non-interactive (pipe) mode
    #[arg(long)]
    pub print: bool,

    /// Maximum budget in USD
    #[arg(long)]
    pub max_budget: Option<f64>,

    /// Enable verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// Resume a previous session (pass session ID, or omit to list recent sessions)
    #[arg(long, num_args = 0..=1, default_missing_value = "")]
    pub resume: Option<String>,

    /// Working directory
    #[arg(short = 'C', long)]
    pub cwd: Option<String>,

    /// System prompt override
    #[arg(long)]
    pub system_prompt: Option<String>,

    /// Append to system prompt
    #[arg(long)]
    pub append_system_prompt: Option<String>,

    /// Permission mode
    #[arg(long, default_value = "default")]
    pub permission_mode: String,

    /// Allowed tools (comma-separated)
    #[arg(long)]
    pub allowed_tools: Option<String>,

    /// Denied tools (comma-separated)
    #[arg(long)]
    pub disallowed_tools: Option<String>,

    /// Dump system prompt and exit
    #[arg(long)]
    pub dump_system_prompt: bool,

    /// Show version and exit
    #[arg(long)]
    pub version_info: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_args() {
        let args = CliArgs::parse_from(["claude-code"]);
        assert!(args.prompt.is_none());
        assert!(args.model.is_none());
        assert!(!args.print);
        assert!(!args.verbose);
        assert_eq!(args.permission_mode, "default");
    }

    #[test]
    fn test_prompt_arg() {
        let args = CliArgs::parse_from(["claude-code", "--prompt", "hello world"]);
        assert_eq!(args.prompt.as_deref(), Some("hello world"));
    }

    #[test]
    fn test_short_prompt() {
        let args = CliArgs::parse_from(["claude-code", "-p", "fix bug"]);
        assert_eq!(args.prompt.as_deref(), Some("fix bug"));
    }

    #[test]
    fn test_model_arg() {
        let args = CliArgs::parse_from(["claude-code", "--model", "claude-opus-4-6"]);
        assert_eq!(args.model.as_deref(), Some("claude-opus-4-6"));
    }

    #[test]
    fn test_print_mode() {
        let args = CliArgs::parse_from(["claude-code", "--print"]);
        assert!(args.print);
    }

    #[test]
    fn test_verbose() {
        let args = CliArgs::parse_from(["claude-code", "-v"]);
        assert!(args.verbose);
    }

    #[test]
    fn test_cwd_arg() {
        let args = CliArgs::parse_from(["claude-code", "-C", "/tmp"]);
        assert_eq!(args.cwd.as_deref(), Some("/tmp"));
    }

    #[test]
    fn test_max_budget() {
        let args = CliArgs::parse_from(["claude-code", "--max-budget", "5.0"]);
        assert_eq!(args.max_budget, Some(5.0));
    }

    #[test]
    fn test_version_info() {
        let args = CliArgs::parse_from(["claude-code", "--version-info"]);
        assert!(args.version_info);
    }

    #[test]
    fn test_dump_system_prompt() {
        let args = CliArgs::parse_from(["claude-code", "--dump-system-prompt"]);
        assert!(args.dump_system_prompt);
    }

    #[test]
    fn test_permission_mode() {
        let args = CliArgs::parse_from(["claude-code", "--permission-mode", "trust"]);
        assert_eq!(args.permission_mode, "trust");
    }

    #[test]
    fn test_allowed_tools() {
        let args = CliArgs::parse_from(["claude-code", "--allowed-tools", "Bash,Read"]);
        assert_eq!(args.allowed_tools.as_deref(), Some("Bash,Read"));
    }

    #[test]
    fn test_combined_args() {
        let args = CliArgs::parse_from([
            "claude-code",
            "-p",
            "hello",
            "-m",
            "opus",
            "-v",
            "--print",
            "--max-budget",
            "10.0",
        ]);
        assert_eq!(args.prompt.as_deref(), Some("hello"));
        assert_eq!(args.model.as_deref(), Some("opus"));
        assert!(args.verbose);
        assert!(args.print);
        assert_eq!(args.max_budget, Some(10.0));
    }
}
