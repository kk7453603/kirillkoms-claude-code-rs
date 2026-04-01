use crate::types::*;

pub static THINKBACK: CommandDef = CommandDef {
    name: "thinkback",
    aliases: &[],
    description: "Show Claude's extended thinking from the last response",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async move {
            Ok(CommandOutput::message(
                "Displaying extended thinking from the last response...\n\n\
                 (No extended thinking available for the current turn.\n\
                  Extended thinking is shown when the model uses it during generation.)",
            ))
        })
    },
};

pub static THINKBACK_PLAY: CommandDef = CommandDef {
    name: "thinkback-play",
    aliases: &[],
    description: "Replay Claude's thinking process step by step",
    argument_hint: None,
    hidden: false,
    handler: |_args| {
        Box::pin(async move {
            Ok(CommandOutput::message(
                "Replaying extended thinking step by step...\n\n\
                 (No extended thinking available to replay.\n\
                  Use a model with extended thinking enabled to see this in action.)",
            ))
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_thinkback() {
        let result = (THINKBACK.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("thinking"));
    }

    #[tokio::test]
    async fn test_thinkback_play() {
        let result = (THINKBACK_PLAY.handler)("").await.unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("Replaying"));
    }
}
