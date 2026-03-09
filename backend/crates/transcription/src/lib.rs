pub mod buffer;
pub mod vad;
pub mod gemini;
pub mod groq;
pub mod diarize;

use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use common::messages::{TranscriptSegment, WsEvent};
use common::rate_limiter::{RateLimiter, with_retry};
use common::providers::{is_quota_exhausted, is_rate_limit};
use std::time::{SystemTime, UNIX_EPOCH};

// ── Transcription provider chain ──────────────────────────────────────────────

async fn transcribe_with_fallback(
    gemini_key: &str,
    groq_key: Option<&str>,
    pcm: &[u8],
    rate_limiter: &RateLimiter,
) -> Result<String, anyhow::Error> {
    // 1. Groq Whisper — fast, free, separate quota from LLM
    if let Some(key) = groq_key {
        match groq::transcribe(key, pcm).await {
            Ok(text) => return Ok(text),
            Err(e) if is_quota_exhausted(&e) || is_rate_limit(&e) => {
                tracing::warn!("Groq transcription unavailable, falling back to Gemini");
            }
            Err(e) => return Err(e),
        }
    }

    // 2. Gemini — fallback; conserves vision quota for sentiment
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

// ── Heuristic speaker classifier (fallback when diarization unavailable) ──────

/// Layered heuristic for single-stream speaker classification.
/// Used when the pyannote sidecar is not running.
fn classify_speaker(text: &str, prev_speaker: &str, prev_word_count: usize) -> &'static str {
    let lower = text.trim().to_lowercase();
    let word_count = text.split_whitespace().count();

    // 1. Very short acknowledgements → Interviewer
    if word_count <= 6 {
        let acks = ["mm", "hmm", "right", "okay", "ok", "good", "great", "perfect",
                    "i see", "sure", "yes", "yep", "got it", "interesting",
                    "absolutely", "exactly", "fair enough", "makes sense"];
        for ack in &acks {
            if lower.contains(ack) { return "Interviewer"; }
        }
    }

    // 2. Strong candidate self-reference in longer segments → You
    if word_count > 15 {
        let candidate_signals = [
            "in my experience", "at my previous", "at my last", "in my last",
            "when i was at", "when i worked", "i've worked", "i worked on",
            "i've been", "i led", "i built", "i created", "i designed",
            "i implemented", "i managed", "i developed", "my approach",
            "my background", "my role", "my team", "my project",
        ];
        for sig in &candidate_signals { if lower.contains(sig) { return "You"; } }
    }

    // 3. Explicit interviewer-turn patterns
    if is_interviewer_turn(&lower, word_count) { return "Interviewer"; }

    // 4. Long segments → almost certainly the candidate
    if word_count > 60 { return "You"; }

    // 5. Alternation bias: short segment after a long You response → Interviewer
    if prev_speaker == "You" && prev_word_count > 40 && word_count < 50 {
        return "Interviewer";
    }

    // 6. Default
    "You"
}

fn is_interviewer_turn(lower: &str, word_count: usize) -> bool {
    if lower.trim_end().ends_with('?') { return true; }
    if word_count == 0 { return false; }

    let words: Vec<&str> = lower.split_whitespace().collect();
    let starters: &[&str] = &[
        "what", "why", "how", "when", "where", "who", "which",
        "can you", "could you", "would you", "have you", "do you",
        "did you", "are you", "were you", "will you",
        "tell me", "walk me", "walk us", "describe", "explain",
        "give me", "give us", "share", "show me", "talk me", "talk us",
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
        let sw: Vec<&str> = starter.split_whitespace().collect();
        if words.len() >= sw.len() && sw.iter().zip(words.iter()).all(|(a, b)| a == b) {
            return true;
        }
    }
    false
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── is_interviewer_turn ──────────────────────────────────────────────────

    #[test]
    fn question_mark_is_interviewer_turn() {
        assert!(is_interviewer_turn("so you've used kubernetes before?", 7));
        assert!(is_interviewer_turn("really?", 1));
    }

    #[test]
    fn question_word_starters() {
        for phrase in &[
            "what is your experience with rust",
            "why did you leave your last role",
            "how do you approach debugging",
            "when did you start coding",
            "where do you see yourself in five years",
            "who was your most influential mentor",
            "which approach would you prefer",
            "can you walk me through your background",
            "could you describe a challenge you faced",
            "would you be open to relocation",
            "have you led a team before",
            "do you have experience with kubernetes",
            "did you manage the full delivery",
            "are you comfortable with ambiguity",
            "were you the tech lead on that",
            "will you be available to start in january",
        ] {
            let wc = phrase.split_whitespace().count();
            assert!(is_interviewer_turn(phrase, wc), "failed: {phrase}");
        }
    }

    #[test]
    fn statement_question_starters() {
        for phrase in &[
            "tell me about yourself",
            "walk me through your last project",
            "describe your ideal team",
            "explain how you approach conflict",
            "share a time you failed",
            "i'd like to understand your background",
            "i'm curious about your decision to switch",
            "i noticed you worked at acme",
            "let's talk about your technical skills",
            "moving on to the next topic",
            "we're looking for someone who can lead",
            "so tell me more about that project",
            "so how did you handle that situation",
        ] {
            let wc = phrase.split_whitespace().count();
            assert!(is_interviewer_turn(phrase, wc), "failed: {phrase}");
        }
    }

    #[test]
    fn non_interviewer_turns() {
        for phrase in &[
            "that's a great point",
            "i completely agree with you",
            "thank you for the question",
            "let me think about that for a second",
        ] {
            let wc = phrase.split_whitespace().count();
            assert!(!is_interviewer_turn(phrase, wc), "false positive: {phrase}");
        }
    }

    // ── classify_speaker ─────────────────────────────────────────────────────

    #[test]
    fn short_acknowledgements_are_interviewer() {
        for text in &["Right", "I see", "Okay", "Good", "Mm-hmm", "Interesting"] {
            assert_eq!(classify_speaker(text, "You", 50), "Interviewer", "failed: {text}");
        }
    }

    #[test]
    fn candidate_self_reference_in_long_text_is_you() {
        let text = "In my experience building distributed systems at my previous company, \
                    I led a team of five engineers and we delivered the project on time.";
        assert_eq!(classify_speaker(text, "Interviewer", 0), "You");
    }

    #[test]
    fn long_segment_defaults_to_you() {
        let text = "word ".repeat(65); // 65 words
        assert_eq!(classify_speaker(text.trim(), "Interviewer", 10), "You");
    }

    #[test]
    fn explicit_question_classifies_as_interviewer() {
        assert_eq!(
            classify_speaker("Tell me about yourself.", "You", 80),
            "Interviewer"
        );
        assert_eq!(
            classify_speaker("What is your experience with Rust?", "You", 5),
            "Interviewer"
        );
    }

    #[test]
    fn alternation_bias_after_long_you_turn() {
        // Short ambiguous segment after a long You turn → Interviewer
        let short_ambiguous = "That makes sense. Let's move on.";
        assert_eq!(
            classify_speaker(short_ambiguous, "You", 80),
            "Interviewer"
        );
    }
}

