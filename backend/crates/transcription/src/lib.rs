pub mod buffer;
pub mod vad;
pub mod gemini;

use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use common::messages::{TranscriptSegment, WsEvent};
use common::rate_limiter::{RateLimiter, with_retry};
use std::time::{SystemTime, UNIX_EPOCH};

fn is_question(text: &str) -> bool {
    let text = text.trim();
    if text.ends_with('?') {
        return true;
    }
    let lower = text.to_lowercase();
    let question_starters = [
        "what", "why", "how", "when", "where", "who", "which",
        "can you", "could you", "would you", "tell me", "describe", "explain",
        "have you", "do you", "did you", "are you", "were you",
        "walk me", "give me", "share",
    ];
    let words: Vec<&str> = lower.split_whitespace().collect();
    if words.is_empty() {
        return false;
    }
    for q in &question_starters {
        let q_words: Vec<&str> = q.split_whitespace().collect();
        if words.len() >= q_words.len() {
            let matches = q_words.iter().zip(words.iter()).all(|(a, b)| a == b);
            if matches {
                return true;
            }
        }
    }
    false
}

pub async fn run_agent(
    mut audio_rx: mpsc::Receiver<Vec<u8>>,
    question_tx: mpsc::Sender<String>,
    event_tx: broadcast::Sender<WsEvent>,
    transcript: Arc<RwLock<Vec<TranscriptSegment>>>,
    gemini_key: String,
    rate_limiter: RateLimiter,
) {
    let mut ring_buf = buffer::RingBuffer::new();

    loop {
        match audio_rx.recv().await {
            Some(pcm_chunk) => {
                ring_buf.push(&pcm_chunk);
                if ring_buf.should_flush() {
                    let segment_pcm = ring_buf.drain_segment();
                    if segment_pcm.is_empty() {
                        continue;
                    }
                    let key = gemini_key.clone();
                    let qtx = question_tx.clone();
                    let etx = event_tx.clone();
                    let tr = transcript.clone();
                    let rl = rate_limiter.clone();
                    tokio::spawn(async move {
                        let result = with_retry(&rl, || {
                            let k = key.clone();
                            let pcm = segment_pcm.clone();
                            async move { gemini::transcribe(&k, &pcm).await }
                        })
                        .await;

                        match result {
                            Ok(text) if !text.trim().is_empty() => {
                                let ts = SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_millis() as u64;
                                let seg = TranscriptSegment {
                                    text: text.clone(),
                                    timestamp_ms: ts,
                                };
                                {
                                    let mut t = tr.write().await;
                                    t.push(seg);
                                    if t.len() > 100 {
                                        t.remove(0);
                                    }
                                }
                                let _ = etx.send(WsEvent::Transcript {
                                    text: text.clone(),
                                    timestamp_ms: ts,
                                });
                                if is_question(&text) {
                                    let _ = qtx.send(text).await;
                                }
                            }
                            Ok(_) => {}
                            Err(e) => {
                                tracing::error!("Transcription error: {}", e);
                                let _ = etx.send(WsEvent::Error {
                                    message: format!("Transcription error: {}", e),
                                });
                            }
                        }
                    });
                }
            }
            None => break,
        }
    }
}
