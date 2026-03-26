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
use common::messages::{TranscriptSegment, WsEvent, SuggestionMode};
use common::rate_limiter::RateLimiter;
use common::providers::{is_quota_exhausted, is_rate_limit};

pub type CallCounts = Arc<std::sync::Mutex<std::collections::HashMap<String, u64>>>;

fn inc(counts: &Option<CallCounts>, name: &str) {
    if let Some(map) = counts {
        if let Ok(mut m) = map.lock() {
            *m.entry(name.to_string()).or_insert(0) += 1;
        }
    }
}

fn provider_is_local(name: &str) -> bool {
    name.starts_with("Ollama") || name == "Whisper (local)"
}

macro_rules! try_provider {
    ($name:expr, $call:expr, $counts:expr, $etx:expr) => {
        match $call.await {
            Ok(()) => {
                inc($counts, $name);
                let _ = $etx.send(WsEvent::ProviderUsed {
                    service: "suggestions".to_string(),
                    provider: $name.to_string(),
                    local: provider_is_local($name),
                });
                return Ok(());
            }
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
    mode: SuggestionMode,
    rate_limiter: &RateLimiter,
    event_tx: broadcast::Sender<WsEvent>,
    call_counts: &Option<CallCounts>,
) -> anyhow::Result<()> {
    // 1. Claude Haiku — primary, high quality
    if let Some(key) = anthropic_key {
        try_provider!("Claude",
            claude_llm::stream_suggestions(key, system_prompt, user_prompt, mode, event_tx.clone()),
            call_counts, event_tx);
    }

    // 2. Groq key 1 — fast fallback
    if let Some(key) = groq_key {
        try_provider!("Groq",
            groq_llm::stream_suggestions(key, system_prompt, user_prompt, mode, event_tx.clone()),
            call_counts, event_tx);
    }

    // 3. Groq key 2 — higher-limit secondary key
    if let Some(key) = groq_key_2 {
        try_provider!("Groq #2",
            groq_llm::stream_suggestions(key, system_prompt, user_prompt, mode, event_tx.clone()),
            call_counts, event_tx);
    }

    // 4. Cerebras — wafer-scale fast inference
    if let Some(key) = cerebras_key {
        try_provider!("Cerebras",
            cerebras_llm::stream_suggestions(key, system_prompt, user_prompt, mode, event_tx.clone()),
            call_counts, event_tx);
    }

    // 5. OpenRouter — no daily cap, just RPM throttled
    if let Some(key) = openrouter_key {
        try_provider!("OpenRouter",
            openrouter_llm::stream_suggestions(key, system_prompt, user_prompt, mode, event_tx.clone()),
            call_counts, event_tx);
    }

    // 6. Qwen (DashScope) — generous free tier for qwen-turbo
    if let Some(key) = qwen_key {
        try_provider!("Qwen",
            qwen_llm::stream_suggestions(key, system_prompt, user_prompt, mode, event_tx.clone()),
            call_counts, event_tx);
    }

    // 7. Mistral — free tier mistral-small
    if let Some(key) = mistral_key {
        try_provider!("Mistral",
            mistral_llm::stream_suggestions(key, system_prompt, user_prompt, mode, event_tx.clone()),
            call_counts, event_tx);
    }

    // 8. Ollama — try each configured model in order (fastest first)
    for model in ollama_models {
        match ollama_llm::stream_suggestions(ollama_url, model, system_prompt, user_prompt, mode, event_tx.clone()).await {
            Ok(()) => {
                let name = format!("Ollama ({})", model);
                inc(call_counts, &name);
                let _ = event_tx.send(WsEvent::ProviderUsed {
                    service: "suggestions".to_string(),
                    provider: name,
                    local: true,
                });
                return Ok(());
            }
            Err(e) => tracing::warn!("Ollama {} unavailable, trying next: {}", model, e),
        }
    }

    // 9. Gemini — absolute last resort, keep credits for sentiment analysis
    rate_limiter.acquire().await;
    match gemini_llm::stream_suggestions(gemini_key, system_prompt, user_prompt, mode, event_tx.clone()).await {
        Ok(()) => {
            inc(call_counts, "Gemini");
            let _ = event_tx.send(WsEvent::ProviderUsed {
                service: "suggestions".to_string(),
                provider: "Gemini".to_string(),
                local: false,
            });
            return Ok(());
        }
        Err(e) if is_quota_exhausted(&e) => tracing::warn!("Gemini suggestions quota exhausted"),
        Err(e) => return Err(e),
    }

    anyhow::bail!("All suggestion providers exhausted")
}

// Helper to run suggest_with_fallback with all provider keys cloned from run_agent scope
macro_rules! run_suggest {
    ($mode:expr, $prompt:expr, $gkey:expr, $akey:expr, $grkey:expr, $grkey2:expr,
     $orkey:expr, $mkey:expr, $ckey:expr, $qkey:expr,
     $ourl:expr, $omodels:expr, $sp:expr, $rl:expr, $etx:expr, $cc:expr) => {
        suggest_with_fallback(
            &$gkey, $akey.as_deref(), $grkey.as_deref(), $grkey2.as_deref(),
            $orkey.as_deref(), $mkey.as_deref(), $ckey.as_deref(), $qkey.as_deref(),
            &$ourl, &$omodels, &$sp, $prompt, $mode, &$rl, $etx.clone(), &$cc,
        )
    };
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
    call_counts: Option<CallCounts>,
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
                let cc = call_counts.clone();

                let (primary_type, secondary_type) = prompt::classify_question(&question);
                let secondary_tag = secondary_type.map(|qt| prompt::question_type_to_tag(qt).to_string());

                let _ = etx.send(WsEvent::QuestionDetected {
                    question: question.clone(),
                    secondary_tag: secondary_tag.clone(),
                });

                tokio::spawn(async move {
                    let run = async {
                        if let Some(sec_type) = secondary_type {
                            // Compound only — primary/secondary generated on demand when user clicks tab
                            let compound_prompt = prompt::build_compound_user_prompt(&question, &tr, primary_type, sec_type);
                            run_suggest!(SuggestionMode::Compound, &compound_prompt,
                                gkey, akey, grkey, grkey2, orkey, mkey, ckey, qkey,
                                ourl, omodels, sp, rl, etx, cc).await?;
                        } else {
                            // Single type — primary only
                            let user_prompt = prompt::build_user_prompt(&question, &tr);
                            run_suggest!(SuggestionMode::Primary, &user_prompt,
                                gkey, akey, grkey, grkey2, orkey, mkey, ckey, qkey,
                                ourl, omodels, sp, rl, etx, cc).await?;
                        }
                        anyhow::Ok(())
                    };
                    if let Err(e) = run.await {
                        tracing::error!("Suggestion error: {}", e);
                        let _ = etx.send(WsEvent::Error {
                            message: format!("Suggestion error: {}", e),
                        });
                    }
                });
            }
            None => break,
        }
    }
}

