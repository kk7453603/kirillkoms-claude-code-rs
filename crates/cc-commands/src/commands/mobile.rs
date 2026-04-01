use crate::types::*;

pub static MOBILE: CommandDef = CommandDef {
    name: "mobile",
    aliases: &["ios", "android"],
    description: "Show QR code to download the Claude mobile app",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async move {
            Ok(CommandOutput::message(
                "Download the Claude mobile app:\n\n\
                 iOS:     https://apps.apple.com/app/claude/id1665286635\n\
                 Android: https://play.google.com/store/apps/details?id=com.anthropic.claude\n\n\
                 Scan the QR code or visit the links above to install.",
            ))
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mobile() {
        let result = (MOBILE.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("mobile app"));
    }
}
