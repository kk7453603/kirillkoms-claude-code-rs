use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use cc_cli::CliArgs;
use cc_engine::context::SystemContext;
use cc_engine::query_engine::QueryEngine;
use cc_engine::query_loop::QueryEvent;
use cc_engine::tool_execution::{AutoApproveCallback, ExecutionContext};
use cc_permissions::checker::PermissionContext;
use cc_permissions::modes::PermissionMode;
use cc_tui::app::SessionInfo;
use cc_tui::runner::TuiRunner;
use clap::Parser;
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = CliArgs::parse();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env().add_directive(if args.verbose {
                "debug".parse()?
            } else {
                "warn".parse()?
            }),
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
            eprintln!("  OPENAI_API_KEY         - An OpenAI-compatible API key");
            eprintln!();
            eprintln!("Example:");
            eprintln!("  export ANTHROPIC_API_KEY=sk-ant-...");
            eprintln!("  export OPENAI_API_KEY=sk-... OPENAI_BASE_URL=http://localhost:11434");
            std::process::exit(1);
        }
    };

    let client = cc_api::client::create_client(config)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create API client: {}", e))?;

    Ok(Arc::from(client))
}

/// Build a QueryEngine with all the standard setup.
async fn build_engine(
    args: &CliArgs,
    project_root: &PathBuf,
    is_interactive: bool,
) -> anyhow::Result<QueryEngine> {
    let api_client = create_api_client().await?;

    // Model resolution: CLI arg > OPENAI_MODEL (for OpenAI provider) > default
    let openai_model_env = std::env::var("OPENAI_MODEL").ok();
    let default_model = if openai_model_env.is_some()
        && cc_api::auth::ApiProvider::from_env() == cc_api::auth::ApiProvider::OpenAiCompatible
    {
        openai_model_env.as_deref().unwrap_or(cc_config::model_config::default_model())
    } else {
        cc_config::model_config::default_model()
    };
    let model_str = args.model.as_deref().unwrap_or(default_model);

    // Resolve aliases like "opus" -> "claude-opus-4-6"
    let model = cc_config::model_config::resolve_model_alias(model_str)
        .map(|s| s.to_string())
        .unwrap_or_else(|| model_str.to_string());

    let mut engine = QueryEngine::new(api_client, model);

    // Set up turn timeout from env
    if let Some(secs) = cc_config::env::EnvConfig::from_env().turn_timeout_secs {
        engine.turn_timeout = Some(std::time::Duration::from_secs(secs));
    }

    // Set up tools
    engine.tools = Arc::new(cc_tools::registry::ToolRegistry::with_defaults());

    // Set up system context
    let ctx = build_system_context(args, project_root).await;
    engine.set_system_context(ctx);

    // Set up permission context based on --permission-mode
    let permission_mode =
        PermissionMode::from_str_opt(&args.permission_mode).unwrap_or(PermissionMode::Default);
    let permission_ctx = PermissionContext::new(permission_mode);

    // Build execution context with hooks config and permission context
    let hooks_config = cc_hooks::types::HooksConfig::new();
    let session_id = uuid::Uuid::new_v4().to_string();

    let mut exec_ctx = ExecutionContext::new(permission_ctx, project_root.clone());
    exec_ctx.hooks_config = hooks_config;
    exec_ctx.session_id = Some(session_id.clone());

    engine.set_execution_context(exec_ctx);

    // Set permission callback for non-interactive mode.
    // In interactive (TUI) mode, TuiRunner sets its own callback.
    if !is_interactive {
        engine.set_permission_callback(Arc::new(AutoApproveCallback));
    }

    // Enable session persistence (same path as /resume command uses)
    let sessions_dir = cc_config::paths::sessions_dir();
    engine.enable_persistence(sessions_dir.clone(), session_id.clone());

    // Persist session start entry
    let start_path = cc_session::storage::transcript_path(&sessions_dir, &session_id);
    let start_entry = cc_session::persistence::TranscriptEntry {
        timestamp: chrono::Utc::now().to_rfc3339(),
        entry_type: "session_start".to_string(),
        data: serde_json::json!({
            "project_root": project_root.display().to_string(),
            "model": &engine.model,
            "permission_mode": args.permission_mode,
        }),
    };
    if let Err(e) = cc_session::persistence::append_entry(&start_path, &start_entry) {
        tracing::warn!("Failed to persist session start: {}", e);
    }

    Ok(engine)
}

