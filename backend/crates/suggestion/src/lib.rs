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
use common::providers::{is_quota_exhausted, is_rate_limit, is_server_error, SuggestionProvider};
use common::circuit_breaker::CircuitBreaker;

pub type CallCounts = Arc<std::sync::Mutex<std::collections::HashMap<String, u64>>>;

fn inc(counts: &Option<CallCounts>, name: &str) {
    if let Some(map) = counts {
        if let Ok(mut m) = map.lock() {
            *m.entry(name.to_string()).or_insert(0) += 1;
        }
    }
}

// All circuit breakers use a 1-year reset — effectively permanent until server restart.
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

/// Quick reachability check for a local/LAN server (1.5 s timeout).
async fn is_reachable(url: &str) -> bool {
    reqwest::Client::new()
        .get(url)
        .timeout(std::time::Duration::from_millis(1500))
        .send()
        .await
        .is_ok()
}

// ── Context passed to every provider attempt ────────────────────────────────

struct SuggestCtx<'a> {
    gemini_key: &'a str,
    anthropic_key: Option<&'a str>,
    groq_key: Option<&'a str>,
    groq_key_2: Option<&'a str>,
    openrouter_key: Option<&'a str>,
    openrouter_model: Option<&'a str>,
    deepseek_key: Option<&'a str>,
    mistral_key: Option<&'a str>,
    cerebras_key: Option<&'a str>,
    qwen_key: Option<&'a str>,
    bonsai_url: Option<&'a str>,
    bonsai_model: &'a str,
    ollama_url: &'a str,
    ollama_models: &'a [String],
    system_prompt: &'a str,
    user_prompt: &'a str,
    cli_user_prompt: &'a str,
    mode: SuggestionMode,
    rate_limiter: &'a RateLimiter,
    event_tx: broadcast::Sender<WsEvent>,
    call_counts: &'a Option<CallCounts>,
}

enum ProviderOutcome {
    /// This provider succeeded — stop the chain.
    Success,
    /// Provider not configured, circuit open, or unreachable — skip silently.
    Skip,
    /// Quota/rate-limit/server error — log and try next provider.
    Fallthrough,
    /// Hard error — propagate up, don't try further providers.
    Fatal(anyhow::Error),
}

