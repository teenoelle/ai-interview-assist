use anyhow::Result;
use tokio::sync::broadcast;
use common::messages::{WsEvent, SuggestionMode};
use crate::groq_llm::stream_openai_compat;

pub async fn stream_suggestions(
    api_key: &str,
    system_prompt: &str,
    user_prompt: &str,
    mode: SuggestionMode,
    event_tx: broadcast::Sender<WsEvent>,
) -> Result<()> {
    stream_openai_compat(
        api_key,
        "https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions",
        "qwen-turbo",
        "Qwen",
        system_prompt,
        user_prompt,
        mode,
        event_tx,
    )
    .await
}