// ── Agent ─────────────────────────────────────────────────────────────────────

pub async fn run_agent(
    mut audio_rx: mpsc::Receiver<Vec<u8>>,
    question_tx: mpsc::Sender<String>,
    event_tx: broadcast::Sender<WsEvent>,
    transcript: Arc<RwLock<Vec<TranscriptSegment>>>,
    gemini_key: String,
    groq_key: Option<String>,
    diarize_url: Option<String>,
    rate_limiter: RateLimiter,
) {
    let mut ring_buf = buffer::RingBuffer::new();
    let speaker_tracker: diarize::SharedTracker =
        Arc::new(std::sync::Mutex::new(diarize::SpeakerTracker::default()));

    // Heuristic fallback state
    let mut prev_speaker = String::from("You");
    let mut prev_word_count: usize = 0;

    loop {
        match audio_rx.recv().await {
            Some(pcm_chunk) => {
                ring_buf.push(&pcm_chunk);
                if ring_buf.should_flush() {
                    let segment_pcm = ring_buf.drain_segment();
                    if segment_pcm.is_empty() { continue; }

                    let gkey = gemini_key.clone();
                    let grkey = groq_key.clone();
                    let durl = diarize_url.clone();
                    let qtx = question_tx.clone();
                    let etx = event_tx.clone();
                    let tr = transcript.clone();
                    let rl = rate_limiter.clone();
                    let tracker = speaker_tracker.clone();
                    let ps = prev_speaker.clone();
                    let pwc = prev_word_count;

                    tokio::spawn(async move {
                        // Build WAV bytes once — used for diarization
                        let wav_for_diarize = diarize::pcm_to_wav(&segment_pcm).ok();

                        // Run transcription and diarization concurrently
                        let (transcription_result, diarization_result) = tokio::join!(
                            transcribe_with_fallback(&gkey, grkey.as_deref(), &segment_pcm, &rl),
                            async {
                                match (durl.as_deref(), wav_for_diarize) {
                                    (Some(url), Some(wav)) => {
                                        match diarize::diarize(url, wav).await {
                                            Ok(segs) => Some(segs),
                                            Err(e) => {
                                                tracing::debug!("Diarization unavailable: {}", e);
                                                None
                                            }
                                        }
                                    }
                                    _ => None,
                                }
                            }
                        );

                        let text = match transcription_result {
                            Ok(t) if !t.trim().is_empty() => t,
                            Ok(_) => return,
                            Err(e) => {
                                tracing::error!("Transcription error: {}", e);
                                let _ = etx.send(WsEvent::Error {
                                    message: format!("Transcription error: {}", e),
                                });
                                return;
                            }
                        };

                        let ts = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64;

                        let wc = text.split_whitespace().count();

                        // Determine speaker: prefer pyannote, fall back to heuristics
                        let speaker = if let Some(segs) = diarization_result {
                            let mut t = tracker.lock().unwrap();
                            t.record(&segs);
                            if let Some(dominant) = diarize::dominant_speaker(&segs) {
                                t.role(&dominant).to_string()
                            } else {
                                classify_speaker(&text, &ps, pwc).to_string()
                            }
                        } else {
                            classify_speaker(&text, &ps, pwc).to_string()
                        };

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
                        if speaker == "Interviewer" && wc > 5 {
                            let _ = qtx.send(text).await;
                        }
                    });

                    // Update heuristic context from last stored segment (best-effort)
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
