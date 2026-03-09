pub mod detector;
pub mod gemini_llm;
pub mod groq_llm;
pub mod openrouter_llm;
pub mod mistral_llm;
pub mod cerebras_llm;
pub mod prompt;

use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use common::messages::{TranscriptSegment, WsEvent};
use common::rate_limiter::RateLimiter;
use common::providers::is_quota_exhausted;

macro_rules! try_provider {
    ($name:expr, $call:expr, $etx:expr, $next:expr) => {
        match $call.await {
            Ok(()) => return Ok(()),
            Err(e) if is_quota_exhausted(&e) => {
                tracing::warn!("{} suggestions quota exhausted, trying next provider", $name);
            }
            Err(e) => return Err(e),
        }
    };
}

async fn suggest_with_fallback(
    gemini_key: &str,
    groq_key: Option<&str>,
    openrouter_key: Option<&str>,
    mistral_key: Option<&str>,
    cerebras_key: Option<&str>,
    system_prompt: &str,
    user_prompt: &str,
    rate_limiter: &RateLimiter,
    event_tx: broadcast::Sender<WsEvent>,
) -> anyhow::Result<()> {
    // Priority order designed to preserve Gemini credits for sentiment (vision-only):
    // 1. OpenRouter — no daily cap, just RPM throttled; best for high-volume use
    if let Some(key) = openrouter_key {
        try_provider!("OpenRouter",
            openrouter_llm::stream_suggestions(key, system_prompt, user_prompt, event_tx.clone()),
            event_tx, ());
    }

    // 2. Cerebras — very fast free Llama 3.3 70B, generous free tier
    if let Some(key) = cerebras_key {
        try_provider!("Cerebras",
            cerebras_llm::stream_suggestions(key, system_prompt, user_prompt, event_tx.clone()),
            event_tx, ());
    }

    // 3. Mistral — free tier mistral-small, reliable fallback
    if let Some(key) = mistral_key {
        try_provider!("Mistral",
            mistral_llm::stream_suggestions(key, system_prompt, user_prompt, event_tx.clone()),
            event_tx, ());
    }

    // 4. Groq — 1,000 req/day LLM limit; saved here since Whisper uses a separate quota
    if let Some(key) = groq_key {
        try_provider!("Groq",
            groq_llm::stream_suggestions(key, system_prompt, user_prompt, event_tx.clone()),
            event_tx, ());
    }

    // 5. Gemini — absolute last resort, keep credits for sentiment analysis
    rate_limiter.acquire().await;
    match gemini_llm::stream_suggestions(gemini_key, system_prompt, user_prompt, event_tx).await {
        Ok(()) => return Ok(()),
        Err(e) if is_quota_exhausted(&e) => tracing::warn!("Gemini suggestions quota exhausted"),
        Err(e) => return Err(e),
    }

    anyhow::bail!("All suggestion providers exhausted")
}

pub async fn run_agent(
    mut question_rx: mpsc::Receiver<String>,
    event_tx: broadcast::Sender<WsEvent>,
    system_prompt: Arc<RwLock<String>>,
    transcript: Arc<RwLock<Vec<TranscriptSegment>>>,
    gemini_key: String,
    groq_key: Option<String>,
    openrouter_key: Option<String>,
    mistral_key: Option<String>,
    cerebras_key: Option<String>,
    rate_limiter: RateLimiter,
) {
    loop {
        match question_rx.recv().await {
            Some(question) => {
                let gkey = gemini_key.clone();
                let grkey = groq_key.clone();
                let orkey = openrouter_key.clone();
                let mkey = mistral_key.clone();
                let ckey = cerebras_key.clone();
                let etx = event_tx.clone();
                let sp = system_prompt.read().await.clone();
                let tr = transcript.read().await.clone();
                let rl = rate_limiter.clone();

                let user_prompt = prompt::build_user_prompt(&question, &tr);
                let _ = etx.send(WsEvent::QuestionDetected { question: question.clone() });

                tokio::spawn(async move {
                    match suggest_with_fallback(
                        &gkey,
                        grkey.as_deref(),
                        orkey.as_deref(),
                        mkey.as_deref(),
                        ckey.as_deref(),
                        &sp,
                        &user_prompt,
                        &rl,
                        etx.clone(),
                    )
                    .await
                    {
                        Ok(()) => {}
                        Err(e) => {
                            tracing::error!("Suggestion error: {}", e);
                            let _ = etx.send(WsEvent::Error {
                                message: format!("Suggestion error: {}", e),
                            });
                        }
                    }
                });
            }
            None => break,
        }
    }
}
