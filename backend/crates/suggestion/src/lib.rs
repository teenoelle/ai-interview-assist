pub mod detector;
pub mod gemini_llm;
pub mod prompt;

use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use common::messages::{TranscriptSegment, WsEvent};
use common::rate_limiter::RateLimiter;

pub async fn run_agent(
    mut question_rx: mpsc::Receiver<String>,
    event_tx: broadcast::Sender<WsEvent>,
    system_prompt: Arc<RwLock<String>>,
    transcript: Arc<RwLock<Vec<TranscriptSegment>>>,
    gemini_key: String,
    rate_limiter: RateLimiter,
) {
    loop {
        match question_rx.recv().await {
            Some(question) => {
                let key = gemini_key.clone();
                let etx = event_tx.clone();
                let sp = system_prompt.read().await.clone();
                let tr = transcript.read().await.clone();
                let rl = rate_limiter.clone();

                let user_prompt = prompt::build_user_prompt(&question, &tr);

                let _ = etx.send(WsEvent::QuestionDetected {
                    question: question.clone(),
                });

                tokio::spawn(async move {
                    // Acquire one token before streaming (counts as 1 request)
                    rl.acquire().await;
                    match gemini_llm::stream_suggestions(&key, &sp, &user_prompt, etx.clone()).await {
                        Ok(_) => {}
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
