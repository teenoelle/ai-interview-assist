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

/// Classify whether a transcript segment belongs to the interviewer.
/// Uses a layered heuristic since there is only one audio stream:
///   1. Acknowledgement phrases (short) → Interviewer
///   2. Candidate self-reference language in long segments → You
///   3. Interviewer-turn patterns (questions + statement-questions) → Interviewer
///   4. Long segments (>60 words) → You
///   5. Alternation bias: short segment after a long You turn → Interviewer
///   6. Default → You
fn classify_speaker(text: &str, prev_speaker: &str, prev_word_count: usize) -> &'static str {
    let text = text.trim();
    let lower = text.to_lowercase();
    let word_count = text.split_whitespace().count();

    // 1. Very short acknowledgements are almost always the interviewer nodding along
    if word_count <= 6 {
        let acks = ["mm", "hmm", "right", "okay", "ok", "good", "great", "perfect",
                    "i see", "sure", "yes", "yep", "got it", "interesting",
                    "absolutely", "exactly", "fair enough", "makes sense"];
        for ack in &acks {
            if lower.contains(ack) {
                return "Interviewer";
            }
        }
    }

    // 2. Strong candidate self-reference in a medium+ length segment → You
    if word_count > 15 {
        let candidate_signals = [
            "in my experience", "at my previous", "at my last", "in my last",
            "when i was at", "when i worked", "i've worked", "i worked on",
            "i've been", "i led", "i built", "i created", "i designed",
            "i implemented", "i managed", "i developed", "my approach",
            "my background", "my role", "my team", "my project",
        ];
        for sig in &candidate_signals {
            if lower.contains(sig) {
                return "You";
            }
        }
    }

    // 3. Interviewer-turn patterns: questions AND statement-questions
    if is_interviewer_turn(&lower, word_count) {
        return "Interviewer";
    }

    // 4. Long responses (>60 words) are almost always the candidate
    if word_count > 60 {
        return "You";
    }

    // 5. Alternation bias: a short-to-medium segment following a long You response
    //    is likely the interviewer moving to the next topic
    if prev_speaker == "You" && prev_word_count > 40 && word_count < 50 {
        return "Interviewer";
    }

    // 6. Default: candidate (in interviews, candidates produce the majority of words)
    "You"
}

/// Returns true when this segment is an interviewer turn that warrants a response.
/// Covers explicit questions AND statement-questions (interviewer statements that
/// implicitly invite the candidate to elaborate).
fn is_interviewer_turn(lower: &str, word_count: usize) -> bool {
    // Direct question mark
    if lower.trim_end().ends_with('?') {
        return true;
    }

    if word_count == 0 {
        return false;
    }

    let words: Vec<&str> = lower.split_whitespace().collect();

    // Patterns that start an interviewer turn (question starters + statement-questions)
    let starters: &[&str] = &[
        // Classic question words
        "what", "why", "how", "when", "where", "who", "which",
        // Polite question forms
        "can you", "could you", "would you", "have you", "do you",
        "did you", "are you", "were you", "will you",
        // Invitations / directives (interviewer-style)
        "tell me", "walk me", "walk us", "describe", "explain",
        "give me", "give us", "share", "show me",
        "talk me", "talk us",
        // Statement-questions: interviewer statements that invite response
        "i'd like to", "i'd love to", "i want to hear", "i'm curious",
        "i'm interested", "i noticed", "i saw that", "i see that",
        "one thing i", "one area i",
        "let's talk", "let's discuss", "let's move", "let's explore",
        "moving on", "next question", "next i'd",
        "we'd like", "we're looking", "we're interested",
        "the role requires", "the team is", "the position",
        "so tell", "so walk", "so describe", "so explain",
        "so how", "so what", "so why",
    ];

    for starter in starters {
        let s_words: Vec<&str> = starter.split_whitespace().collect();
        if words.len() >= s_words.len() {
            if s_words.iter().zip(words.iter()).all(|(a, b)| a == b) {
                return true;
            }
        }
    }

    false
}

/// Try transcription providers in order, falling back on quota exhaustion.
/// Groq Whisper runs first to preserve Gemini credits for sentiment analysis
/// (vision-only feature that has no free alternative).
async fn transcribe_with_fallback(
    gemini_key: &str,
    groq_key: Option<&str>,
    pcm: &[u8],
    rate_limiter: &RateLimiter,
) -> Result<String, anyhow::Error> {
    // 1. Groq Whisper — free, fast, no vision quota used
    if let Some(key) = groq_key {
        match groq::transcribe(key, pcm).await {
            Ok(text) => return Ok(text),
            Err(e) if is_quota_exhausted(&e) || is_rate_limit(&e) => {
                tracing::warn!("Groq transcription exhausted, falling back to Gemini");
            }
            Err(e) => return Err(e),
        }
    }

    // 2. Gemini — fallback only, preserves vision quota for sentiment
    let result = with_retry(rate_limiter, || {
        let k = gemini_key.to_string();
        let p = pcm.to_vec();
        async move { gemini::transcribe(&k, &p).await }
    })
    .await;

    match result {
        Ok(text) => return Ok(text),
        Err(e) if is_quota_exhausted(&e) => {
            tracing::warn!("Gemini transcription quota exhausted");
        }
        Err(e) => return Err(e),
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
    // Track previous speaker for alternation bias
    let mut prev_speaker = String::from("You");
    let mut prev_word_count: usize = 0;

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
                    let ps = prev_speaker.clone();
                    let pwc = prev_word_count;
                    tokio::spawn(async move {
                        match transcribe_with_fallback(&gkey, grkey.as_deref(), &segment_pcm, &rl).await {
                            Ok(text) if !text.trim().is_empty() => {
                                let ts = SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_millis() as u64;
                                let wc = text.split_whitespace().count();
                                let speaker = classify_speaker(&text, &ps, pwc).to_string();
                                let seg = TranscriptSegment {
                                    text: text.clone(),
                                    timestamp_ms: ts,
                                    speaker: speaker.clone(),
                                };
                                {
                                    let mut t = tr.write().await;
                                    t.push(seg);
                                    if t.len() > 100 { t.remove(0); }
                                }
                                let _ = etx.send(WsEvent::Transcript {
                                    text: text.clone(),
                                    timestamp_ms: ts,
                                    speaker: speaker.clone(),
                                });
                                // Trigger suggestions for any substantive interviewer turn
                                // (> 5 words — filters out bare acknowledgements)
                                if speaker == "Interviewer" && wc > 5 {
                                    let _ = qtx.send(text).await;
                                }
                                // Note: prev_speaker/prev_word_count updates are best-effort;
                                // since these spawns run concurrently the ordering is approximate,
                                // which is acceptable for a heuristic classifier.
                                drop(wc);
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

                    // Update context for next segment (before spawn completes, best-effort)
                    // We read the last transcript entry for prev state
                    {
                        let t = transcript.read().await;
                        if let Some(last) = t.last() {
                            prev_speaker = last.speaker.clone();
                            prev_word_count = last.text.split_whitespace().count();
                        }
                    }
                }
            }
            None => break,
        }
    }
}
