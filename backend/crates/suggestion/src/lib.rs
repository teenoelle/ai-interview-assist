pub mod detector;
pub mod gemini_llm;
pub mod groq_llm;
pub mod openrouter_llm;
pub mod mistral_llm;
pub mod cerebras_llm;
pub mod claude_llm;
pub mod qwen_llm;
pub mod bonsai_llm;
pub mod claude_cli_llm;
pub mod deepseek_llm;
pub mod gemma_llm;
pub mod ollama_llm;
pub mod prompt;

use std::sync::Arc;
use std::sync::OnceLock;
use tokio::sync::{broadcast, mpsc, RwLock};
use common::messages::{TranscriptSegment, WsEvent, SuggestionMode};
use common::rate_limiter::RateLimiter;
use common::providers::{is_quota_exhausted, is_rate_limit, is_server_error};
use common::circuit_breaker::CircuitBreaker;

pub type CallCounts = Arc<std::sync::Mutex<std::collections::HashMap<String, u64>>>;

fn inc(counts: &Option<CallCounts>, name: &str) {
    if let Some(map) = counts {
        if let Ok(mut m) = map.lock() {
            *m.entry(name.to_string()).or_insert(0) += 1;
        }
    }
}

fn provider_is_local(name: &str) -> bool {
    name.starts_with("Ollama") || name == "Bonsai" || name == "Claude CLI" || name == "Whisper (local)"
}

// All circuit breakers use a 1-year reset — effectively permanent until server restart.
// A browser page refresh does not restart the server, so once a provider hits quota
// it stays skipped for the entire session.
const CB_PERMANENT: u64 = 86_400 * 365;

static CLAUDE_CLI_CB: OnceLock<CircuitBreaker> = OnceLock::new();
fn claude_cli_cb() -> &'static CircuitBreaker {
    CLAUDE_CLI_CB.get_or_init(|| CircuitBreaker::new("claude-cli", 1, CB_PERMANENT))
}

static CLAUDE_API_CB: OnceLock<CircuitBreaker> = OnceLock::new();
fn claude_api_cb() -> &'static CircuitBreaker {
    CLAUDE_API_CB.get_or_init(|| CircuitBreaker::new("claude-api", 1, CB_PERMANENT))
}

static GROQ_CB: OnceLock<CircuitBreaker> = OnceLock::new();
fn groq_cb() -> &'static CircuitBreaker {
    GROQ_CB.get_or_init(|| CircuitBreaker::new("groq", 1, CB_PERMANENT))
}

static GROQ2_CB: OnceLock<CircuitBreaker> = OnceLock::new();
fn groq2_cb() -> &'static CircuitBreaker {
    GROQ2_CB.get_or_init(|| CircuitBreaker::new("groq-2", 1, CB_PERMANENT))
}

static MISTRAL_CB: OnceLock<CircuitBreaker> = OnceLock::new();
fn mistral_cb() -> &'static CircuitBreaker {
    MISTRAL_CB.get_or_init(|| CircuitBreaker::new("mistral", 1, CB_PERMANENT))
}

