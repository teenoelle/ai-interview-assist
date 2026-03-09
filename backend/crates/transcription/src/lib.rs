pub mod buffer;
pub mod vad;
pub mod gemini;
pub mod groq;

use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use common::messages::{TranscriptSegment, WsEvent};
use common::rate_limiter::{RateLimiter, with_retry};
use common::providers::{is_quota_exhausted, is_rate_limit};
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
        if words.len() >= q_words.len() && q_words.iter().zip(words.iter()).all(|(a, b)| a == b) {
            return true;
        }
    }
    false
}

/// Try transcription providers in order, falling back on quota exhaustion.
async fn transcribe_with_fallback(
    gemini_key: &str,
    groq_key: Option<&str>,
    pcm: &[u8],
    rate_limiter: &RateLimiter,
) -> Result<String, anyhow::Error> {
    // 1. Try Gemini with retry for temporary rate limits
    let result = with_retry(rate_limiter, || {
        let k = gemini_key.to_string();
        let p = pcm.to_vec();
        async move { gemini::transcribe(&k, &p).await }
    })
    .await;

    match result {
        Ok(text) => return Ok(text),
        Err(e) if is_quota_exhausted(&e) => {
            tracing::warn!("Gemini transcription quota exhausted, falling back to Groq");
        }
        Err(e) => return Err(e),
    }

    // 2. Try Groq Whisper
    if let Some(key) = groq_key {
        match groq::transcribe(key, pcm).await {
            Ok(text) => return Ok(text),
            Err(e) if is_quota_exhausted(&e) || is_rate_limit(&e) => {
                tracing::warn!("Groq transcription also exhausted: {}", e);
            }
            Err(e) => return Err(e),
        }
    }

    anyhow::bail!("All transcription providers exhausted")
}

pub async fn run_agent(
    mut audio_rx: mpsc::Receiver<Vec<u8>>,
    question_tx: mpsc::Sender<String>,
    event_tx: broadcast::Sender<WsEvent>,
    transcript: Arc<RwLock<Vec<TranscriptSegment>>>,
    gemini_key: String,
    groq_key: Option<String>,
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
                    let gkey = gemini_key.clone();
                    let grkey = groq_key.clone();
                    let qtx = question_tx.clone();
                    let etx = event_tx.clone();
                    let tr = transcript.clone();
                    let rl = rate_limiter.clone();
                    tokio::spawn(async move {
                        match transcribe_with_fallback(&gkey, grkey.as_deref(), &segment_pcm, &rl).await {
                            Ok(text) if !text.trim().is_empty() => {
                                let ts = SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_millis() as u64;
                                let seg = TranscriptSegment { text: text.clone(), timestamp_ms: ts };
                                {
                                    let mut t = tr.write().await;
                                    t.push(seg);
                                    if t.len() > 100 { t.remove(0); }
                                }
                                let _ = etx.send(WsEvent::Transcript { text: text.clone(), timestamp_ms: ts });
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
