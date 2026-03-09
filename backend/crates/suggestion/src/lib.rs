pub mod detector;
pub mod gemini_llm;
pub mod groq_llm;
pub mod openrouter_llm;
pub mod prompt;

use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use common::messages::{TranscriptSegment, WsEvent};
use common::rate_limiter::RateLimiter;
use common::providers::is_quota_exhausted;

async fn suggest_with_fallback(
    gemini_key: &str,
    groq_key: Option<&str>,
    openrouter_key: Option<&str>,
    system_prompt: &str,
    user_prompt: &str,
    rate_limiter: &RateLimiter,
    event_tx: broadcast::Sender<WsEvent>,
) -> anyhow::Result<()> {
    // 1. Gemini (acquire rate-limit token first)
    rate_limiter.acquire().await;
    match gemini_llm::stream_suggestions(gemini_key, system_prompt, user_prompt, event_tx.clone()).await {
        Ok(()) => return Ok(()),
        Err(e) if is_quota_exhausted(&e) => {
            tracing::warn!("Gemini suggestions quota exhausted, trying Groq");
        }
        Err(e) => return Err(e),
    }

    // 2. Groq Llama 3.3 70B
    if let Some(key) = groq_key {
        match groq_llm::stream_suggestions(key, system_prompt, user_prompt, event_tx.clone()).await {
            Ok(()) => return Ok(()),
            Err(e) if is_quota_exhausted(&e) => {
                tracing::warn!("Groq suggestions quota exhausted, trying OpenRouter");
            }
            Err(e) => return Err(e),
        }
    }

    // 3. OpenRouter free models
    if let Some(key) = openrouter_key {
        return openrouter_llm::stream_suggestions(key, system_prompt, user_prompt, event_tx).await;
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
    rate_limiter: RateLimiter,
) {
    loop {
        match question_rx.recv().await {
            Some(question) => {
                let gkey = gemini_key.clone();
                let grkey = groq_key.clone();
                let orkey = openrouter_key.clone();
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