/// Non-interactive pipe/print mode.
async fn run_print_mode(args: CliArgs, project_root: PathBuf) -> anyhow::Result<()> {
    let mut engine = build_engine(&args, &project_root, false).await?;

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

/// Load MCP settings from the settings.json files (global + project), connect clients,
/// list their tools and register everything into the ToolRegistry.
///
/// This runs after the engine is built so the registry is already populated with
/// built-in tools.  MCP tools are added on top.
async fn init_mcp(engine: &mut cc_engine::query_engine::QueryEngine, project_root: &PathBuf) {
    use cc_mcp::client::{McpClient, StdioMcpClient};
    use std::sync::Arc;

    // Merge settings: global (~/.claude/settings.json) + project (.claude/settings.json)
    let mut merged = serde_json::json!({});
    for path in [
        cc_config::paths::global_settings_path(),
        cc_config::paths::project_settings_path(project_root),
    ] {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) {
                // Merge mcpServers keys (project overrides global for same name)
                if let Some(servers) = value.get("mcpServers").and_then(|v| v.as_object()) {
                    let target = merged
                        .as_object_mut()
                        .unwrap()
                        .entry("mcpServers")
                        .or_insert_with(|| serde_json::json!({}));
                    for (k, v) in servers {
                        target
                            .as_object_mut()
                            .unwrap()
                            .insert(k.clone(), v.clone());
                    }
                }
            }
        }
    }

    let configs = cc_mcp::config::load_mcp_configs(&merged);
    if configs.is_empty() {
        tracing::debug!("No MCP server configs found");
        return;
    }

    // We need mutable access to the registry.  The registry is behind an Arc, so we
    // build a new one with MCP tools appended and swap it in.
    let mut new_registry = cc_tools::registry::ToolRegistry::with_defaults();

    for config in configs {
        if !config.enabled {
            tracing::debug!("MCP server '{}' is disabled, skipping", config.name);
            continue;
        }

        let server_name = config.name.clone();
        let client = Arc::new(StdioMcpClient::new(config));

        tracing::info!("Connecting to MCP server '{}'", server_name);
        if let Err(e) = client.connect().await {
            tracing::warn!("Failed to connect to MCP server '{}': {}", server_name, e);
            continue;
        }

        // List tools exposed by this server
        let tools = match client.list_tools().await {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!(
                    "Failed to list tools from MCP server '{}': {}",
                    server_name,
                    e
                );
                vec![]
            }
        };

        tracing::info!(
            "MCP server '{}' connected with {} tools",
            server_name,
            tools.len()
        );

        // Register the client globally so ListMcpResources/ReadMcpResource can find it
        cc_tools::register_mcp_client(server_name.clone(), client.clone());

        // Register each MCP tool as a McpDynamicTool in the registry
        for tool_def in tools {
            let mcp_tool = cc_tools::McpDynamicTool::new(&server_name, tool_def, client.clone());
            new_registry.register(std::sync::Arc::new(mcp_tool));
        }
    }

    engine.tools = Arc::new(new_registry);
}

