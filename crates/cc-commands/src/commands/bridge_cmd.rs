use crate::types::*;

pub static BRIDGE: CommandDef = CommandDef {
    name: "bridge",
    aliases: &["remote-control"],
    description: "Manage bidirectional bridge connection",
    argument_hint: Some("[status|connect|disconnect]"),
    hidden: false,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            match args.as_str() {
                "status" => Ok(CommandOutput::message(
                    "Bridge status: not connected\n\
                     No active bridge session.",
                )),
                "connect" => Ok(CommandOutput::message(
                    "Attempting to establish bridge connection...\n\
                     Bridge connection is not available in the current environment.",
                )),
                "disconnect" => Ok(CommandOutput::message("Bridge disconnected.")),
                "" => Ok(CommandOutput::message(
                    "Bridge connection manager.\n\n\
                     Usage: /bridge [status|connect|disconnect]\n\n\
                     Manages the bidirectional bridge connection for remote control.",
                )),
                _ => Ok(CommandOutput::message(
                    "Unknown bridge command. Usage: /bridge [status|connect|disconnect]",
                )),
            }
        })
    },
};

pub static BRIDGE_KICK: CommandDef = CommandDef {
    name: "bridge-kick",
    aliases: &[],
    description: "Inject bridge failure states for testing (internal)",
    argument_hint: Some("<close|poll> <code>"),
    hidden: true,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            if args.is_empty() {
                Ok(CommandOutput::message(
                    "Internal command: bridge-kick injects failure states for bridge testing.\n\n\
                     Usage:\n  \
                     /bridge-kick close <code>  - Simulate WebSocket close with code\n  \
                     /bridge-kick poll <code>   - Simulate poll error with HTTP status",
                ))
            } else {
                Ok(CommandOutput::message(&format!(
                    "Bridge kick simulated: {}\n\
                     (Internal testing command - no actual bridge affected)",
                    args
                )))
            }
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bridge_status() {
        let result = (BRIDGE.handler)("status").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("not connected"));
    }

    #[tokio::test]
    async fn test_bridge_kick() {
        let result = (BRIDGE_KICK.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("bridge-kick"));
    }
}
