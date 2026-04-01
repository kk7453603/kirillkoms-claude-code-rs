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

// New commands
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
        // New commands
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
    ]
}