/// Generate a single suggestion mode on demand (called from HTTP handler for opt-in primary/secondary).
pub async fn run_single(
    question: &str,
    mode: SuggestionMode,
    system_prompt: &str,
    transcript: &[common::messages::TranscriptSegment],
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
    rate_limiter: &RateLimiter,
    event_tx: broadcast::Sender<WsEvent>,
    call_counts: &Option<CallCounts>,
) -> anyhow::Result<()> {
    let (primary_type, secondary_type) = prompt::classify_question(question);
    let user_prompt = match mode {
        SuggestionMode::Secondary => {
            if let Some(sec_type) = secondary_type {
                prompt::build_user_prompt_for_type(question, transcript, sec_type)
            } else {
                prompt::build_user_prompt(question, transcript)
            }
        }
        SuggestionMode::Compound => {
            if let Some(sec_type) = secondary_type {
                prompt::build_compound_user_prompt(question, transcript, primary_type, sec_type)
            } else {
                prompt::build_user_prompt(question, transcript)
            }
        }
        SuggestionMode::Primary => prompt::build_user_prompt_for_type(question, transcript, primary_type),
    };
    suggest_with_fallback(
        gemini_key, anthropic_key, groq_key, groq_key_2,
        openrouter_key, mistral_key, cerebras_key, qwen_key,
        ollama_url, ollama_models, system_prompt, &user_prompt, mode,
        rate_limiter, event_tx, call_counts,
    ).await
}
