use anyhow::Result;
use tokio::sync::broadcast;
use common::messages::WsEvent;
use crate::groq_llm::stream_openai_compat;

pub async fn stream_suggestions(
    api_key: &str,
    system_prompt: &str,
    user_prompt: &str,
    event_tx: broadcast::Sender<WsEvent>,
) -> Result<()> {
    stream_openai_compat(
        api_key,
        "https://api.cerebras.ai/v1/chat/completions",
        "llama3.1-8b",
        "Cerebras",
        system_prompt,
        user_prompt,
        event_tx,
    )
    .await
}
