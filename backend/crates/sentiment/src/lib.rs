pub mod gemini_vision;

use tokio::sync::{broadcast, mpsc};
use common::messages::WsEvent;
use common::rate_limiter::{RateLimiter, with_retry};

pub async fn run_agent(
    mut video_rx: mpsc::Receiver<Vec<u8>>,
    event_tx: broadcast::Sender<WsEvent>,
    gemini_key: String,
    rate_limiter: RateLimiter,
) {
    loop {
        match video_rx.recv().await {
            Some(jpeg_bytes) => {
                let key = gemini_key.clone();
                let etx = event_tx.clone();
                let rl = rate_limiter.clone();
                tokio::spawn(async move {
                    let result = with_retry(&rl, || {
                        let k = key.clone();
                        let jpg = jpeg_bytes.clone();
                        async move { gemini_vision::analyze_sentiment(&k, &jpg).await }
                    })
                    .await;

                    match result {
                        Ok(emotion) => {
                            let _ = etx.send(WsEvent::Sentiment { emotion });
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
