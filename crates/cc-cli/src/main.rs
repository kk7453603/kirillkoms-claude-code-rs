use cc_cli::CliArgs;
use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = CliArgs::parse();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env().add_directive(
                if args.verbose {
                    "debug".parse()?
                } else {
                    "warn".parse()?
                },
            ),
        )
        .init();

    if args.version_info {
        println!("claude-code {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    // Set working directory
    if let Some(cwd) = &args.cwd {
        std::env::set_current_dir(cwd)?;
    }

    // Load configuration
    let project_root = std::env::current_dir()?;

    println!("Claude Code v{}", env!("CARGO_PKG_VERSION"));
    println!(
        "Model: {}",
        args.model
            .as_deref()
            .unwrap_or(cc_config::model_config::default_model())
    );
    println!("Working directory: {}", project_root.display());

    if args.dump_system_prompt {
        println!("\n(System prompt would be dumped here)");
        return Ok(());
    }

    if let Some(prompt) = args.prompt {
        println!("\nProcessing: {}", prompt);
        println!("(Full query loop integration pending)");
    } else if !args.print {
        println!("\nInteractive mode (TUI integration pending)");
        println!("Type your message or use /help for commands.");
    }

    Ok(())
}