/// Quick reachability check for a local server (1.5 s timeout).
/// Returns true if the server responds to a GET on its health/root endpoint.
async fn is_reachable(url: &str) -> bool {
    reqwest::Client::new()
        .get(url)
        .timeout(std::time::Duration::from_millis(1500))
        .send()
        .await
        .is_ok()
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
                let _ = $etx.send(WsEvent::Error { message: format!("{}: {}", $name, e) });
            }
            Err(e) if is_server_error(&e) => {
                tracing::warn!("{} server error, trying next provider: {}", $name, e);
                let _ = $etx.send(WsEvent::Error { message: format!("{}: {}", $name, e) });
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
    deepseek_key: Option<&str>,
    mistral_key: Option<&str>,
    cerebras_key: Option<&str>,
    qwen_key: Option<&str>,
    bonsai_url: Option<&str>,
    bonsai_model: &str,
    ollama_url: &str,
    ollama_models: &[String],
    system_prompt: &str,
    user_prompt: &str,
    mode: SuggestionMode,
    rate_limiter: &RateLimiter,
    event_tx: broadcast::Sender<WsEvent>,
    call_counts: &Option<CallCounts>,
) -> anyhow::Result<()> {
    // 1. Groq key 1 — fast (~2-5 s), good format compliance
    if groq_cb().is_open() {
        tracing::debug!("Groq circuit open — skipping");
    } else if let Some(key) = groq_key {
        match groq_llm::stream_suggestions(key, system_prompt, user_prompt, mode, event_tx.clone()).await {
            Ok(()) => {
                groq_cb().record_success();
                inc(call_counts, "Groq");
                let _ = event_tx.send(WsEvent::ProviderUsed { service: "suggestions".to_string(), provider: "Groq".to_string(), local: false });
                return Ok(());
            }
            Err(e) if is_quota_exhausted(&e) || is_rate_limit(&e) => {
                if is_quota_exhausted(&e) { groq_cb().record_failure(); }
                tracing::warn!("Groq unavailable (quota/rate-limit): {}", e);
                let _ = event_tx.send(WsEvent::Error { message: format!("Groq: {}", e) });
            }
            Err(e) if is_server_error(&e) => {
                tracing::warn!("Groq server error, trying next: {}", e);
                let _ = event_tx.send(WsEvent::Error { message: format!("Groq: {}", e) });
            }
            Err(e) => return Err(e),
        }
    }

    // 2. Mistral — higher monthly limits, fallback when Groq exhausted
    if mistral_cb().is_open() {
        tracing::debug!("Mistral circuit open — skipping");
    } else if let Some(key) = mistral_key {
        match mistral_llm::stream_suggestions(key, system_prompt, user_prompt, mode, event_tx.clone()).await {
            Ok(()) => {
                mistral_cb().record_success();
                inc(call_counts, "Mistral");
                let _ = event_tx.send(WsEvent::ProviderUsed { service: "suggestions".to_string(), provider: "Mistral".to_string(), local: false });
                return Ok(());
            }
            Err(e) if is_quota_exhausted(&e) || is_rate_limit(&e) => {
                if is_quota_exhausted(&e) { mistral_cb().record_failure(); }
                tracing::warn!("Mistral unavailable (quota/rate-limit): {}", e);
                let _ = event_tx.send(WsEvent::Error { message: format!("Mistral: {}", e) });
            }
            Err(e) if is_server_error(&e) => {
                tracing::warn!("Mistral server error, trying next: {}", e);
                let _ = event_tx.send(WsEvent::Error { message: format!("Mistral: {}", e) });
            }
            Err(e) => return Err(e),
        }
    }

    // 3. Groq key 2 — independent quota, same model pool
    if groq2_cb().is_open() {
        tracing::debug!("Groq #2 circuit open — skipping");
    } else if let Some(key) = groq_key_2 {
        match groq_llm::stream_suggestions(key, system_prompt, user_prompt, mode, event_tx.clone()).await {
            Ok(()) => {
                groq2_cb().record_success();
                inc(call_counts, "Groq #2");
                let _ = event_tx.send(WsEvent::ProviderUsed { service: "suggestions".to_string(), provider: "Groq #2".to_string(), local: false });
                return Ok(());
            }
            Err(e) if is_quota_exhausted(&e) || is_rate_limit(&e) => {
                if is_quota_exhausted(&e) { groq2_cb().record_failure(); }
                tracing::warn!("Groq #2 unavailable (quota/rate-limit): {}", e);
                let _ = event_tx.send(WsEvent::Error { message: format!("Groq #2: {}", e) });
            }
            Err(e) if is_server_error(&e) => {
                tracing::warn!("Groq #2 server error, trying next: {}", e);
                let _ = event_tx.send(WsEvent::Error { message: format!("Groq #2: {}", e) });
            }
            Err(e) => return Err(e),
        }
    }

    // 4. Claude CLI — circuit breaker: 1 failure → permanent skip
    if claude_cli_cb().is_open() {
        tracing::debug!("Claude CLI circuit open — skipping");
    } else {
        match claude_cli_llm::stream_suggestions(system_prompt, user_prompt, mode, event_tx.clone()).await {
            Ok(()) => {
                claude_cli_cb().record_success();
                inc(call_counts, "Claude CLI");
                let _ = event_tx.send(WsEvent::ProviderUsed {
                    service: "suggestions".to_string(),
                    provider: "Claude CLI".to_string(),
                    local: true,
                });
                return Ok(());
            }
            Err(e) => {
                let msg = e.to_string();
                let transient = msg.contains("rate") || msg.contains("overload")
                    || msg.contains("529") || msg.contains("Too Many") || msg.contains("capacity");
                if !transient {
                    claude_cli_cb().record_failure();
                }
                tracing::warn!("Claude CLI unavailable, falling back: {}", e);
                let _ = event_tx.send(WsEvent::Error { message: format!("Claude CLI: {}", e) });
            }
        }
    }

    // 5. Claude API — circuit breaker: 1 quota failure → permanent skip
    if claude_api_cb().is_open() {
        tracing::debug!("Claude API circuit open — skipping");
    } else if let Some(key) = anthropic_key {
        match claude_llm::stream_suggestions(key, system_prompt, user_prompt, mode, event_tx.clone()).await {
            Ok(()) => {
                claude_api_cb().record_success();
                inc(call_counts, "Claude API");
                let _ = event_tx.send(WsEvent::ProviderUsed {
                    service: "suggestions".to_string(),
                    provider: "Claude API".to_string(),
                    local: false,
                });
                return Ok(());
            }
            Err(e) if is_quota_exhausted(&e) || is_rate_limit(&e) => {
                if is_quota_exhausted(&e) { claude_api_cb().record_failure(); }
                tracing::warn!("Claude API unavailable (quota/rate-limit), trying next: {}", e);
                let _ = event_tx.send(WsEvent::Error { message: format!("Claude API: {}", e) });
            }
            Err(e) if is_server_error(&e) => {
                tracing::warn!("Claude API server error, trying next: {}", e);
                let _ = event_tx.send(WsEvent::Error { message: format!("Claude API: {}", e) });
            }
            Err(e) => return Err(e),
        }
    }

    // 6. Ollama — local models; single reachability check before looping
    let ollama_up = is_reachable(&format!("{}/api/tags", ollama_url.trim_end_matches('/'))).await;
    if !ollama_up {
        tracing::debug!("Ollama unreachable — skipping all local models");
    } else {
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
                Err(e) => {
                    tracing::warn!("Ollama {} unavailable, trying next: {}", model, e);
                    let _ = event_tx.send(WsEvent::Error { message: format!("Ollama ({}): {}", model, e) });
                }
            }
        }
    }

    // 7. OpenRouter
    if let Some(key) = openrouter_key {
        try_provider!("OpenRouter",
            openrouter_llm::stream_suggestions(key, system_prompt, user_prompt, mode, event_tx.clone()),
            call_counts, event_tx);
    }

    // 8. Qwen
    if let Some(key) = qwen_key {
        try_provider!("Qwen",
            qwen_llm::stream_suggestions(key, system_prompt, user_prompt, mode, event_tx.clone()),
            call_counts, event_tx);
    }

    // 9. Cerebras
    if let Some(key) = cerebras_key {
        try_provider!("Cerebras",
            cerebras_llm::stream_suggestions(key, system_prompt, user_prompt, mode, event_tx.clone()),
            call_counts, event_tx);
    }

    // 10. DeepSeek
    if let Some(key) = deepseek_key {
        try_provider!("DeepSeek",
            deepseek_llm::stream_suggestions(key, system_prompt, user_prompt, mode, event_tx.clone()),
            call_counts, event_tx);
    }

    // 11. LAN Ollama — remote machine; health-check first (/health or /)
    if let Some(url) = bonsai_url {
        let base = url.trim_end_matches('/');
        let reachable = {
            let r = reqwest::Client::new().get(format!("{}/health", base))
                .timeout(std::time::Duration::from_millis(1500)).send().await;
            match r {
                Ok(resp) if resp.status().is_success() => true,
                _ => is_reachable(base).await,
            }
        };
        if !reachable {
            tracing::debug!("LAN Ollama unreachable — skipping");
        } else {
            match bonsai_llm::stream_suggestions(url, bonsai_model, system_prompt, user_prompt, mode, event_tx.clone()).await {
                Ok(()) => {
                    let name = format!("LAN Ollama ({})", bonsai_model);
                    inc(call_counts, &name);
                    let _ = event_tx.send(WsEvent::ProviderUsed {
                        service: "suggestions".to_string(),
                        provider: name,
                        local: false,
                    });
                    return Ok(());
                }
                Err(e) => {
                    tracing::warn!("LAN Ollama ({}) unavailable, falling back: {}", bonsai_model, e);
                    let _ = event_tx.send(WsEvent::Error { message: format!("LAN Ollama ({}): {}", bonsai_model, e) });
                }
            }
        }
    }

    // 12. Gemma 4 — Gemini API, rate-limited
    rate_limiter.acquire().await;
    match gemma_llm::stream_suggestions(gemini_key, system_prompt, user_prompt, mode, event_tx.clone()).await {
        Ok(()) => {
            inc(call_counts, "Gemma");
            let _ = event_tx.send(WsEvent::ProviderUsed {
                service: "suggestions".to_string(),
                provider: "Gemma".to_string(),
                local: false,
            });
            return Ok(());
        }
        Err(e) if is_quota_exhausted(&e) || is_rate_limit(&e) => tracing::warn!("Gemma suggestions quota/rate-limit, trying Gemini"),
        Err(e) => return Err(e),
    }

    // 14. Gemini — absolute last resort, keep credits for sentiment analysis
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
     $orkey:expr, $dkey:expr, $mkey:expr, $ckey:expr, $qkey:expr,
     $burl:expr, $bmodel:expr, $ourl:expr, $omodels:expr, $sp:expr, $rl:expr, $etx:expr, $cc:expr) => {
        suggest_with_fallback(
            &$gkey, $akey.as_deref(), $grkey.as_deref(), $grkey2.as_deref(),
            $orkey.as_deref(), $dkey.as_deref(), $mkey.as_deref(), $ckey.as_deref(), $qkey.as_deref(),
            $burl.as_deref(), &$bmodel, &$ourl, &$omodels, &$sp, $prompt, $mode, &$rl, $etx.clone(), &$cc,
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
    deepseek_key: Option<String>,
    mistral_key: Option<String>,
    cerebras_key: Option<String>,
    qwen_key: Option<String>,
    bonsai_url: Option<String>,
    bonsai_model: String,
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
                let dkey = deepseek_key.clone();
                let mkey = mistral_key.clone();
                let ckey = cerebras_key.clone();
                let qkey = qwen_key.clone();
                let burl = bonsai_url.clone();
                let bmodel = bonsai_model.clone();
                let ourl = ollama_url.clone();
                let omodels = ollama_models.clone();
                let etx = event_tx.clone();
                let sp = system_prompt.read().await.clone();
                let tr = transcript.read().await.clone();
                let rl = rate_limiter.clone();
                let cc = call_counts.clone();

                let (primary_type, secondary_type) = prompt::classify_question(&question);

                // Smalltalk: emit instant pre-written response, skip LLM entirely
                if matches!(primary_type, prompt::QuestionType::Smalltalk) {
                    let _ = etx.send(WsEvent::QuestionDetected {
                        question: question.clone(),
                        secondary_tag: None,
                    });
                    let _ = etx.send(WsEvent::SuggestionComplete {
                        full_text: prompt::smalltalk_response(&question),
                        mode: SuggestionMode::Primary,
                    });
                    continue;
                }

                // Closing: sections fetched on-demand — just signal detection, no auto-generation
                if matches!(primary_type, prompt::QuestionType::Closing) {
                    let _ = etx.send(WsEvent::QuestionDetected {
                        question: question.clone(),
                        secondary_tag: None,
                    });
                    continue;
                }

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
                                gkey, akey, grkey, grkey2, orkey, dkey, mkey, ckey, qkey,
                                burl, bmodel, ourl, omodels, sp, rl, etx, cc).await?;
                        } else {
                            // Single type — primary only
                            let user_prompt = prompt::build_user_prompt(&question, &tr);
                            run_suggest!(SuggestionMode::Primary, &user_prompt,
                                gkey, akey, grkey, grkey2, orkey, dkey, mkey, ckey, qkey,
                                burl, bmodel, ourl, omodels, sp, rl, etx, cc).await?;
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
    deepseek_key: Option<&str>,
    mistral_key: Option<&str>,
    cerebras_key: Option<&str>,
    qwen_key: Option<&str>,
    bonsai_url: Option<&str>,
    bonsai_model: &str,
    ollama_url: &str,
    ollama_models: &[String],
    rate_limiter: &RateLimiter,
    event_tx: broadcast::Sender<WsEvent>,
    call_counts: &Option<CallCounts>,
) -> anyhow::Result<()> {
    let (primary_type, secondary_type) = prompt::classify_question(question);

    // Smalltalk: return instant pre-written response, skip LLM entirely
    if matches!(primary_type, prompt::QuestionType::Smalltalk) {
        let full_text = prompt::smalltalk_response(question);
        let _ = event_tx.send(WsEvent::SuggestionComplete { full_text, mode });
        return Ok(());
    }

    let ctx = prompt::make_ctx_prefix(transcript);
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
        SuggestionMode::ClosingHr  => prompt::build_closing_hr_prompt(&ctx, question),
        SuggestionMode::ClosingHm  => prompt::build_closing_hm_prompt(&ctx, question),
        SuggestionMode::ClosingCeo => prompt::build_closing_ceo_prompt(&ctx, question),
        SuggestionMode::Primary => prompt::build_user_prompt_for_type(question, transcript, primary_type),
    };
    suggest_with_fallback(
        gemini_key, anthropic_key, groq_key, groq_key_2,
        openrouter_key, deepseek_key, mistral_key, cerebras_key, qwen_key,
        bonsai_url, bonsai_model, ollama_url, ollama_models,
        system_prompt, &user_prompt, mode,
        rate_limiter, event_tx, call_counts,
    ).await
}
