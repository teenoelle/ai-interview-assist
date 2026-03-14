pub mod gemini_vision;
pub mod claude_vision;

use tokio::sync::{broadcast, mpsc};
use common::messages::WsEvent;
use common::rate_limiter::{RateLimiter, with_retry};
use common::providers::{is_quota_exhausted, is_rate_limit};

pub async fn run_agent(
    mut video_rx: mpsc::Receiver<Vec<u8>>,
    event_tx: broadcast::Sender<WsEvent>,
    gemini_key: String,
    anthropic_key: Option<String>,
    rate_limiter: RateLimiter,
) {
    loop {
        match video_rx.recv().await {
            Some(jpeg_bytes) => {
                let gkey = gemini_key.clone();
                let akey = anthropic_key.clone();
                let etx = event_tx.clone();
                let rl = rate_limiter.clone();

                tokio::spawn(async move {
                    // 1. Claude — primary when key is available (no rate limiter needed,
                    //    separate quota from Gemini)
                    if let Some(key) = &akey {
                        match claude_vision::analyze_sentiment(key, &jpeg_bytes).await {
                            Ok(result) => {
                                let _ = etx.send(WsEvent::Sentiment { emotion: result.emotion, reason: result.reason, coaching: result.coaching });
                                // Broadcast rate limit info so the frontend can display it
                                if let (Some(remaining), Some(limit)) =
                                    (result.requests_remaining, result.requests_limit)
                                {
                                    let _ = etx.send(WsEvent::RateLimit {
                                        provider: "Claude".to_string(),
                                        requests_remaining: remaining,
                                        requests_limit: limit,
                                    });
                                }
                                return;
                            }
                            Err(e) if is_quota_exhausted(&e) || is_rate_limit(&e) => {
                                tracing::warn!("Claude sentiment unavailable, falling back to Gemini: {}", e);
                            }
                            Err(e) => {
                                tracing::error!("Claude sentiment error: {}", e);
                                let _ = etx.send(WsEvent::Error {
                                    message: format!("Sentiment error: {}", e),
                                });
                                return;
                            }
                        }
                    }

                    // 2. Gemini — fallback
                    let result = with_retry(&rl, || {
                        let k = gkey.clone();
                        let jpg = jpeg_bytes.clone();
                        async move { gemini_vision::analyze_sentiment(&k, &jpg).await }
                    })
                    .await;

                    match result {
                        Ok(result) => {
                            let _ = etx.send(WsEvent::Sentiment { emotion: result.emotion, reason: result.reason, coaching: result.coaching });
                        }
                        Err(e) => {
                            tracing::error!("Sentiment error: {}", e);
                            let _ = etx.send(WsEvent::Error {
                                message: format!("Sentiment error: {}", e),
                            });
                        }
                    }
                });
            }
            None => break,
        }
    }
}
