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
        "https://api.deepseek.com/v1/chat/completions",
        "deepseek-chat",
        "DeepSeek",
        system_prompt,
        user_prompt,
        mode,
        event_tx,
    )
    .await
}
