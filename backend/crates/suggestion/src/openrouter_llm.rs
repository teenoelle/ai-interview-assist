use anyhow::Result;
use tokio::sync::broadcast;
use common::messages::WsEvent;
use crate::groq_llm::stream_openai_compat;

// Free models on OpenRouter (no credits needed, just a free account)
const FREE_MODELS: &[&str] = &[
    "meta-llama/llama-3.3-70b-instruct:free",
    "mistralai/mistral-7b-instruct:free",
    "google/gemini-2.0-flash-exp:free",
];

pub async fn stream_suggestions(
    api_key: &str,
    system_prompt: &str,
    user_prompt: &str,
    event_tx: broadcast::Sender<WsEvent>,
) -> Result<()> {
    // Try each free model in order
    let mut last_err = anyhow::anyhow!("No OpenRouter free models available");
    for model in FREE_MODELS {
        match stream_openai_compat(
            api_key,
            "https://openrouter.ai/api/v1/chat/completions",
            model,
            system_prompt,
            user_prompt,
            event_tx.clone(),
        )
        .await
        {
            Ok(()) => return Ok(()),
            Err(e) => {
                tracing::warn!("OpenRouter model {} failed: {}", model, e);
                last_err = e;
            }
        }
    }
    Err(last_err)
}
