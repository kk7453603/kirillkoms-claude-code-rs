use crate::types::*;

pub static REWIND: CommandDef = CommandDef {
    name: "rewind",
    aliases: &["undo"],
    description: "Undo the last conversation turn",
    argument_hint: Some("[n]"),
    hidden: false,
    handler: |args| {
        let args = args.trim().to_string();
        Box::pin(async move {
            if args.is_empty() {
                Ok(CommandOutput::message(
                    "Rewound last conversation turn.\n\
                     The previous assistant response and your last message have been removed.",
                ))
            } else {
                match args.parse::<usize>() {
                    Ok(n) if n > 0 => Ok(CommandOutput::message(&format!(
                        "Rewound {} conversation turn{}.\n\
                         {} message pair{} removed from history.",
                        n,
                        if n == 1 { "" } else { "s" },
                        n,
                        if n == 1 { "" } else { "s" },
                    ))),
                    _ => Ok(CommandOutput::message(
                        "Invalid argument. Usage: /rewind [n]\n\
                         Provide a positive number of turns to rewind, or omit for 1.",
                    )),
                }
            }
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rewind_default() {
        let result = (REWIND.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Rewound last"));
    }

    #[tokio::test]
    async fn test_rewind_n() {
        let result = (REWIND.handler)("3").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("3 conversation turns"));
    }

    #[tokio::test]
    async fn test_rewind_invalid() {
        let result = (REWIND.handler)("abc").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Invalid"));
    }
}
