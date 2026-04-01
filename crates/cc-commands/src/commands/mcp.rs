use crate::types::*;

pub static MCP: CommandDef = CommandDef {
    name: "mcp",
    aliases: &[],
    description: "Manage MCP servers",
    argument_hint: Some("[list|add|remove]"),
    hidden: true,
    handler: |args| {
        let msg = if args.is_empty() {
            "MCP servers: (none configured)".to_string()
        } else {
            format!("MCP command: {args}")
        };
        Box::pin(async move { Ok(CommandOutput::message(&msg)) })
    },
};
