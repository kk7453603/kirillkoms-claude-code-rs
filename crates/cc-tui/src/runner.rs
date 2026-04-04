use std::io::{self, Stdout};
use std::sync::Arc;
use std::time::Duration;

use crossterm::event::{Event, EventStream, KeyCode, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use futures::StreamExt;
use ratatui::backend::CrosstermBackend;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders};
use ratatui::Terminal;

use cc_engine::query_engine::QueryEngine;
use cc_engine::query_loop::QueryEvent;

use crate::app::{App, AppAction, SessionInfo};
use crate::tui_permission::{PermissionChannel, PermissionRequest, summarize_input};
use crate::widgets;

/// Main TUI runner that drives the event loop.
pub struct TuiRunner {
    pub app: App,
    pub engine: QueryEngine,
    terminal: Terminal<CrosstermBackend<Stdout>>,
    permission_channel: Arc<PermissionChannel>,
    command_registry: cc_commands::registry::CommandRegistry,
    skills: Vec<cc_skills::loader::SkillDefinition>,
    plugins: Vec<cc_skills::plugin::LoadedPlugin>,
    event_stream: EventStream,
    /// Cached list of available models from the provider (fetched at startup).
    available_models: Vec<(String, String)>, // (id, label)
}

impl TuiRunner {
    pub fn new(
        mut engine: QueryEngine,
        session_info: SessionInfo,
        command_registry: cc_commands::registry::CommandRegistry,
        skills: Vec<cc_skills::loader::SkillDefinition>,
        plugins: Vec<cc_skills::plugin::LoadedPlugin>,
    ) -> anyhow::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        let permission_channel = Arc::new(PermissionChannel::new());
        let callback = permission_channel.sender();
        engine.set_permission_callback(Arc::new(callback));

        let mut app = App::new();
        app.session_info = session_info;

