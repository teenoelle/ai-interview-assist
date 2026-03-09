pub mod gemini_vision;

use tokio::sync::{broadcast, mpsc};
use common::messages::WsEvent;

pub async fn run_agent(
    mut video_rx: mpsc::Receiver<Vec<u8>>,
    event_tx: broadcast::Sender<WsEvent>,
    gemini_key: String,
) {
    loop {
        match video_rx.recv().await {
            Some(jpeg_bytes) => {
                let key = gemini_key.clone();
                let etx = event_tx.clone();
                tokio::spawn(async move {
                    match gemini_vision::analyze_sentiment(&key, &jpeg_bytes).await {
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
