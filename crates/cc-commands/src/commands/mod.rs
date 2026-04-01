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

use crate::types::CommandDef;

pub fn all_commands() -> Vec<&'static CommandDef> {
    vec![
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
    ]
}
