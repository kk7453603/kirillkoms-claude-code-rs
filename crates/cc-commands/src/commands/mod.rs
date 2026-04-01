pub mod help;
pub mod clear;
pub mod compact;
pub mod config;
pub mod cost;
pub mod diff;
pub mod doctor;
pub mod exit;
pub mod model;
pub mod status;
pub mod version;
pub mod memory;
pub mod resume;
pub mod session;
pub mod theme;
pub mod permissions;
pub mod context;
pub mod commit;
pub mod review;
pub mod hooks;
pub mod mcp;

// Batch 2 commands
pub mod add_dir;
pub mod agents;
pub mod branch;
pub mod bug;
pub mod copy;
pub mod effort;
pub mod export;
pub mod fast;
pub mod feedback;
pub mod files;
pub mod ide;
pub mod init;
pub mod keybindings;
pub mod login;
pub mod logout;
pub mod plan;
pub mod plugin;
pub mod pr_comments;
pub mod release_notes;
pub mod rename;
pub mod share;
pub mod skills;
pub mod stats;
pub mod tasks;
pub mod teleport;
pub mod upgrade;
pub mod usage;
pub mod vim;
pub mod voice;

// Batch 3 - previously on disk but not registered
pub mod brief;
pub mod btw;
pub mod color;
pub mod env_cmd;
pub mod good_claude;
pub mod issue;
pub mod onboarding;
pub mod privacy_settings;
pub mod rewind;
pub mod sandbox_toggle;
pub mod stickers;
pub mod summary;
pub mod tag;
pub mod thinkback;

// Batch 4 - new command files
pub mod advisor;
pub mod ant_internal;
pub mod autofix_pr;
pub mod bridge_cmd;
pub mod bughunter;
pub mod chrome;
pub mod commit_push_pr;
pub mod ctx_viz;
pub mod desktop;
pub mod extra_usage;
pub mod init_verifiers;
pub mod insights;
pub mod install_cmd;
pub mod mobile;
pub mod oauth_refresh;
pub mod passes;
pub mod rate_limit_options;
pub mod reload_plugins;
pub mod remote;
pub mod security_review;
pub mod statusline;
pub mod terminal_setup;
pub mod ultraplan;

use crate::types::CommandDef;

pub fn all_commands() -> Vec<&'static CommandDef> {
    vec![
        // Original commands
        &help::HELP,
        &clear::CLEAR,
        &compact::COMPACT,
        &config::CONFIG,
        &cost::COST,
        &diff::DIFF,
        &doctor::DOCTOR,
        &exit::EXIT,
        &model::MODEL,
        &status::STATUS,
        &version::VERSION,
        &memory::MEMORY,
        &resume::RESUME,
        &session::SESSION,
        &theme::THEME,
        &permissions::PERMISSIONS,
        &context::CONTEXT,
        &commit::COMMIT,
        &review::REVIEW,
        &hooks::HOOKS,
        &mcp::MCP,
        // Batch 2
        &add_dir::ADD_DIR,
        &agents::AGENTS,
        &branch::BRANCH,
        &bug::BUG,
        &copy::COPY,
        &effort::EFFORT,
        &export::EXPORT,
        &fast::FAST,
        &feedback::FEEDBACK,
        &files::FILES,
        &ide::IDE,
        &init::INIT,
        &keybindings::KEYBINDINGS,
        &login::LOGIN,
        &logout::LOGOUT,
        &plan::PLAN,
        &plugin::PLUGIN,
        &pr_comments::PR_COMMENTS,
        &release_notes::RELEASE_NOTES,
        &rename::RENAME,
        &share::SHARE,
        &skills::SKILLS,
        &stats::STATS,
        &tasks::TASKS,
        &teleport::TELEPORT,
        &upgrade::UPGRADE,
        &usage::USAGE,
        &vim::VIM,
        &voice::VOICE,
        // Batch 3 - previously on disk
        &brief::BRIEF,
        &btw::BTW,
        &color::COLOR,
        &env_cmd::ENV_CMD,
        &good_claude::GOOD_CLAUDE,
        &issue::ISSUE,
        &onboarding::ONBOARDING,
        &privacy_settings::PRIVACY_SETTINGS,
        &rewind::REWIND,
        &sandbox_toggle::SANDBOX_TOGGLE,
        &stickers::STICKERS,
        &summary::SUMMARY,
        &tag::TAG,
        &thinkback::THINKBACK,
        &thinkback::THINKBACK_PLAY,
        // Batch 4 - new commands
        &advisor::ADVISOR,
        &ant_internal::ANT_TRACE,
        &ant_internal::BACKFILL_SESSIONS,
        &ant_internal::BREAK_CACHE,
        &ant_internal::DEBUG_TOOL_CALL,
        &ant_internal::HEAPDUMP,
        &ant_internal::MOCK_LIMITS,
        &ant_internal::RESET_LIMITS,
        &ant_internal::PERF_ISSUE,
        &autofix_pr::AUTOFIX_PR,
        &bridge_cmd::BRIDGE,
        &bridge_cmd::BRIDGE_KICK,
        &bughunter::BUGHUNTER,
        &chrome::CHROME,
        &commit_push_pr::COMMIT_PUSH_PR,
        &ctx_viz::CTX_VIZ,
        &desktop::DESKTOP,
        &extra_usage::EXTRA_USAGE,
        &init_verifiers::INIT_VERIFIERS,
        &insights::INSIGHTS,
        &install_cmd::INSTALL,
        &install_cmd::INSTALL_GITHUB_APP,
        &install_cmd::INSTALL_SLACK_APP,
        &mobile::MOBILE,
        &oauth_refresh::OAUTH_REFRESH,
        &passes::PASSES,
        &rate_limit_options::RATE_LIMIT_OPTIONS,
        &reload_plugins::RELOAD_PLUGINS,
        &remote::REMOTE_ENV,
        &remote::REMOTE_SETUP,
        &security_review::SECURITY_REVIEW,
        &statusline::STATUSLINE,
        &terminal_setup::TERMINAL_SETUP,
        &ultraplan::ULTRAPLAN,
    ]
}
