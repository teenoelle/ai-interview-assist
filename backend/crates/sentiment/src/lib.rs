pub mod gemini_vision;
pub mod claude_vision;
pub mod ollama_vision;

use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::{broadcast, mpsc, RwLock};
use common::messages::WsEvent;
use common::rate_limiter::RateLimiter;
use common::providers::{is_quota_exhausted, is_rate_limit, SentimentProvider};

struct SentCtx<'a> {
    gemini_key: &'a str,
    anthropic_key: Option<&'a str>,
    ollama_url: &'a str,
    ollama_vision_model: &'a str,
    jpeg_bytes: &'a [u8],
    rate_limiter: &'a RateLimiter,
    event_tx: &'a broadcast::Sender<WsEvent>,
}

enum SentOutcome {
    Success,
    Skip,
    Fallthrough,
    Fatal(anyhow::Error),
}

async fn try_one_sent(provider: SentimentProvider, ctx: &SentCtx<'_>) -> SentOutcome {
    let name = provider.name();

    match provider {
        SentimentProvider::OllamaVision => {
            match ollama_vision::analyze_sentiment(ctx.ollama_url, ctx.ollama_vision_model, ctx.jpeg_bytes).await {
                Ok(result) => {
                    tracing::info!("sentiment ✓ Ollama ({})", ctx.ollama_vision_model);
                    let _ = ctx.event_tx.send(WsEvent::Sentiment {
                        emotion: result.emotion, reason: result.reason,
                        coaching: result.coaching, coaching_why: result.coaching_why,
                    });
                    let _ = ctx.event_tx.send(WsEvent::ProviderUsed {
                        service: "sentiment".to_string(),
                        provider: format!("Ollama ({})", ctx.ollama_vision_model),
                        local: true,
                    });
                    SentOutcome::Success
                }
                Err(e) => {
                    tracing::warn!("Ollama vision unavailable, trying next: {}", e);
                    SentOutcome::Fallthrough
                }
            }
        }

        SentimentProvider::GeminiVision => {
            ctx.rate_limiter.acquire().await;
            match gemini_vision::analyze_sentiment(ctx.gemini_key, ctx.jpeg_bytes).await {
                Ok(result) => {
                    tracing::info!("sentiment ✓ Gemini Vision");
                    let _ = ctx.event_tx.send(WsEvent::Sentiment {
                        emotion: result.emotion, reason: result.reason,
                        coaching: result.coaching, coaching_why: result.coaching_why,
                    });
                    let _ = ctx.event_tx.send(WsEvent::ProviderUsed {
                        service: "sentiment".to_string(),
                        provider: name.to_string(),
                        local: false,
                    });
                    SentOutcome::Success
                }
                Err(e) if is_rate_limit(&e) || is_quota_exhausted(&e) => {
                    tracing::warn!("Gemini sentiment rate-limited, trying next: {}", e);
                    SentOutcome::Fallthrough
                }
                Err(e) => SentOutcome::Fatal(e),
            }
        }

        SentimentProvider::ClaudeVision => {
            let Some(key) = ctx.anthropic_key else { return SentOutcome::Skip; };
            match claude_vision::analyze_sentiment(key, ctx.jpeg_bytes).await {
                Ok(result) => {
                    tracing::info!("sentiment ✓ Claude");
                    let _ = ctx.event_tx.send(WsEvent::Sentiment {
                        emotion: result.emotion, reason: result.reason,
                        coaching: result.coaching, coaching_why: result.coaching_why,
                    });
                    let _ = ctx.event_tx.send(WsEvent::ProviderUsed {
                        service: "sentiment".to_string(),
                        provider: name.to_string(),
                        local: false,
                    });
                    if let (Some(remaining), Some(limit)) = (result.requests_remaining, result.requests_limit) {
                        let _ = ctx.event_tx.send(WsEvent::RateLimit {
                            provider: name.to_string(),
                            requests_remaining: remaining,
                            requests_limit: limit,
                        });
                    }
                    SentOutcome::Success
                }
                Err(e) => {
                    tracing::warn!("Claude sentiment unavailable: {}", e);
                    SentOutcome::Fallthrough
                }
            }
        }
    }
}

pub async fn run_agent(
    mut video_rx: mpsc::Receiver<Vec<u8>>,
    event_tx: broadcast::Sender<WsEvent>,
    gemini_key: String,
    anthropic_key: Option<String>,
    ollama_url: String,
    ollama_vision_model: String,
    rate_limiter: RateLimiter,
    sentiment_order: Arc<RwLock<Vec<SentimentProvider>>>,
    runtime_keys: Arc<RwLock<HashMap<String, String>>>,
    runtime_urls: Arc<RwLock<HashMap<String, String>>>,
) {
    loop {
        match video_rx.recv().await {
            Some(jpeg_bytes) => {
                let gkey = gemini_key.clone();
                let ovmodel = ollama_vision_model.clone();
                let etx = event_tx.clone();
                let rl = rate_limiter.clone();
                let order = sentiment_order.read().await.clone();
                let rk = runtime_keys.read().await;
                let akey = rk.get("anthropic").cloned().or_else(|| anthropic_key.clone());
                drop(rk);
                let ru = runtime_urls.read().await;
                let ourl = ru.get("ollama").cloned().unwrap_or_else(|| ollama_url.clone());
                drop(ru);

                tokio::spawn(async move {
                    let ctx = SentCtx {
                        gemini_key: &gkey,
                        anthropic_key: akey.as_deref(),
                        ollama_url: &ourl,
                        ollama_vision_model: &ovmodel,
                        jpeg_bytes: &jpeg_bytes,
                        rate_limiter: &rl,
                        event_tx: &etx,
                    };
                    for &provider in &order {
                        match try_one_sent(provider, &ctx).await {
                            SentOutcome::Success => return,
                            SentOutcome::Skip | SentOutcome::Fallthrough => continue,
                            SentOutcome::Fatal(e) => {
                                tracing::error!("Sentiment error: {}", e);
                                return;
                            }
                        }
                    }
                });
            }
            None => break,
        }
    }
}