async fn try_one(provider: SuggestionProvider, ctx: &SuggestCtx<'_>) -> ProviderOutcome {
    let etx = ctx.event_tx.clone();
    let name = provider.name();

    // Helper closures so match arms stay concise
    let fallthrough = |e: anyhow::Error| {
        tracing::warn!("{} unavailable (quota/rate-limit), trying next: {}", name, e);
        let _ = etx.send(WsEvent::Error { message: format!("{}: {}", name, e) });
        ProviderOutcome::Fallthrough
    };
    let server_err = |e: anyhow::Error| {
        tracing::warn!("{} server error, trying next: {}", name, e);
        let _ = etx.send(WsEvent::Error { message: format!("{}: {}", name, e) });
        ProviderOutcome::Fallthrough
    };
    let success = || {
        inc(ctx.call_counts, name);
        let _ = ctx.event_tx.send(WsEvent::ProviderUsed {
            service: "suggestions".to_string(),
            provider: name.to_string(),
            local: provider.is_local(),
        });
        ProviderOutcome::Success
    };

    match provider {
        // ── Groq key 1 ──────────────────────────────────────────────────────
        SuggestionProvider::Groq => {
            if groq_cb().is_open() { return ProviderOutcome::Skip; }
            let Some(key) = ctx.groq_key else { return ProviderOutcome::Skip; };
            match groq_llm::stream_suggestions(key, ctx.system_prompt, ctx.user_prompt, ctx.mode, etx.clone()).await {
                Ok(()) => { groq_cb().record_success(); success() }
                Err(e) if is_quota_exhausted(&e) || is_rate_limit(&e) => {
                    if is_quota_exhausted(&e) { groq_cb().record_failure(); }
                    fallthrough(e)
                }
                Err(e) if is_server_error(&e) => server_err(e),
                Err(e) => ProviderOutcome::Fatal(e),
            }
        }

        // ── Groq key 2 ──────────────────────────────────────────────────────
        SuggestionProvider::Groq2 => {
            if groq2_cb().is_open() { return ProviderOutcome::Skip; }
            let Some(key) = ctx.groq_key_2 else { return ProviderOutcome::Skip; };
            match groq_llm::stream_suggestions(key, ctx.system_prompt, ctx.user_prompt, ctx.mode, etx.clone()).await {
                Ok(()) => { groq2_cb().record_success(); success() }
                Err(e) if is_quota_exhausted(&e) || is_rate_limit(&e) => {
                    if is_quota_exhausted(&e) { groq2_cb().record_failure(); }
                    fallthrough(e)
                }
                Err(e) if is_server_error(&e) => server_err(e),
                Err(e) => ProviderOutcome::Fatal(e),
            }
        }

        // ── Mistral ─────────────────────────────────────────────────────────
        SuggestionProvider::Mistral => {
            if mistral_cb().is_open() {
                tracing::warn!("Mistral: circuit breaker open (tripped by a prior quota/auth error) — skipping");
                return ProviderOutcome::Skip;
            }
            let Some(key) = ctx.mistral_key else {
                tracing::warn!("Mistral: no API key — skipping (set MISTRAL_API_KEY in .env or add via Providers panel)");
                return ProviderOutcome::Skip;
            };
            match mistral_llm::stream_suggestions(key, ctx.system_prompt, ctx.user_prompt, ctx.mode, etx.clone()).await {
                Ok(()) => { mistral_cb().record_success(); success() }
                Err(e) if is_quota_exhausted(&e) || is_rate_limit(&e) => {
                    if is_quota_exhausted(&e) { mistral_cb().record_failure(); }
                    fallthrough(e)
                }
                Err(e) if is_server_error(&e) => server_err(e),
                Err(e) => ProviderOutcome::Fatal(e),
            }
        }

        // ── Claude CLI ──────────────────────────────────────────────────────
        // Unique: classifies errors as transient (rate/overload) vs permanent (not installed).
        SuggestionProvider::ClaudeCli => {
            if claude_cli_cb().is_open() { return ProviderOutcome::Skip; }
            match claude_cli_llm::stream_suggestions(ctx.system_prompt, ctx.cli_user_prompt, ctx.mode, etx.clone()).await {
                Ok(()) => { claude_cli_cb().record_success(); success() }
                Err(e) => {
                    let msg = e.to_string();
                    let transient = msg.contains("rate") || msg.contains("overload")
                        || msg.contains("529") || msg.contains("Too Many") || msg.contains("capacity");
                    if !transient { claude_cli_cb().record_failure(); }
                    tracing::warn!("Claude CLI unavailable, falling back: {}", e);
                    let _ = etx.send(WsEvent::Error { message: format!("Claude CLI: {}", e) });
                    ProviderOutcome::Fallthrough
                }
            }
        }

        // ── Claude API ──────────────────────────────────────────────────────
        SuggestionProvider::ClaudeApi => {
            if claude_api_cb().is_open() { return ProviderOutcome::Skip; }
            let Some(key) = ctx.anthropic_key else { return ProviderOutcome::Skip; };
            match claude_llm::stream_suggestions(key, ctx.system_prompt, ctx.user_prompt, ctx.mode, etx.clone()).await {
                Ok(()) => { claude_api_cb().record_success(); success() }
                Err(e) if is_quota_exhausted(&e) || is_rate_limit(&e) => {
                    if is_quota_exhausted(&e) { claude_api_cb().record_failure(); }
                    fallthrough(e)
                }
                Err(e) if is_server_error(&e) => server_err(e),
                Err(e) => ProviderOutcome::Fatal(e),
            }
        }

        // ── Ollama (local) ──────────────────────────────────────────────────
        // Loops over all configured models; single reachability check up front.
        SuggestionProvider::Ollama => {
            let ollama_up = is_reachable(
                &format!("{}/api/tags", ctx.ollama_url.trim_end_matches('/'))
            ).await;
            if !ollama_up {
                tracing::debug!("Ollama unreachable — skipping all local models");
                return ProviderOutcome::Skip;
            }
            for model in ctx.ollama_models {
                match ollama_llm::stream_suggestions(
                    ctx.ollama_url, model, ctx.system_prompt, ctx.user_prompt, ctx.mode, etx.clone()
                ).await {
                    Ok(()) => {
                        let label = format!("Ollama ({})", model);
                        inc(ctx.call_counts, &label);
                        let _ = ctx.event_tx.send(WsEvent::ProviderUsed {
                            service: "suggestions".to_string(),
                            provider: label,
                            local: true,
                        });
                        return ProviderOutcome::Success;
                    }
                    Err(e) => {
                        tracing::warn!("Ollama {} failed, trying next model: {}", model, e);
                        let _ = etx.send(WsEvent::Error { message: format!("Ollama ({}): {}", model, e) });
                    }
                }
            }
            ProviderOutcome::Fallthrough
        }

        // ── OpenRouter ──────────────────────────────────────────────────────
        SuggestionProvider::OpenRouter => {
            let Some(key) = ctx.openrouter_key else { return ProviderOutcome::Skip; };
            match openrouter_llm::stream_suggestions(key, ctx.openrouter_model, ctx.system_prompt, ctx.user_prompt, ctx.mode, etx.clone()).await {
                Ok(()) => success(),
                Err(e) if is_quota_exhausted(&e) || is_rate_limit(&e) => fallthrough(e),
                Err(e) if is_server_error(&e) => server_err(e),
                Err(e) => ProviderOutcome::Fatal(e),
            }
        }

        // ── Qwen ────────────────────────────────────────────────────────────
        SuggestionProvider::Qwen => {
            let Some(key) = ctx.qwen_key else { return ProviderOutcome::Skip; };
            match qwen_llm::stream_suggestions(key, ctx.system_prompt, ctx.user_prompt, ctx.mode, etx.clone()).await {
                Ok(()) => success(),
                Err(e) if is_quota_exhausted(&e) || is_rate_limit(&e) => fallthrough(e),
                Err(e) if is_server_error(&e) => server_err(e),
                Err(e) => ProviderOutcome::Fatal(e),
            }
        }

        // ── Cerebras ────────────────────────────────────────────────────────
        SuggestionProvider::Cerebras => {
            let Some(key) = ctx.cerebras_key else { return ProviderOutcome::Skip; };
            match cerebras_llm::stream_suggestions(key, ctx.system_prompt, ctx.user_prompt, ctx.mode, etx.clone()).await {
                Ok(()) => success(),
                Err(e) if is_quota_exhausted(&e) || is_rate_limit(&e) => fallthrough(e),
                Err(e) if is_server_error(&e) => server_err(e),
                Err(e) => ProviderOutcome::Fatal(e),
            }
        }

        // ── DeepSeek ────────────────────────────────────────────────────────
        SuggestionProvider::DeepSeek => {
            let Some(key) = ctx.deepseek_key else { return ProviderOutcome::Skip; };
            match deepseek_llm::stream_suggestions(key, ctx.system_prompt, ctx.user_prompt, ctx.mode, etx.clone()).await {
                Ok(()) => success(),
                Err(e) if is_quota_exhausted(&e) || is_rate_limit(&e) => fallthrough(e),
                Err(e) if is_server_error(&e) => server_err(e),
                Err(e) => ProviderOutcome::Fatal(e),
            }
        }

        // ── LAN Ollama (Bonsai) ──────────────────────────────────────────────
        // Tries /health endpoint first, falls back to root ping.
        SuggestionProvider::LanOllama => {
            let Some(url) = ctx.bonsai_url else { return ProviderOutcome::Skip; };
            let base = url.trim_end_matches('/');
            let reachable = {
                let r = reqwest::Client::new()
                    .get(format!("{}/health", base))
                    .timeout(std::time::Duration::from_millis(1500))
                    .send().await;
                match r {
                    Ok(resp) if resp.status().is_success() => true,
                    _ => is_reachable(base).await,
                }
            };
            if !reachable {
                tracing::debug!("LAN Ollama unreachable — skipping");
                return ProviderOutcome::Skip;
            }
            match bonsai_llm::stream_suggestions(url, ctx.bonsai_model, ctx.system_prompt, ctx.user_prompt, ctx.mode, ctx.event_tx.clone()).await {
                Ok(()) => {
                    let label = format!("LAN Ollama ({})", ctx.bonsai_model);
                    inc(ctx.call_counts, &label);
                    let _ = ctx.event_tx.send(WsEvent::ProviderUsed {
                        service: "suggestions".to_string(),
                        provider: label,
                        local: false,
                    });
                    ProviderOutcome::Success
                }
                Err(e) => {
                    tracing::warn!("LAN Ollama ({}) unavailable, falling back: {}", ctx.bonsai_model, e);
                    let _ = etx.send(WsEvent::Error { message: format!("LAN Ollama ({}): {}", ctx.bonsai_model, e) });
                    ProviderOutcome::Fallthrough
                }
            }
        }

        // ── Gemma (Gemini API) ───────────────────────────────────────────────
        // Acquires a rate-limiter slot — shared with Gemini below.
        SuggestionProvider::Gemma => {
            ctx.rate_limiter.acquire().await;
            match gemma_llm::stream_suggestions(ctx.gemini_key, ctx.system_prompt, ctx.user_prompt, ctx.mode, etx.clone()).await {
                Ok(()) => success(),
                Err(e) if is_quota_exhausted(&e) || is_rate_limit(&e) => {
                    tracing::warn!("Gemma quota/rate-limit, trying Gemini: {}", e);
                    ProviderOutcome::Fallthrough
                }
                Err(e) => ProviderOutcome::Fatal(e),
            }
        }

        // ── Gemini ──────────────────────────────────────────────────────────
        SuggestionProvider::Gemini => {
            ctx.rate_limiter.acquire().await;
            match gemini_llm::stream_suggestions(ctx.gemini_key, ctx.system_prompt, ctx.user_prompt, ctx.mode, etx.clone()).await {
                Ok(()) => success(),
                Err(e) if is_quota_exhausted(&e) => {
                    tracing::warn!("Gemini suggestions quota exhausted: {}", e);
                    ProviderOutcome::Fallthrough
                }
                Err(e) => ProviderOutcome::Fatal(e),
            }
        }
    }
}