/// Interactive TUI mode.
async fn run_interactive_mode(args: CliArgs, project_root: PathBuf) -> anyhow::Result<()> {
    let mut engine = build_engine(&args, &project_root, true).await?;

    // Connect MCP servers and register their tools into the engine's registry
    init_mcp(&mut engine, &project_root).await;
    let command_registry = cc_commands::registry::CommandRegistry::with_defaults();

    // Load skills (handled separately from CommandRegistry — used in TUI runner)
    let skills = load_skills(&project_root);

    // Resolve model display name
    let model_str = args
        .model
        .as_deref()
        .unwrap_or(cc_config::model_config::default_model());
    let model_display =
        cc_config::model_config::resolve_model_alias(model_str).unwrap_or(model_str);

    // Build session info for the TUI banner
    let git_branch = cc_utils::git::current_branch(&project_root).await.ok();
    let session_info = SessionInfo {
        model: model_display.to_string(),
        cwd: project_root.display().to_string(),
        git_branch,
        session_id: engine
            .session_id
            .clone()
            .unwrap_or_default(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };

    // Create TUI runner (enters alternate screen, enables raw mode)
    let mut runner = TuiRunner::new(engine, session_info, command_registry, skills)?;

    // Handle --resume
    if let Some(ref resume_id) = args.resume {
        let sessions_dir = cc_config::paths::sessions_dir();

        if resume_id.is_empty() {
            // --resume without ID: list recent sessions and exit
            match cc_session::storage::list_sessions(&sessions_dir) {
                Ok(sessions) if sessions.is_empty() => {
                    // Show as system message in TUI
                    runner.app.on_error("No saved sessions found.");
                }
                Ok(sessions) => {
                    // Show recent sessions (last 10)
                    let recent: Vec<&String> = sessions.iter().rev().take(10).collect();
                    let mut listing = String::from("Recent sessions:\n");
                    for sid in &recent {
                        // Try to get first user message as preview
                        let preview = cc_session::resume::load_resume_data(&sessions_dir, sid)
                            .ok()
                            .and_then(|d| {
                                d.messages.first().and_then(|m| {
                                    m.get("text")
                                        .and_then(|v| v.as_str())
                                        .map(|s| {
                                            if s.len() > 60 {
                                                format!("{}...", &s[..60])
                                            } else {
                                                s.to_string()
                                            }
                                        })
                                })
                            })
                            .unwrap_or_default();
                        listing.push_str(&format!("  {} — {}\n", sid, preview));
                    }
                    listing.push_str("\nUse: --resume <session-id>");
                    runner.app.add_user_message("/resume");
                    runner.app.on_text_delta(&listing);
                    runner.app.on_turn_complete("end_turn");
                }
                Err(e) => {
                    runner.app.on_error(&format!("Cannot list sessions: {}", e));
                }
            }
        } else {
            // --resume <ID>: load specific session
            match cc_session::resume::load_resume_data(&sessions_dir, resume_id) {
                Ok(data) => {
                    runner
                        .engine
                        .enable_persistence(sessions_dir, resume_id.clone());

                    for msg_data in &data.messages {
                        if let Some(text) = msg_data.get("text").and_then(|v| v.as_str()) {
                            let role = if runner.engine.messages.len() % 2 == 0 {
                                cc_api::types::Role::User
                            } else {
                                cc_api::types::Role::Assistant
                            };
                            runner.engine.messages.push(cc_api::types::ApiMessage {
                                role,
                                content: vec![cc_api::types::ContentBlock::Text {
                                    text: text.to_string(),
                                }],
                            });
                        }
                    }

                    // Show resumed messages in TUI
                    let msg_count = runner.engine.messages.len();
                    runner.app.add_user_message(&format!(
                        "Resumed session {} ({} messages)",
                        resume_id, msg_count
                    ));
                }
                Err(e) => {
                    runner.app.on_error(&format!(
                        "Cannot resume session '{}': {}",
                        resume_id, e
                    ));
                }
            }
        }
    }

    // If a prompt was given on the CLI, process it first
    if let Some(ref initial_prompt) = args.prompt {
        runner.submit_initial_prompt(initial_prompt).await?;
    }

    // Run the TUI event loop
    runner.run().await
}

/// Load all available skills from both user home and project directories.
fn load_skills(project_root: &Path) -> Vec<cc_skills::loader::SkillDefinition> {
    let mut skills = cc_skills::bundled::bundled_skills();

    // ~/.claude/skills/
    let home_skills = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".claude")
        .join("skills");
    if let Ok(s) = cc_skills::loader::load_skills_from_dir(&home_skills) {
        skills.extend(s);
    }

    // <project>/.claude/skills/
    let proj_skills = project_root.join(".claude").join("skills");
    if proj_skills != home_skills {
        if let Ok(s) = cc_skills::loader::load_skills_from_dir(&proj_skills) {
            skills.extend(s);
        }
    }

    skills
}

