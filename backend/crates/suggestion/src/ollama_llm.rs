use anyhow::Result;
use tokio::sync::broadcast;
use common::messages::WsEvent;
use crate::groq_llm::stream_openai_compat;

/// Stream suggestions from a local Ollama instance.
/// Ollama serves an OpenAI-compatible endpoint at {base_url}/v1/chat/completions.
/// No API key is required; an empty bearer token is sent and silently ignored.
pub async fn stream_suggestions(
    base_url: &str,
    model: &str,
    system_prompt: &str,
    user_prompt: &str,
    event_tx: broadcast::Sender<WsEvent>,
) -> Result<()> {
    let url = format!("{}/v1/chat/completions", base_url.trim_end_matches('/'));
    stream_openai_compat(
        "",   // no API key — Ollama ignores the Authorization header
        &url,
        model,
        "Ollama",
        system_prompt,
        user_prompt,
        event_tx,
    )
    .await
}