async fn suggest_with_fallback(
    order: &[SuggestionProvider],
    ctx: SuggestCtx<'_>,
) -> anyhow::Result<()> {
    for &provider in order {
        match try_one(provider, &ctx).await {
            ProviderOutcome::Success => return Ok(()),
            ProviderOutcome::Skip | ProviderOutcome::Fallthrough => continue,
            ProviderOutcome::Fatal(e) => return Err(e),
        }
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
    suggestion_order: Arc<RwLock<Vec<SuggestionProvider>>>,
    runtime_keys: Arc<RwLock<std::collections::HashMap<String, String>>>,
    runtime_urls: Arc<RwLock<std::collections::HashMap<String, String>>>,
    runtime_models: Arc<RwLock<std::collections::HashMap<String, String>>>,
) {
    loop {
        match question_rx.recv().await {
            Some(question) => {
                let etx = event_tx.clone();

                // Classify first — before any async locks — so smalltalk/closing fire instantly
                let (primary_type, secondary_type) = prompt::classify_question(&question);

                // Smalltalk: emit instant pre-written response, skip LLM entirely
                if matches!(primary_type, prompt::QuestionType::Smalltalk) {
                    tracing::info!("smalltalk bypass — instant response for: {:?}", question);
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

                // Closing: signal detection only, no auto-generation
                if matches!(primary_type, prompt::QuestionType::Closing) {
                    let _ = etx.send(WsEvent::QuestionDetected {
                        question: question.clone(),
                        secondary_tag: None,
                    });
                    continue;
                }

                let sp = system_prompt.read().await.clone();
                let tr = transcript.read().await.clone();
                let rl = rate_limiter.clone();
                let cc = call_counts.clone();
                let order = suggestion_order.read().await.clone();

                // Resolve runtime key overrides
                let rk = runtime_keys.read().await;
                let resolve = |name: &str, env: Option<String>| rk.get(name).cloned().or(env);
                let gkey  = gemini_key.clone();
                let akey  = resolve("anthropic",  anthropic_key.clone());
                let grkey = resolve("groq",        groq_key.clone());
                let grkey2= resolve("groq2",       groq_key_2.clone());
                let orkey = resolve("openrouter",  openrouter_key.clone());
                let dkey  = resolve("deepseek",    deepseek_key.clone());
                let mkey  = resolve("mistral",     mistral_key.clone());
                let ckey  = resolve("cerebras",    cerebras_key.clone());
                let qkey  = resolve("qwen",        qwen_key.clone());
                drop(rk);
                // Resolve runtime URL overrides
                let ru = runtime_urls.read().await;
                let ourl   = ru.get("ollama").cloned().unwrap_or_else(|| ollama_url.clone());
                let burl   = ru.get("lan_ollama").cloned().or_else(|| bonsai_url.clone());
                drop(ru);
                // Resolve runtime model overrides
                let rm = runtime_models.read().await;
                let ormodel = rm.get("openrouter").cloned();
                let omodels = if let Some(m) = rm.get("ollama").cloned() {
                    vec![m]
                } else {
                    ollama_models.clone()
                };
                let bmodel = rm.get("lan_ollama").cloned().unwrap_or_else(|| bonsai_model.clone());
                drop(rm);

                let secondary_tag = secondary_type.map(|qt| prompt::question_type_to_tag(qt).to_string());
                let _ = etx.send(WsEvent::QuestionDetected {
                    question: question.clone(),
                    secondary_tag: secondary_tag.clone(),
                });

                tokio::spawn(async move {
                    let run = async {
                        let user_prompt = if let Some(sec_type) = secondary_type {
                            prompt::build_compound_user_prompt(&question, &tr, primary_type, sec_type)
                        } else {
                            prompt::build_user_prompt(&question, &tr)
                        };
                        let cli_user_prompt = prompt::build_user_prompt_slim(&question, &tr);
                        let mode = if secondary_type.is_some() {
                            SuggestionMode::Compound
                        } else {
                            SuggestionMode::Primary
                        };
                        let ctx = SuggestCtx {
                            gemini_key: &gkey,
                            anthropic_key: akey.as_deref(),
                            groq_key: grkey.as_deref(),
                            groq_key_2: grkey2.as_deref(),
                            openrouter_key: orkey.as_deref(),
                            openrouter_model: ormodel.as_deref(),
                            deepseek_key: dkey.as_deref(),
                            mistral_key: mkey.as_deref(),
                            cerebras_key: ckey.as_deref(),
                            qwen_key: qkey.as_deref(),
                            bonsai_url: burl.as_deref(),
                            bonsai_model: &bmodel,
                            ollama_url: &ourl,
                            ollama_models: &omodels,
                            system_prompt: &sp,
                            user_prompt: &user_prompt,
                            cli_user_prompt: &cli_user_prompt,
                            mode,
                            rate_limiter: &rl,
                            event_tx: etx.clone(),
                            call_counts: &cc,
                        };
                        suggest_with_fallback(&order, ctx).await
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
/// Keys have already been resolved by the caller (runtime overrides applied).
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
    openrouter_model: Option<&str>,
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
    suggestion_order: &[SuggestionProvider],
) -> anyhow::Result<()> {
    let (primary_type, secondary_type) = prompt::classify_question(question);

    // Smalltalk: return instant pre-written response, skip LLM entirely
    if matches!(primary_type, prompt::QuestionType::Smalltalk) {
        let full_text = prompt::smalltalk_response(question);
        let _ = event_tx.send(WsEvent::SuggestionComplete { full_text, mode });
        return Ok(());
    }

    let ctx_prompt = prompt::make_ctx_prefix(transcript);
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
        SuggestionMode::ClosingHr  => prompt::build_closing_hr_prompt(&ctx_prompt, question),
        SuggestionMode::ClosingHm  => prompt::build_closing_hm_prompt(&ctx_prompt, question),
        SuggestionMode::ClosingCeo => prompt::build_closing_ceo_prompt(&ctx_prompt, question),
        SuggestionMode::Primary    => prompt::build_user_prompt_for_type(question, transcript, primary_type),
    };

    let ctx = SuggestCtx {
        gemini_key,
        anthropic_key,
        groq_key,
        groq_key_2,
        openrouter_key,
        openrouter_model,
        deepseek_key,
        mistral_key,
        cerebras_key,
        qwen_key,
        bonsai_url,
        bonsai_model,
        ollama_url,
        ollama_models,
        system_prompt,
        user_prompt: &user_prompt,
        cli_user_prompt: &user_prompt,
        mode,
        rate_limiter,
        event_tx,
        call_counts,
    };
    suggest_with_fallback(suggestion_order, ctx).await
}
