use anyhow::Result;
use tokio::sync::broadcast;
use common::messages::{WsEvent, SuggestionMode};
use crate::groq_llm::stream_openai_compat;

const FREE_MODELS: &[&str] = &[
    "meta-llama/llama-3.3-70b-instruct:free",
    "deepseek/deepseek-r1:free",
    "qwen/qwen-2.5-72b-instruct:free",
    "mistralai/mistral-7b-instruct:free",
    "google/gemma-3-27b-it:free",
];

pub async fn stream_suggestions(
    api_key: &str,
    system_prompt: &str,
    user_prompt: &str,
    mode: SuggestionMode,
    event_tx: broadcast::Sender<WsEvent>,
) -> Result<()> {
    let mut last_err = anyhow::anyhow!("No OpenRouter free models available");
    for model in FREE_MODELS {
        match stream_openai_compat(
            api_key,
            "https://openrouter.ai/api/v1/chat/completions",
            model,
            "OpenRouter",
            system_prompt,
            user_prompt,
            mode,
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