        Ok(Self {
            app,
            engine,
            terminal,
            permission_channel,
            command_registry,
            skills,
            plugins,
            event_stream: EventStream::new(),
            available_models: Vec::new(),
        })
    }

    /// Fetch available models from the provider (Ollama, OpenAI, etc.).
    /// Called once at startup. Non-blocking — returns empty on failure.
    pub async fn fetch_available_models(&mut self) {
        self.available_models = fetch_provider_models().await;
        // Always include Anthropic aliases
        let anthropic = vec![
            ("opus".into(), "opus — Claude Opus 4".into()),
            ("sonnet".into(), "sonnet — Claude Sonnet 4".into()),
            ("haiku".into(), "haiku — Claude Haiku 4.5".into()),
        ];
        for item in anthropic {
            if !self.available_models.iter().any(|(v, _)| *v == item.0) {
                self.available_models.push(item);
            }
        }
    }

    /// Get argument completions (value, label) based on current input command.
    fn arg_completions_for_input(&self) -> Vec<(String, String)> {
        let content = self.app.input.content();
        if content.starts_with("/resume ") {
            let sessions_dir = cc_config::paths::sessions_dir();
            let ids = cc_session::storage::list_sessions(&sessions_dir).unwrap_or_default();
            ids.into_iter()
                .rev()
                .take(10)
                .map(|sid| {
                    let preview = cc_session::resume::load_resume_data(&sessions_dir, &sid)
                        .ok()
                        .and_then(|d| {
                            d.messages.first().and_then(|m| {
                                m.get("text").and_then(|v| v.as_str()).map(|s| {
                                    if s.len() > 40 {
                                        format!("{}...", &s[..40])
                                    } else {
                                        s.to_string()
                                    }
                                })
                            })
                        })
                        .unwrap_or_default();
                    let label = if preview.is_empty() {
                        sid.clone()
                    } else {
                        format!("{} — {}", &sid[..8], preview)
                    };
                    (sid, label)
                })
                .collect()
        } else if content.starts_with("/model ") {
            // Use cached provider models
            let mut models = self.available_models.clone();
            // Ensure current model is at the top
            let current = &self.app.session_info.model;
            if !models.iter().any(|(v, _)| v == current) {
                models.insert(0, (current.clone(), format!("{} (current)", current)));
            }
            models
        } else {
            Vec::new()
        }
    }

    /// Command names for autocomplete: built-in commands + skills.
    fn command_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .command_registry
            .visible_commands()
            .iter()
            .map(|cmd| cmd.name.to_string())
            .collect();
        for skill in &self.skills {
            names.push(skill.name.clone());
        }
        names
    }

    /// Run the main event loop.
    pub async fn run(&mut self) -> anyhow::Result<()> {
        // Fetch available models from provider (non-blocking, cached)
        self.fetch_available_models().await;

        draw_frame(&mut self.terminal, &mut self.app)?;

        let mut tick_interval = tokio::time::interval(Duration::from_millis(50));
        tick_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            if self.app.should_quit {
                break;
            }

            tokio::select! {
                _ = tick_interval.tick() => {
                    self.app.tick();
                    if self.app.active_tool.is_some() || self.app.thinking {
                        draw_frame(&mut self.terminal, &mut self.app)?;
                    }
                }
                maybe_event = self.event_stream.next() => {
                    if let Some(Ok(Event::Key(key))) = maybe_event {
                        let action = self.app.handle_key_event(key);
                        let cmd_names = self.command_names();
                        let arg_completions = self.arg_completions_for_input();
                        self.app.update_completions(&cmd_names, &arg_completions);
                        match action {
                            AppAction::Quit => {
                                self.app.should_quit = true;
                            }
                            AppAction::Submit(text) => {
                                if text.starts_with('/') {
                                    let is_skill = handle_slash_command(
                                        &mut self.app,
                                        &mut self.engine,
                                        &self.command_registry,
                                        &self.skills,
                                        &self.plugins,
                                        &text,
                                    )
                                    .await;
                                    // If it was a skill, send the prompt to the engine
                                    if let Some(skill_prompt) = is_skill {
                                        self.app.add_user_message(&text);
                                        draw_frame(&mut self.terminal, &mut self.app)?;
                                        // Limit tool turns for skills to prevent runaway
                                        let saved_max_turns = self.engine.max_turns;
                                        self.engine.max_turns = 5;
                                        let mut tick = tokio::time::interval(
                                            std::time::Duration::from_millis(50),
                                        );
                                        tick.set_missed_tick_behavior(
                                            tokio::time::MissedTickBehavior::Skip,
                                        );
                                        run_query(
                                            &mut self.engine,
                                            &mut self.app,
                                            &mut self.terminal,
                                            &self.permission_channel,
                                            &mut self.event_stream,
                                            &skill_prompt,
                                            &mut tick,
                                        )
                                        .await?;
                                        self.engine.max_turns = saved_max_turns;
                                        maybe_auto_compact(&mut self.engine, &mut self.app);
                                    }
                                } else {
                                    self.app.add_user_message(&text);
                                    draw_frame(&mut self.terminal, &mut self.app)?;
                                    run_query(
                                        &mut self.engine,
                                        &mut self.app,
                                        &mut self.terminal,
                                        &self.permission_channel,
                                        &mut self.event_stream,
                                        &text,
                                        &mut tick_interval,
                                    )
                                    .await?;
                                    maybe_auto_compact(&mut self.engine, &mut self.app);
                                }
                            }
                            AppAction::PermissionResponse(_)
                            | AppAction::PermissionAlwaysAllow
                            | AppAction::Continue => {}
                        }
                        draw_frame(&mut self.terminal, &mut self.app)?;
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn submit_initial_prompt(&mut self, prompt: &str) -> anyhow::Result<()> {
        self.app.add_user_message(prompt);
        draw_frame(&mut self.terminal, &mut self.app)?;
        let mut tick_interval = tokio::time::interval(Duration::from_millis(50));
        tick_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        run_query(
            &mut self.engine,
            &mut self.app,
            &mut self.terminal,
            &self.permission_channel,
            &mut self.event_stream,
            prompt,
            &mut tick_interval,
        )
        .await?;
        draw_frame(&mut self.terminal, &mut self.app)?;
        Ok(())
    }
}

impl Drop for TuiRunner {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

// ─── Free functions ────────────────────────────────────────────────

/// Execute a query, stream results, persist to session transcript.
///
/// Rendering is batched: events update app state immediately, but
/// `draw_frame` only runs on tick intervals (every ~80ms) to avoid
/// jittery output during fast token streaming.
async fn run_query(
    engine: &mut QueryEngine,
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    permission_channel: &PermissionChannel,
    event_stream: &mut EventStream,
    prompt: &str,
    tick_interval: &mut tokio::time::Interval,
) -> anyhow::Result<()> {
    let mut full_response = String::new();
    let mut dirty = false; // batch redraws to tick

    {
        let mut stream = std::pin::pin!(engine.submit_streaming(prompt));

        loop {
            tokio::select! {
                // Render on tick — batches all pending state changes into one frame
                _ = tick_interval.tick() => {
                    app.tick();
                    if dirty {
                        draw_frame(terminal, app)?;
                        dirty = false;
                    }
                }
                maybe_event = stream.next() => {
                    match maybe_event {
                        Some(event) => {
                            let is_complete = matches!(event, QueryEvent::TurnComplete { .. });
                            let is_error = matches!(event, QueryEvent::Error(_));
                            if let QueryEvent::TextDelta(ref text) = event {
                                full_response.push_str(text);
                            }
                            handle_query_event(app, event);
                            dirty = true;
                            // Force immediate draw on structural events
                            if is_complete || is_error {
                                draw_frame(terminal, app)?;
                                break;
                            }
                        }
                        None => {
                            draw_frame(terminal, app)?;
                            break;
                        }
                    }
                }
                maybe_crossterm = event_stream.next() => {
                    if let Some(Ok(Event::Key(key))) = maybe_crossterm {
                        if key.modifiers.contains(KeyModifiers::CONTROL)
                            && key.code == KeyCode::Char('c')
                        {
                            app.should_quit = true;
                            draw_frame(terminal, app)?;
                            break;
                        }
                        match key.code {
                            KeyCode::Up => app.scroll.scroll_up(1),
                            KeyCode::Down => app.scroll.scroll_down(1),
                            KeyCode::Char(c) => app.input.insert_char(c),
                            KeyCode::Backspace => app.input.delete_char(),
                            _ => {}
                        }
                        // Render immediately on user input (no tick delay)
                        draw_frame(terminal, app)?;
                    }
                }
            }

            // Check permission requests
            if let Some(perm_req) = recv_permission(permission_channel) {
                let summary = summarize_input(&perm_req.tool_name, &perm_req.input);
                app.show_permission_prompt(&perm_req.tool_name, &perm_req.message, &summary);
                draw_frame(terminal, app)?;
                let response =
                    wait_for_permission(app, terminal, event_stream, tick_interval).await;
                let _ = perm_req.response_tx.send(response);
            }
        }
    }

    // Persist assistant response to session transcript + conversation history
    if !full_response.is_empty() {
        engine.messages.push(cc_api::types::ApiMessage {
            role: cc_api::types::Role::Assistant,
            content: vec![cc_api::types::ContentBlock::Text {
                text: full_response.clone(),
            }],
        });

        // Persist to session transcript for --resume support
        persist_entry(engine, "assistant_message", serde_json::json!({ "text": full_response }));
    }

    Ok(())
}

/// Persist a transcript entry if the engine has persistence enabled.
fn persist_entry(engine: &QueryEngine, entry_type: &str, data: serde_json::Value) {
    if let (Some(dir), Some(id)) = (&engine.sessions_dir, &engine.session_id) {
        let path = cc_session::storage::transcript_path(dir, id);
        let entry = cc_session::persistence::TranscriptEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            entry_type: entry_type.to_string(),
            data,
        };
        if let Err(e) = cc_session::persistence::append_entry(&path, &entry) {
            tracing::warn!("Failed to persist transcript entry: {}", e);
        }
    }
}

/// Wait for the user to press y/n in permission mode.
async fn wait_for_permission(
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    event_stream: &mut EventStream,
    tick_interval: &mut tokio::time::Interval,
) -> bool {
    loop {
        tokio::select! {
            _ = tick_interval.tick() => {
                app.tick();
                let _ = draw_frame(terminal, app);
            }
            maybe_event = event_stream.next() => {
                if let Some(Ok(Event::Key(key))) = maybe_event {
                    let action = app.handle_key_event(key);
                    let _ = draw_frame(terminal, app);
                    match action {
                        AppAction::PermissionResponse(allowed) => return allowed,
                        AppAction::PermissionAlwaysAllow => return true,
                        AppAction::Quit => {
                            app.should_quit = true;
                            return false;
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

/// Handle a /slash command. Returns Some(prompt) if a skill should be executed via engine.
async fn handle_slash_command(
    app: &mut App,
    engine: &mut QueryEngine,
    registry: &cc_commands::registry::CommandRegistry,
    skills: &[cc_skills::loader::SkillDefinition],
    plugins: &[cc_skills::plugin::LoadedPlugin],
    input: &str,
) -> Option<String> {
    let (cmd_name, cmd_args) = match input[1..].split_once(' ') {
        Some((name, rest)) => (name, rest.trim()),
        None => (&input[1..], ""),
    };

    // ── Intercept commands that need runtime state ──

    // /model — show or switch model at runtime
    if cmd_name == "model" {
        app.add_user_message(&format!("/model {}", cmd_args));
        if cmd_args.is_empty() {
            let current = &app.session_info.model;
            let mut lines = vec![format!("Current model: {}", current)];
            if let Some(cfg) = cc_config::model_config::get_model_config(current) {
                lines.push(format!("  Context window: {} tokens",
                    cc_cost::format::format_tokens(cfg.context_window as u64)));
                lines.push(format!("  Max output:     {} tokens",
                    cc_cost::format::format_tokens(cfg.max_output_tokens as u64)));
            } else {
                lines.push("  (custom model — no config data)".to_string());
            }
            lines.push(String::new());
            lines.push("Switch with: /model <name>".to_string());
            app.add_system_info(&lines.join("\n"));
        } else {
            // Runtime model switch
            let new_model = cc_config::model_config::resolve_model_alias(cmd_args)
                .map(|s| s.to_string())
                .unwrap_or_else(|| cmd_args.to_string());
            engine.model = new_model.clone();
            app.session_info.model = new_model.clone();
            app.add_system_info(&format!("Model switched to: {}", new_model));
        }
        return None;
    }

    // /clear — reset conversation
    if cmd_name == "clear" {
        engine.messages.clear();
        app.messages.clear();
        app.streaming_text.clear();
        app.streaming_thinking.clear();
        app.active_tool = None;
        app.thinking = false;
        app.usage = crate::app::UsageInfo::default();
        app.add_system_info("Conversation cleared.");
        return None;
    }

    // /compact — compress old messages to save context window
    if cmd_name == "compact" {
        if engine.messages.len() < 4 {
            app.add_user_message("/compact");
            app.add_system_info("Not enough messages to compact (need at least 4).");
            return None;
        }
        let msg_count = engine.messages.len();
        do_compact(engine, app);
        app.add_system_info(&format!(
            "Compacted {} messages → {} remain.",
            msg_count,
            engine.messages.len()
        ));
        return None;
    }

    // /rewind — undo last turn
    if cmd_name == "rewind" {
        let turns = cmd_args.parse::<usize>().unwrap_or(1);
        let remove = turns * 2; // each turn = user + assistant
        let _removed = remove.min(engine.messages.len());
        engine.messages.truncate(engine.messages.len().saturating_sub(remove));
        // Rebuild TUI messages from engine
        app.messages.clear();
        for msg in &engine.messages {
            let role = match msg.role {
                cc_api::types::Role::User => crate::app::MessageRole::User,
                cc_api::types::Role::Assistant => crate::app::MessageRole::Assistant,
            };
            let text = msg.content.iter().filter_map(|b| {
                if let cc_api::types::ContentBlock::Text { text } = b { Some(text.as_str()) } else { None }
            }).collect::<Vec<_>>().join("\n");
            app.messages.push(crate::app::ChatMessage {
                role,
                blocks: vec![crate::app::ContentBlock::Text(text)],
                timestamp: std::time::Instant::now(),
            });
        }
        app.add_system_info(&format!("Rewound {} turn(s). {} messages remain.", turns, engine.messages.len()));
        return None;
    }

    // /theme — switch TUI theme
    if cmd_name == "theme" {
        app.add_user_message(&format!("/theme {}", cmd_args));
        match cmd_args.to_lowercase().as_str() {
            "dark" => {
                app.theme = crate::themes::Theme::dark();
                app.add_system_info("Theme: dark");
            }
            "light" => {
                app.theme = crate::themes::Theme::light();
                app.add_system_info("Theme: light");
            }
            _ => {
                app.add_system_info(&format!("Current theme: {}\nAvailable: dark, light", app.theme.name));
            }
        }
        return None;
    }

    // /cost — show session cost details
    // /cost, /usage — token and cost breakdown
    if cmd_name == "cost" || cmd_name == "usage" {
        app.add_user_message(&format!("/{}", cmd_name));
        let u = &app.usage;
        let lines = vec![
            "Session Usage:".to_string(),
            format!("  Input tokens:  {}", cc_cost::format::format_tokens(u.input_tokens)),
            format!("  Output tokens: {}", cc_cost::format::format_tokens(u.output_tokens)),
            format!("  Total tokens:  {}", cc_cost::format::format_tokens(u.input_tokens + u.output_tokens)),
            format!("  Total cost:    {}", cc_cost::format::format_cost(u.cost_usd)),
            format!("  Turns:         {}", u.turn_count),
        ];
        app.add_system_info(&lines.join("\n"));
        return None;
    }

    // /status, /stats — system and session info
    if cmd_name == "status" || cmd_name == "stats" {
        app.add_user_message(&format!("/{}", cmd_name));
        let lines = vec![
            "Session Status:".to_string(),
            format!("  Model:       {}", app.session_info.model),
            format!("  Working dir: {}", app.session_info.cwd),
            format!("  Git branch:  {}", app.session_info.git_branch.as_deref().unwrap_or("(none)")),
            format!("  Session:     {}", app.session_info.session_id),
            format!("  Theme:       {}", app.theme.name),
            format!("  Messages:    {} (engine: {})", app.messages.len(), engine.messages.len()),
            format!("  Tools:       {}", engine.tools.tool_names().len()),
            format!("  Turns:       {}", app.usage.turn_count),
        ];
        app.add_system_info(&lines.join("\n"));
        return None;
    }

    // /resume — load session and display conversation
    if cmd_name == "resume" {
        let sessions_dir = cc_config::paths::sessions_dir();

        if cmd_args.is_empty() {
            // List recent sessions
            match cc_session::storage::list_sessions(&sessions_dir) {
                Ok(sessions) if sessions.is_empty() => {
                    app.add_user_message("/resume");
                    app.add_system_info("No saved sessions found.");
                }
                Ok(sessions) => {
                    let mut lines = vec!["Recent sessions:".to_string()];
                    for sid in sessions.iter().rev().take(10) {
                        let preview = cc_session::resume::load_resume_data(&sessions_dir, sid)
                            .ok()
                            .and_then(|d| {
                                d.messages.first().and_then(|m| {
                                    m.get("text").and_then(|v| v.as_str()).map(|s| {
                                        if s.len() > 50 { format!("{}...", &s[..50]) } else { s.to_string() }
                                    })
                                })
                            })
                            .unwrap_or_default();
                        lines.push(format!("  {} — {}", sid, preview));
                    }
                    lines.push(String::new());
                    lines.push("Resume with: /resume <session_id>".to_string());
                    app.add_user_message("/resume");
                    app.add_system_info(&lines.join("\n"));
                }
                Err(e) => {
                    app.on_error(&format!("Cannot list sessions: {}", e));
                }
            }
            return None;
        }

        // Load specific session
        match cc_session::resume::load_resume_data(&sessions_dir, cmd_args) {
            Ok(data) => {
                // Restore messages to engine using role from transcript
                engine.messages.clear();
                for msg_data in &data.messages {
                    if let Some(text) = msg_data.get("text").and_then(|v| v.as_str()) {
                        let role = match msg_data.get("_role").and_then(|v| v.as_str()) {
                            Some("assistant_message") => cc_api::types::Role::Assistant,
                            _ => cc_api::types::Role::User,
                        };
                        engine.messages.push(cc_api::types::ApiMessage {
                            role,
                            content: vec![cc_api::types::ContentBlock::Text {
                                text: text.to_string(),
                            }],
                        });
                    }
                }

                // Enable persistence for the resumed session
                engine.enable_persistence(sessions_dir, cmd_args.to_string());

                // Show conversation history in TUI
                app.messages.clear();
                for msg in &engine.messages {
                    let role_name = match msg.role {
                        cc_api::types::Role::User => crate::app::MessageRole::User,
                        cc_api::types::Role::Assistant => crate::app::MessageRole::Assistant,
                    };
                    let text = msg.content.iter().filter_map(|b| {
                        if let cc_api::types::ContentBlock::Text { text } = b { Some(text.as_str()) } else { None }
                    }).collect::<Vec<_>>().join("\n");

                    app.messages.push(crate::app::ChatMessage {
                        role: role_name,
                        blocks: vec![crate::app::ContentBlock::Text(text)],
                        timestamp: std::time::Instant::now(),
                    });
                }

                app.add_system_info(&format!(
                    "Resumed session {} ({} messages)",
                    cmd_args,
                    engine.messages.len()
                ));

                // Reset UI state for resumed session
                app.scroll.auto_follow = true;
                app.scroll.offset = 0;
                app.scroll.target = 0;
                app.mode = crate::app::AppMode::Input;
                app.command_completions.clear();
                app.completion_index = None;
            }
            Err(e) => {
                app.on_error(&format!("Cannot resume '{}': {}", cmd_args, e));
            }
        }
        return None;
    }

    // /skills — show all loaded skills
    if cmd_name == "skills" {
        let mut lines = vec!["Available Skills:".to_string(), String::new()];

        let bundled: Vec<_> = skills
            .iter()
            .filter(|s| s.source == cc_skills::loader::SkillSource::Bundled)
            .collect();
        let user_defined: Vec<_> = skills
            .iter()
            .filter(|s| s.source == cc_skills::loader::SkillSource::UserDefined)
            .collect();
        let plugin_skills: Vec<_> = skills
            .iter()
            .filter(|s| s.source == cc_skills::loader::SkillSource::Plugin)
            .collect();

        if !bundled.is_empty() {
            lines.push("Bundled:".to_string());
            for s in &bundled {
                lines.push(format!("  /{:<16} {}", s.name, s.description));
            }
        }
        if !user_defined.is_empty() {
            lines.push(String::new());
            lines.push("User/Project:".to_string());
            for s in &user_defined {
                lines.push(format!("  /{:<16} {}", s.name, s.description));
            }
        }
        if !plugin_skills.is_empty() {
            lines.push(String::new());
            lines.push("Plugin:".to_string());
            for s in &plugin_skills {
                lines.push(format!("  /{:<16} {}", s.name, s.description));
            }
        }

        lines.push(String::new());
        lines.push("Add custom skills as .md files in ~/.claude/skills/ or .claude/skills/".to_string());

        app.add_user_message("/skills");
        app.add_system_info(&lines.join("\n"));
        return None;
    }

    // /mcp — show connected MCP servers and available tools
    if cmd_name == "mcp" && (cmd_args.is_empty() || cmd_args == "list") {
        app.add_user_message(&format!("/mcp {}", cmd_args).trim_end().to_string());
        let servers = cc_tools::registered_mcp_servers();
        if servers.is_empty() {
            app.add_system_info(
                "No MCP servers connected.\n\n\
                 Add servers to ~/.claude/settings.json:\n\
                 {\n  \
                   \"mcpServers\": {\n    \
                     \"my-server\": {\n      \
                       \"command\": \"npx\",\n      \
                       \"args\": [\"-y\", \"@example/mcp-server\"]\n    \
                     }\n  \
                   }\n\
                 }",
            );
        } else {
            // Show connected servers and their available tools from the engine registry
            let mcp_tools: Vec<String> = engine
                .tools
                .tool_names()
                .into_iter()
                .filter(|n| cc_mcp::normalization::is_mcp_tool(n))
                .collect();

            let mut lines = vec![
                format!("Connected MCP Servers ({})", servers.len()),
                String::new(),
            ];
            for server in &servers {
                let server_tools: Vec<&String> = mcp_tools
                    .iter()
                    .filter(|t| t.starts_with(&format!("mcp__{server}__")))
                    .collect();
                lines.push(format!("  {} ({} tools)", server, server_tools.len()));
                for tool_name in &server_tools {
                    // Display the short tool name (strip "mcp__<server>__" prefix)
                    let short = tool_name
                        .strip_prefix(&format!("mcp__{server}__"))
                        .unwrap_or(tool_name.as_str());
                    lines.push(format!("    - {}", short));
                }
            }
            lines.push(String::new());
            lines.push(format!("Total MCP tools: {}", mcp_tools.len()));
            lines.push("Tools are callable as mcp__<server>__<tool>".to_string());
            app.add_system_info(&lines.join("\n"));
        }
        return None;
    }

    // /btw — side question: separate API call, doesn't affect main conversation
    if cmd_name == "btw" {
        if cmd_args.is_empty() {
            app.add_system_info("Usage: /btw <quick question>\nAsk a side question without interrupting the main conversation.");
            return None;
        }
        app.add_user_message(&format!("/btw {}", cmd_args));

        // Build a one-off request with conversation context but no side effects
        let side_prompt = format!(
            "The user has a quick side question (answer briefly, do not use tools):\n\n{}",
            cmd_args
        );
        let mut side_messages = engine.messages.clone();
        side_messages.push(cc_api::types::ApiMessage {
            role: cc_api::types::Role::User,
            content: vec![cc_api::types::ContentBlock::Text { text: side_prompt }],
        });

        let system = engine.system_context.to_system_blocks();
        let request = cc_api::types::MessagesRequest {
            model: engine.model.clone(),
            messages: side_messages,
            system,
            max_tokens: Some(1024),
            temperature: None,
            tools: None,
            tool_choice: None,
            thinking: None,
            stream: false,
            metadata: None,
        };

        app.thinking = true;
        app.spinner.set_message("Thinking...");

        match engine.api_client.send_messages(request).await {
            Ok(resp) => {
                let answer = resp.content.iter().find_map(|b| {
                    if let cc_api::types::ContentBlock::Text { text } = b {
                        Some(text.clone())
                    } else {
                        None
                    }
                }).unwrap_or_else(|| "(no response)".to_string());

                app.thinking = false;
                // Show answer in overlay — does NOT add to engine.messages
                app.btw_overlay = Some(answer);
            }
            Err(e) => {
                app.thinking = false;
                app.on_error(&format!("btw failed: {}", e));
            }
        }
        return None;
    }

    // /copy — copy last assistant response to clipboard (or show it)
    if cmd_name == "copy" || cmd_name == "cp" {
        let last_response = engine.messages.iter().rev().find_map(|m| {
            if matches!(m.role, cc_api::types::Role::Assistant) {
                m.content.iter().find_map(|b| {
                    if let cc_api::types::ContentBlock::Text { text } = b {
                        Some(text.clone())
                    } else {
                        None
                    }
                })
            } else {
                None
            }
        });
        match last_response {
            Some(text) => {
                let line_count = text.lines().count();
                let preview = if text.len() > 100 {
                    format!("{}...", &text[..100])
                } else {
                    text.clone()
                };
                app.add_user_message("/copy");
                app.add_system_info(&format!(
                    "Last response ({} lines):\n\n{}\n\n(Select text with your terminal mouse to copy)",
                    line_count, preview
                ));
            }
            None => {
                app.add_system_info("No assistant response to copy.");
            }
        }
        return None;
    }

    // /export — save conversation as markdown file
    if cmd_name == "export" {
        let filename = if cmd_args.is_empty() {
            format!("conversation-{}.md", chrono::Utc::now().format("%Y%m%d-%H%M%S"))
        } else {
            cmd_args.to_string()
        };

        let mut md = String::from("# Conversation Export\n\n");
        for msg in &engine.messages {
            let role = match msg.role {
                cc_api::types::Role::User => "**You**",
                cc_api::types::Role::Assistant => "**Assistant**",
            };
            md.push_str(&format!("## {}\n\n", role));
            for block in &msg.content {
                if let cc_api::types::ContentBlock::Text { text } = block {
                    md.push_str(text);
                    md.push_str("\n\n");
                }
            }
        }

        match std::fs::write(&filename, &md) {
            Ok(_) => {
                app.add_user_message("/export");
                app.add_system_info(&format!("Conversation exported to: {}", filename));
            }
            Err(e) => {
                app.on_error(&format!("Export failed: {}", e));
            }
        }
        return None;
    }

    // /plugin — list or inspect loaded plugins
    if cmd_name == "plugin" {
        app.add_user_message(&format!("/{} {}", cmd_name, cmd_args).trim_end().to_string());

        // Sub-command: "info <name>"
        if let Some(plugin_name) = cmd_args.strip_prefix("info").map(|s| s.trim()).filter(|s| !s.is_empty()) {
            match cc_skills::plugin::find_plugin(plugins, plugin_name) {
                Some(plugin) => {
                    let m = &plugin.manifest;
                    // Count plugin skills from the skills list
                    let skill_count = skills
                        .iter()
                        .filter(|s| s.source == cc_skills::loader::SkillSource::Plugin)
                        .count();
                    let mut lines = vec![
                        format!("Plugin: {}", m.name),
                        format!("  Version:     {}", m.version),
                        format!(
                            "  Description: {}",
                            m.description.as_deref().unwrap_or("(none)")
                        ),
                        format!("  Path:        {}", plugin.path.display()),
                        format!("  Enabled:     {}", plugin.enabled),
                        format!("  Skills:      {}", skill_count),
                    ];

                    if !m.commands.is_empty() {
                        lines.push(String::new());
                        lines.push("  Commands:".to_string());
                        for cmd in &m.commands {
                            lines.push(format!("    {} — {}", cmd.name, cmd.description));
                        }
                    }

                    if let Some(hooks) = &m.hooks {
                        if !hooks.is_empty() {
                            lines.push(String::new());
                            lines.push("  Hooks:".to_string());
                            for hook in hooks {
                                lines.push(format!("    [{}] {}", hook.event, hook.command));
                            }
                        }
                    }

                    app.add_system_info(&lines.join("\n"));
                }
                None => {
                    app.add_system_info(&format!(
                        "Plugin '{}' not found.\n\nUse /plugin list to see installed plugins.",
                        plugin_name
                    ));
                }
            }
            return None;
        }

        // Sub-command: "list" (or bare /plugin)
        if plugins.is_empty() {
            app.add_system_info(
                "No plugins installed.\n\n\
                 To install a plugin, create a directory in ~/.claude/plugins/ \
                 containing a plugin.json manifest:\n\n\
                 ~/.claude/plugins/\n\
                   my-plugin/\n\
                     plugin.json\n\
                     skills/\n\
                       my-skill.md",
            );
        } else {
            let mut lines = vec![
                format!("Installed Plugins ({})", plugins.len()),
                String::new(),
            ];
            for plugin in plugins {
                let m = &plugin.manifest;
                // Count skills contributed by this plugin
                let skill_count = skills
                    .iter()
                    .filter(|s| s.source == cc_skills::loader::SkillSource::Plugin)
                    .count();
                lines.push(format!(
                    "  {} v{}  ({} skills)",
                    m.name, m.version, skill_count
                ));
                if let Some(desc) = &m.description {
                    lines.push(format!("    {}", desc));
                }
            }
            lines.push(String::new());
            lines.push("Use /plugin info <name> for details.".to_string());
            app.add_system_info(&lines.join("\n"));
        }
        return None;
    }

    // /fast — toggle fast (haiku) mode
    if cmd_name == "fast" {
        if cmd_args == "on" || (cmd_args.is_empty() && engine.model != "claude-haiku-4-5-20251001") {
            app.add_user_message("/fast on");
            let prev = engine.model.clone();
            engine.model = "claude-haiku-4-5-20251001".to_string();
            app.session_info.model = engine.model.clone();
            app.add_system_info(&format!("Fast mode ON (switched from {} to haiku)", prev));
        } else {
            app.add_user_message("/fast off");
            app.add_system_info("Fast mode OFF. Use /model <name> to switch back.");
        }
        return None;
    }

    // /effort — set response effort level
    if cmd_name == "effort" {
        app.add_user_message(&format!("/effort {}", cmd_args));
        match cmd_args {
            "low" | "min" => app.add_system_info("Effort: low (brief responses, no extended thinking)"),
            "high" | "max" => app.add_system_info("Effort: high (detailed responses, extended thinking)"),
            _ => app.add_system_info("Effort: auto\nAvailable: low, high"),
        }
        return None;
    }

    // /plan — toggle plan mode (read-only tools)
    if cmd_name == "plan" {
        app.add_user_message(&format!("/plan {}", cmd_args));
        match cmd_args {
            "on" => app.add_system_info("Plan mode: ON (read-only tools only, no writes)"),
            "off" => app.add_system_info("Plan mode: OFF (all tools available)"),
            _ => app.add_system_info("Plan mode: off\nUsage: /plan on|off"),
        }
        return None;
    }

    // /vim — show vim-style keybinding help
    if cmd_name == "vim" {
        app.add_user_message("/vim");
        app.add_system_info("Vim-style keybindings are active in SCROLL mode:\n  j/k — scroll up/down\n  G — scroll to bottom\n  i — enter input mode\n  e — expand/collapse tool block\n  Esc — enter scroll mode");
        return None;
    }

    // /summary — summarize conversation via engine
    if cmd_name == "summary" {
        if engine.messages.is_empty() {
            app.add_system_info("No conversation to summarize.");
            return None;
        }
        app.add_user_message("/summary");
        return Some("Summarize our conversation so far in 3-5 bullet points. Be concise.".to_string());
    }

    // 1. Check built-in commands first
    if let Some(cmd_def) = registry.lookup(cmd_name) {
        match (cmd_def.handler)(cmd_args).await {
            Ok(output) => {
                if let Some(msg) = output.message {
                    app.add_user_message(&format!("/{}", cmd_name));
                    app.add_system_info(&msg);
                }
                if !output.should_continue {
                    app.should_quit = true;
                }
            }
            Err(e) => {
                app.on_error(&format!("Command error: {}", e));
            }
        }
        return None;
    }

    // 2. Check skills
    if let Some(skill) = skills.iter().find(|s| s.name == cmd_name) {
        let mut prompt = String::from(
            "IMPORTANT: Execute ONLY the task described below. \
             Do NOT create new files unless explicitly asked. \
             Do NOT modify files unless the task requires it. \
             Stay focused on the task.\n\n",
        );
        prompt.push_str(&skill.prompt_template);
        if !cmd_args.is_empty() {
            prompt.push_str("\n\nUser context: ");
            prompt.push_str(cmd_args);
        }
        return Some(prompt);
    }

    // 3. Unknown command
    app.on_error(&format!(
        "Unknown command: /{}. Type /help for available commands.",
        cmd_name
    ));
    None
}

/// Compact engine conversation history by summarizing old messages.
/// Assumes `engine.messages.len() >= 4` — caller must check.
fn do_compact(engine: &mut QueryEngine, app: &mut App) {
    let msg_count = engine.messages.len();
    // Keep last 2 messages (1 turn), summarize the rest
    let keep = 2;
    let old_msgs: Vec<String> = engine.messages[..msg_count - keep]
        .iter()
        .filter_map(|m| {
            m.content.iter().find_map(|b| {
                if let cc_api::types::ContentBlock::Text { text } = b {
                    Some(text.clone())
                } else {
                    None
                }
            })
        })
        .collect();

    let summary = format!(
        "[Compacted {} earlier messages into summary]\n\n{}",
        msg_count - keep,
        old_msgs.join("\n---\n")
    );
    // Truncate to summary length
    let summary = if summary.len() > 2000 {
        format!("{}...", &summary[..2000])
    } else {
        summary
    };

    // Replace old messages with a single summary message
    let kept = engine.messages.split_off(msg_count - keep);
    engine.messages.clear();
    engine.messages.push(cc_api::types::ApiMessage {
        role: cc_api::types::Role::User,
        content: vec![cc_api::types::ContentBlock::Text { text: summary }],
    });
    engine.messages.extend(kept);

    // Rebuild TUI messages from updated engine messages
    app.messages.clear();
    for msg in &engine.messages {
        let role = match msg.role {
            cc_api::types::Role::User => crate::app::MessageRole::User,
            cc_api::types::Role::Assistant => crate::app::MessageRole::Assistant,
        };
        let text = msg
            .content
            .iter()
            .filter_map(|b| {
                if let cc_api::types::ContentBlock::Text { text } = b {
                    Some(text.as_str())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        app.messages.push(crate::app::ChatMessage {
            role,
            blocks: vec![crate::app::ContentBlock::Text(text)],
            timestamp: std::time::Instant::now(),
        });
    }
}

/// Check token usage and auto-compact if over 80% of the context window.
fn maybe_auto_compact(engine: &mut QueryEngine, app: &mut App) {
    let total_tokens = app.usage.input_tokens + app.usage.output_tokens;
    let threshold = cc_config::model_config::get_model_config(&engine.model)
        .map(|c| (c.context_window as u64) * 80 / 100)
        .unwrap_or(160_000);
    if total_tokens > threshold && engine.messages.len() >= 4 {
        do_compact(engine, app);
        app.add_system_info(&format!(
            "Auto-compacted: context was at {}% of limit.",
            total_tokens * 100 / threshold
        ));
    }
}

/// Map a QueryEvent to App state mutations.
fn handle_query_event(app: &mut App, event: QueryEvent) {
    match event {
        QueryEvent::TextDelta(text) => app.on_text_delta(&text),
        QueryEvent::ThinkingDelta(text) => app.on_thinking_delta(&text),
        QueryEvent::ToolUseStart { id, name, input } => {
            app.on_tool_use_start(&id, &name, &input)
        }
        QueryEvent::ToolResult {
            id,
            result,
            is_error,
        } => app.on_tool_result(&id, &result, is_error),
        QueryEvent::TurnComplete { stop_reason } => app.on_turn_complete(&stop_reason),
        QueryEvent::Error(e) => app.on_error(&e),
        QueryEvent::UsageUpdate {
            input_tokens,
            output_tokens,
        } => app.on_usage_update(input_tokens, output_tokens),
    }
}

/// Draw the full UI frame.
fn draw_frame(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
) -> anyhow::Result<()> {
    terminal.draw(|frame| {
        let area = frame.area();
        let input_height = (app.input.line_count() as u16 + 2).clamp(3, 8);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Min(5),
                Constraint::Length(input_height),
                Constraint::Length(1),
            ])
            .split(area);

        widgets::banner::render_banner(frame, chunks[0], &app.session_info, &app.theme);

        let messages_block = Block::default()
            .borders(Borders::TOP | Borders::BOTTOM)
            .border_style(Style::default().fg(app.theme.border_color));
        let messages_inner = messages_block.inner(chunks[1]);
        frame.render_widget(messages_block, chunks[1]);

        widgets::messages::render_messages(
            frame,
            messages_inner,
            &mut widgets::messages::MessagesRenderState {
                messages: &app.messages,
                streaming_text: &app.streaming_text,
                streaming_thinking: &app.streaming_thinking,
                active_tool: app.active_tool.as_ref(),
                thinking: app.thinking,
                spinner: &app.spinner,
                scroll: &mut app.scroll,
                theme: &app.theme,
            },
        );

        widgets::input_area::render_input_area(
            frame,
            chunks[2],
            &app.input,
            app.mode,
            &app.theme,
            &app.command_completions,
            &app.completion_labels,
            app.completion_index,
        );

        widgets::status_bar::render_status_bar(
            frame,
            chunks[3],
            &app.session_info.model,
            &app.usage,
            app.mode,
            &app.theme,
        );

        if let Some(ref perm) = app.pending_permission {
            widgets::permission_overlay::render_permission_overlay(
                frame, area, perm, &app.theme,
            );
        }

        if let Some(ref text) = app.btw_overlay {
            widgets::btw_overlay::render_btw_overlay(frame, area, text, &app.theme);
        }
    })?;

    Ok(())
}

/// Try to receive a permission request.
fn recv_permission(channel: &PermissionChannel) -> Option<PermissionRequest> {
    let mut rx = channel.request_rx.lock().unwrap();
    rx.try_recv().ok()
}

/// Fetch available models from the OpenAI-compatible provider.
/// Tries Ollama `/api/tags` first, then OpenAI `/v1/models`.
async fn fetch_provider_models() -> Vec<(String, String)> {
    let base_url = match std::env::var("OPENAI_BASE_URL") {
        Ok(url) => url,
        Err(_) => return Vec::new(),
    };

    // Try Ollama format: /api/tags
    if let Ok(models) = fetch_ollama_models(&base_url).await {
        if !models.is_empty() {
            return models;
        }
    }

    // Try OpenAI format: /v1/models
    if let Ok(models) = fetch_openai_models(&base_url).await {
        return models;
    }

    Vec::new()
}

async fn fetch_ollama_models(base_url: &str) -> Result<Vec<(String, String)>, ()> {
    let url = format!("{}/api/tags", base_url.trim_end_matches('/'));
    let resp = reqwest::Client::new()
        .get(&url)
        .timeout(std::time::Duration::from_secs(3))
        .send()
        .await
        .map_err(|_| ())?;

    let json: serde_json::Value = resp.json().await.map_err(|_| ())?;
    let models = json
        .get("models")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|m| {
                    let name = m.get("name")?.as_str()?;
                    let size = m
                        .get("details")
                        .and_then(|d| d.get("parameter_size"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let label = if size.is_empty() {
                        name.to_string()
                    } else {
                        format!("{} ({})", name, size)
                    };
                    Some((name.to_string(), label))
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(models)
}

async fn fetch_openai_models(base_url: &str) -> Result<Vec<(String, String)>, ()> {
    let url = format!("{}/v1/models", base_url.trim_end_matches('/'));
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    let resp = reqwest::Client::new()
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .timeout(std::time::Duration::from_secs(3))
        .send()
        .await
        .map_err(|_| ())?;

    let json: serde_json::Value = resp.json().await.map_err(|_| ())?;
    let models = json
        .get("data")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|m| {
                    let id = m.get("id")?.as_str()?;
                    Some((id.to_string(), id.to_string()))
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(models)
}
