use anyhow::Result;
use tokio::sync::broadcast;
use common::messages::{WsEvent, SuggestionMode};
use crate::groq_llm::stream_openai_compat;

const FREE_MODELS: &[&str] = &[
    "google/gemini-2.0-flash-exp:free",   // best format compliance on free tier
    "google/gemini-flash-1.5:free",
    "deepseek/deepseek-chat:free",
    "meta-llama/llama-4-maverick:free",
    "meta-llama/llama-3.3-70b-instruct:free",
    "google/gemma-3-27b-it:free",
];

/// `custom_model`: if Some, use that specific model instead of the free-tier rotation list.
pub async fn stream_suggestions(
    api_key: &str,
    custom_model: Option<&str>,
    system_prompt: &str,
    user_prompt: &str,
    mode: SuggestionMode,
    event_tx: broadcast::Sender<WsEvent>,
) -> Result<()> {
    // If the user pinned a specific model, use only that one.
    let custom_arr;
    let models: &[&str] = if let Some(m) = custom_model {
        custom_arr = [m];
        &custom_arr
    } else {
        FREE_MODELS
    };

    let mut last_err = anyhow::anyhow!("No OpenRouter models available");
    for model in models {
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
