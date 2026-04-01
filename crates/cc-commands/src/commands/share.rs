use crate::types::*;

pub static SHARE: CommandDef = CommandDef {
    name: "share",
    aliases: &[],
    description: "Share session transcript",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async {
            Ok(CommandOutput::message(
                "Session sharing is not yet available in this version.\n\n\
                 You can export the session instead:\n  \
                 /export markdown session.md\n  \
                 /export json session.json",
            ))
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_share() {
        let result = (SHARE.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("sharing"));
    }
}
