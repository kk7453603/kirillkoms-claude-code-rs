use std::io::IsTerminal;
use std::path::PathBuf;
use std::sync::Arc;

use cc_cli::CliArgs;
use cc_engine::context::SystemContext;
use cc_engine::query_engine::QueryEngine;
use cc_engine::query_loop::QueryEvent;
use clap::Parser;
use tokio_stream::StreamExt;

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

    let project_root = std::env::current_dir()?;

    // --dump-system-prompt: assemble and print, then exit
    if args.dump_system_prompt {
        let ctx = build_system_context(&args, &project_root).await;
        let blocks = ctx.to_system_blocks();
        for block in &blocks {
            match block {
                cc_api::types::SystemBlock::Text { text, .. } => {
                    println!("{}", text);
                    println!("---");
                }
            }
        }
        return Ok(());
    }

    // Determine mode: pipe (--print or -p with prompt, or stdin is piped) vs interactive
    let has_piped_stdin = !std::io::stdin().is_terminal();
    let is_print_mode = args.print || (args.prompt.is_some() && has_piped_stdin);

    if is_print_mode {
        run_print_mode(args, project_root).await
    } else if args.prompt.is_some() {
        // Prompt given without --print: single query then interactive
        run_interactive_mode(args, project_root).await
    } else {
        run_interactive_mode(args, project_root).await
    }
}

/// Build the SystemContext from CLI args and environment.
async fn build_system_context(args: &CliArgs, project_root: &PathBuf) -> SystemContext {
    let mut ctx = SystemContext::from_env(project_root).await;

    if let Some(ref custom) = args.system_prompt {
        ctx.custom_system_prompt = Some(custom.clone());
    }
    if let Some(ref append) = args.append_system_prompt {
        ctx.append_system_prompt = Some(append.clone());
    }

    ctx
}

/// Create an API client, handling missing credentials gracefully.
async fn create_api_client() -> anyhow::Result<Arc<dyn cc_api::client::ApiClient>> {
    let config = match cc_api::auth::ApiConfig::from_env() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!();
            eprintln!("To use Claude Code, set one of the following environment variables:");
            eprintln!("  ANTHROPIC_API_KEY      - Your Anthropic API key");
            eprintln!("  ANTHROPIC_AUTH_TOKEN    - An OAuth token");
            eprintln!("  CLAUDE_CODE_OAUTH_TOKEN - An OAuth token (alternative)");
            eprintln!();
            eprintln!("Example:");
            eprintln!("  export ANTHROPIC_API_KEY=sk-ant-...");
            std::process::exit(1);
        }
    };

    let client = cc_api::client::create_client(config).await.map_err(|e| {
        anyhow::anyhow!("Failed to create API client: {}", e)
    })?;

    Ok(Arc::from(client))
}

/// Build a QueryEngine with all the standard setup.
async fn build_engine(args: &CliArgs, project_root: &PathBuf) -> anyhow::Result<QueryEngine> {
    let api_client = create_api_client().await?;

    let model_str = args
        .model
        .as_deref()
        .unwrap_or(cc_config::model_config::default_model());

    // Resolve aliases like "opus" -> "claude-opus-4-6"
    let model = cc_config::model_config::resolve_model_alias(model_str)
        .map(|s| s.to_string())
        .unwrap_or_else(|| model_str.to_string());

    let mut engine = QueryEngine::new(api_client, model);

    // Set up tools
    engine.tools = Arc::new(cc_tools::registry::ToolRegistry::with_defaults());

    // Set up system context
    let ctx = build_system_context(args, project_root).await;
    engine.set_system_context(ctx);

    Ok(engine)
}

