pub mod gemini_vision;
pub mod claude_vision;
pub mod ollama_vision;

use tokio::sync::{broadcast, mpsc};
use common::messages::WsEvent;
use common::rate_limiter::RateLimiter;
use common::providers::{is_quota_exhausted, is_rate_limit};

pub async fn run_agent(
    mut video_rx: mpsc::Receiver<Vec<u8>>,
    event_tx: broadcast::Sender<WsEvent>,
    gemini_key: String,
    anthropic_key: Option<String>,
    ollama_url: String,
    ollama_vision_model: String,
    rate_limiter: RateLimiter,
) {
    loop {
        match video_rx.recv().await {
            Some(jpeg_bytes) => {
                let gkey = gemini_key.clone();
                let akey = anthropic_key.clone();
                let ourl = ollama_url.clone();
                let ovmodel = ollama_vision_model.clone();
                let etx = event_tx.clone();
                let rl = rate_limiter.clone();

                tokio::spawn(async move {
                    // 1. Ollama vision — local, free, no quota
                    match ollama_vision::analyze_sentiment(&ourl, &ovmodel, &jpeg_bytes).await {
                        Ok(result) => {
                            tracing::info!("sentiment ✓ Ollama ({})", ovmodel);
                            let _ = etx.send(WsEvent::Sentiment { emotion: result.emotion, reason: result.reason, coaching: result.coaching, coaching_why: result.coaching_why });
                            let _ = etx.send(WsEvent::ProviderUsed { service: "sentiment".to_string(), provider: format!("Ollama ({})", ovmodel), local: true });
                            return;
                        }
                        Err(e) => tracing::warn!("Ollama vision unavailable, trying Gemini: {}", e),
                    }

                    // 2. Gemini Vision — rate-limited to avoid competing with transcription
                    rl.acquire().await;
                    match gemini_vision::analyze_sentiment(&gkey, &jpeg_bytes).await {
                        Ok(result) => {
                            tracing::info!("sentiment ✓ Gemini Vision");
                            let _ = etx.send(WsEvent::Sentiment { emotion: result.emotion, reason: result.reason, coaching: result.coaching, coaching_why: result.coaching_why });
                            let _ = etx.send(WsEvent::ProviderUsed { service: "sentiment".to_string(), provider: "Gemini Vision".to_string(), local: false });
                            return;
                        }
                        Err(e) if is_rate_limit(&e) || is_quota_exhausted(&e) => {
                            tracing::warn!("Gemini sentiment rate-limited, trying Claude: {}", e);
                        }
                        Err(e) => {
                            tracing::error!("Sentiment error: {}", e);
                            return;
                        }
                    }

                    // 3. Claude — last resort
                    if let Some(key) = &akey {
                        match claude_vision::analyze_sentiment(key, &jpeg_bytes).await {
                            Ok(result) => {
                                tracing::info!("sentiment ✓ Claude");
                                let _ = etx.send(WsEvent::Sentiment { emotion: result.emotion, reason: result.reason, coaching: result.coaching, coaching_why: result.coaching_why });
                                let _ = etx.send(WsEvent::ProviderUsed { service: "sentiment".to_string(), provider: "Claude API".to_string(), local: false });
                                if let (Some(remaining), Some(limit)) =
                                    (result.requests_remaining, result.requests_limit)
                                {
                                    let _ = etx.send(WsEvent::RateLimit {
                                        provider: "Claude API".to_string(),
                                        requests_remaining: remaining,
                                        requests_limit: limit,
                                    });
                                }
                            }
                            Err(e) => tracing::warn!("Claude sentiment unavailable: {}", e),
                        }
                    }
                });
            }
            None => break,
        }
    }
}
