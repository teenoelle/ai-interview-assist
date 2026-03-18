pub mod detector;
pub mod gemini_llm;
pub mod groq_llm;
pub mod openrouter_llm;
pub mod mistral_llm;
pub mod cerebras_llm;
pub mod claude_llm;
pub mod qwen_llm;
pub mod ollama_llm;
pub mod prompt;

use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use common::messages::{TranscriptSegment, WsEvent};
use common::rate_limiter::RateLimiter;
use common::providers::{is_quota_exhausted, is_rate_limit};

macro_rules! try_provider {
    ($name:expr, $call:expr, $etx:expr, $next:expr) => {
        match $call.await {
            Ok(()) => return Ok(()),
            Err(e) if is_quota_exhausted(&e) || is_rate_limit(&e) => {
                tracing::warn!("{} unavailable (quota/rate-limit), trying next provider: {}", $name, e);
            }
            Err(e) => return Err(e),
        }
    };
}

async fn suggest_with_fallback(
    gemini_key: &str,
    anthropic_key: Option<&str>,
    groq_key: Option<&str>,
    groq_key_2: Option<&str>,
    openrouter_key: Option<&str>,
    mistral_key: Option<&str>,
    cerebras_key: Option<&str>,
    qwen_key: Option<&str>,
    ollama_url: &str,
    ollama_models: &[String],
    system_prompt: &str,
    user_prompt: &str,
    rate_limiter: &RateLimiter,
    event_tx: broadcast::Sender<WsEvent>,
) -> anyhow::Result<()> {
    // 1. Claude Haiku — primary, high quality
    if let Some(key) = anthropic_key {
        try_provider!("Claude",
            claude_llm::stream_suggestions(key, system_prompt, user_prompt, event_tx.clone()),
            event_tx, ());
    }

    // 2. Groq key 1 — fast fallback
    if let Some(key) = groq_key {
        try_provider!("Groq key 1",
            groq_llm::stream_suggestions(key, system_prompt, user_prompt, event_tx.clone()),
            event_tx, ());
    }

    // 3. Groq key 2 — higher-limit secondary key
    if let Some(key) = groq_key_2 {
        try_provider!("Groq key 2",
            groq_llm::stream_suggestions(key, system_prompt, user_prompt, event_tx.clone()),
            event_tx, ());
    }

    // 4. Cerebras — wafer-scale fast inference
    if let Some(key) = cerebras_key {
        try_provider!("Cerebras",
            cerebras_llm::stream_suggestions(key, system_prompt, user_prompt, event_tx.clone()),
            event_tx, ());
    }

    // 5. OpenRouter — no daily cap, just RPM throttled
    if let Some(key) = openrouter_key {
        try_provider!("OpenRouter",
            openrouter_llm::stream_suggestions(key, system_prompt, user_prompt, event_tx.clone()),
            event_tx, ());
    }

    // 6. Qwen (DashScope) — generous free tier for qwen-turbo
    if let Some(key) = qwen_key {
        try_provider!("Qwen",
            qwen_llm::stream_suggestions(key, system_prompt, user_prompt, event_tx.clone()),
            event_tx, ());
    }

    // 7. Mistral — free tier mistral-small
    if let Some(key) = mistral_key {
        try_provider!("Mistral",
            mistral_llm::stream_suggestions(key, system_prompt, user_prompt, event_tx.clone()),
            event_tx, ());
    }

    // 7. Ollama — try each configured model in order (fastest first)
    for model in ollama_models {
        match ollama_llm::stream_suggestions(ollama_url, model, system_prompt, user_prompt, event_tx.clone()).await {
            Ok(()) => return Ok(()),
            Err(e) => tracing::warn!("Ollama {} unavailable, trying next: {}", model, e),
        }
    }

    // 8. Gemini — absolute last resort, keep credits for sentiment analysis
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
    anthropic_key: Option<String>,
    groq_key: Option<String>,
    groq_key_2: Option<String>,
    openrouter_key: Option<String>,
    mistral_key: Option<String>,
    cerebras_key: Option<String>,
    qwen_key: Option<String>,
    ollama_url: String,
    ollama_models: Vec<String>,
    rate_limiter: RateLimiter,
) {
    loop {
        match question_rx.recv().await {
            Some(question) => {
                let gkey = gemini_key.clone();
                let akey = anthropic_key.clone();
                let grkey = groq_key.clone();
                let grkey2 = groq_key_2.clone();
                let orkey = openrouter_key.clone();
                let mkey = mistral_key.clone();
                let ckey = cerebras_key.clone();
                let qkey = qwen_key.clone();
                let ourl = ollama_url.clone();
                let omodels = ollama_models.clone();
                let etx = event_tx.clone();
                let sp = system_prompt.read().await.clone();
                let tr = transcript.read().await.clone();
                let rl = rate_limiter.clone();

                let user_prompt = prompt::build_user_prompt(&question, &tr);
                let _ = etx.send(WsEvent::QuestionDetected { question: question.clone() });

                tokio::spawn(async move {
                    match suggest_with_fallback(
                        &gkey,
                        akey.as_deref(),
                        grkey.as_deref(),
                        grkey2.as_deref(),
                        orkey.as_deref(),
                        mkey.as_deref(),
                        ckey.as_deref(),
                        qkey.as_deref(),
                        &ourl,
                        &omodels,
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
