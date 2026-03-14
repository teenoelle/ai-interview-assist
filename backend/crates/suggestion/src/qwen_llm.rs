use anyhow::Result;
use tokio::sync::broadcast;
use common::messages::WsEvent;
use crate::groq_llm::stream_openai_compat;

/// Qwen via Alibaba Cloud DashScope (OpenAI-compatible endpoint).
/// Free tier: generous daily limits on qwen-turbo and qwen-plus.
pub async fn stream_suggestions(
    api_key: &str,
    system_prompt: &str,
    user_prompt: &str,
    event_tx: broadcast::Sender<WsEvent>,
) -> Result<()> {
    stream_openai_compat(
        api_key,
        "https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions",
        "qwen-turbo",
        system_prompt,
        user_prompt,
        event_tx,
    )
    .await
}