/// Non-interactive pipe/print mode.
async fn run_print_mode(args: CliArgs, project_root: PathBuf) -> anyhow::Result<()> {
    let mut engine = build_engine(&args, &project_root).await?;

    // Gather the prompt: from --prompt arg, or from stdin, or both
    let mut prompt = String::new();

    if let Some(ref p) = args.prompt {
        prompt.push_str(p);
    }

    // Read from stdin if it is piped
    if !std::io::stdin().is_terminal() {
        use tokio::io::AsyncReadExt;
        let mut stdin_buf = String::new();
        let _ = tokio::io::stdin().read_to_string(&mut stdin_buf).await;
        if !stdin_buf.is_empty() {
            if !prompt.is_empty() {
                prompt.push('\n');
            }
            prompt.push_str(&stdin_buf);
        }
    }

    if prompt.trim().is_empty() {
        eprintln!("Error: No prompt provided. Use --prompt/-p or pipe input via stdin.");
        std::process::exit(1);
    }

    // Stream the response
    let verbose = args.verbose;
    let mut stream = std::pin::pin!(engine.submit_streaming(&prompt));

    while let Some(event) = stream.next().await {
        match event {
            QueryEvent::TextDelta(text) => {
                print!("{}", text);
            }
            QueryEvent::ThinkingDelta(text) => {
                if verbose {
                    eprint!("{}", text);
                }
            }
            QueryEvent::ToolUseStart { name, .. } => {
                eprintln!("[Tool: {}]", name);
            }
            QueryEvent::ToolResult {
                result, is_error, ..
            } => {
                if is_error {
                    eprintln!("[Tool Error: {}]", result);
                } else {
                    eprintln!("[Tool Result: {}]", result);
                }
            }
            QueryEvent::TurnComplete { .. } => {
                println!();
                break;
            }
            QueryEvent::Error(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
            QueryEvent::UsageUpdate { .. } => {}
        }
    }

    Ok(())
}

/// Interactive REPL mode.
async fn run_interactive_mode(args: CliArgs, project_root: PathBuf) -> anyhow::Result<()> {
    let mut engine = build_engine(&args, &project_root).await?;
    let command_registry = cc_commands::registry::CommandRegistry::with_defaults();
    let verbose = args.verbose;

    let model_str = args
        .model
        .as_deref()
        .unwrap_or(cc_config::model_config::default_model());
    let model_display = cc_config::model_config::resolve_model_alias(model_str)
        .unwrap_or(model_str);

    println!("Claude Code v{}", env!("CARGO_PKG_VERSION"));
    println!("Model: {}", model_display);
    println!("Working directory: {}", project_root.display());
    println!();
    println!("Type your message, or use /help for commands. Press Ctrl+C to exit.");
    println!();

    // If a prompt was given on the CLI, process it first
    if let Some(ref initial_prompt) = args.prompt {
        stream_response(&mut engine, initial_prompt, verbose).await?;
    }

    // Set up Ctrl+C handler
    let running = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let r = running.clone();
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        r.store(false, std::sync::atomic::Ordering::SeqCst);
    });

    // REPL loop using line-buffered stdin
    use tokio::io::AsyncBufReadExt;
    let stdin = tokio::io::BufReader::new(tokio::io::stdin());
    let mut lines = stdin.lines();

    loop {
        if !running.load(std::sync::atomic::Ordering::SeqCst) {
            println!("\nGoodbye!");
            break;
        }

        eprint!("> ");

        let line: String = match lines.next_line().await {
            Ok(Some(line)) => line,
            Ok(None) => {
                // EOF
                println!("\nGoodbye!");
                break;
            }
            Err(e) => {
                eprintln!("Input error: {}", e);
                break;
            }
        };

        let line = line.trim().to_string();

        if line.is_empty() {
            continue;
        }

        // Check for slash commands
        if line.starts_with('/') {
            let (cmd_name, cmd_args): (&str, &str) = match line[1..].split_once(' ') {
                Some((name, rest)) => (name, rest.trim()),
                None => (&line[1..], ""),
            };

            if let Some(cmd_def) = command_registry.lookup(cmd_name) {
                match (cmd_def.handler)(cmd_args).await {
                    Ok(output) => {
                        if let Some(msg) = output.message {
                            println!("{}", msg);
                        }
                        if !output.should_continue {
                            println!("Goodbye!");
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("Command error: {}", e);
                    }
                }
            } else {
                eprintln!("Unknown command: /{}. Type /help for available commands.", cmd_name);
            }

            continue;
        }

        // Regular message: submit to engine
        if let Err(e) = stream_response(&mut engine, &line, verbose).await {
            eprintln!("Error: {}", e);
        }
    }

    Ok(())
}

/// Stream a query response to stdout.
async fn stream_response(
    engine: &mut QueryEngine,
    prompt: &str,
    verbose: bool,
) -> anyhow::Result<()> {
    let mut stream = std::pin::pin!(engine.submit_streaming(prompt));

    while let Some(event) = stream.next().await {
        match event {
            QueryEvent::TextDelta(text) => {
                print!("{}", text);
            }
            QueryEvent::ThinkingDelta(text) => {
                if verbose {
                    eprint!("{}", text);
                }
            }
            QueryEvent::ToolUseStart { name, .. } => {
                eprintln!("\n[Tool: {}]", name);
            }
            QueryEvent::ToolResult {
                result, is_error, ..
            } => {
                if is_error {
                    eprintln!("[Tool Error: {}]", result);
                } else {
                    eprintln!("[Tool Result: {}]", result);
                }
            }
            QueryEvent::TurnComplete { .. } => {
                println!();
            }
            QueryEvent::Error(e) => {
                return Err(anyhow::anyhow!("{}", e));
            }
            QueryEvent::UsageUpdate {
                input_tokens,
                output_tokens,
            } => {
                if verbose {
                    eprintln!(
                        "[Tokens: {} in, {} out]",
                        input_tokens, output_tokens
                    );
                }
            }
        }
    }

    Ok(())
}
