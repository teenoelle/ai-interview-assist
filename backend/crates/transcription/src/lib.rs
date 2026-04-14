pub mod buffer;
pub mod vad;
pub mod gemini;
pub mod groq;
pub mod deepgram;
pub mod diarize;

use std::sync::{Arc, OnceLock};
use tokio::sync::{broadcast, mpsc, RwLock};
use common::messages::{TranscriptSegment, WsEvent};
use common::rate_limiter::{RateLimiter, with_retry};
use common::providers::{is_quota_exhausted, is_rate_limit};
use common::circuit_breaker::CircuitBreaker;
use std::time::{SystemTime, UNIX_EPOCH};

static WHISPER_CB: OnceLock<CircuitBreaker> = OnceLock::new();
static GROQ_KEY1_CB: OnceLock<CircuitBreaker> = OnceLock::new();

fn whisper_cb() -> &'static CircuitBreaker {
    WHISPER_CB.get_or_init(|| CircuitBreaker::new("local-whisper", 6, 120))
}

fn groq_key1_cb() -> &'static CircuitBreaker {
    GROQ_KEY1_CB.get_or_init(|| CircuitBreaker::new("groq-key-1", 5, 900))
}

type CallCounts = Arc<std::sync::Mutex<std::collections::HashMap<String, u64>>>;

fn inc(counts: &Option<CallCounts>, name: &str) {
    if let Some(map) = counts {
        if let Ok(mut m) = map.lock() {
            *m.entry(name.to_string()).or_insert(0) += 1;
        }
    }
}

// ── Transcription provider chain ──────────────────────────────────────────────

async fn transcribe_with_fallback(
    gemini_key: &str,
    groq_key: Option<&str>,
    groq_key_2: Option<&str>,
    deepgram_key: Option<&str>,
    whisper_url: Option<&str>,
    whisper_model: &str,
    pcm: &[u8],
    rate_limiter: &RateLimiter,
    call_counts: &Option<CallCounts>,
    event_tx: &broadcast::Sender<WsEvent>,
) -> Result<String, anyhow::Error> {
    // 1. Local Whisper — completely free, no quota; silently skip if not running or circuit open
    if let Some(url) = whisper_url {
        if whisper_cb().is_open() {
            tracing::debug!("Local Whisper circuit open ({} failures) — skipping", whisper_cb().failure_count());
        } else {
            let endpoint = format!("{}/v1/audio/transcriptions", url.trim_end_matches('/'));
            match groq::transcribe_openai_asr(&endpoint, "", whisper_model, pcm, 30).await {
                Ok(text) => {
                    whisper_cb().record_success();
                    inc(call_counts, "Whisper (local)");
                    tracing::info!("transcription ✓ Whisper (local) — {} chars", text.len());
                    let _ = event_tx.send(WsEvent::ProviderUsed { service: "transcription".to_string(), provider: "Whisper (local)".to_string(), local: true });
                    return Ok(text);
                }
                Err(e) => {
                    whisper_cb().record_failure();
                    tracing::warn!("Local Whisper failed (failure #{}): {}", whisper_cb().failure_count(), e);
                }
            }
        }
    }

    // 2. Deepgram Nova-2 — 200 hours/month free, no RPM limit; most reliable API option
    if let Some(key) = deepgram_key {
        match deepgram::transcribe(key, pcm).await {
            Ok(text) => {
                inc(call_counts, "Deepgram");
                tracing::info!("transcription ✓ Deepgram — {} chars", text.len());
                let _ = event_tx.send(WsEvent::ProviderUsed { service: "transcription".to_string(), provider: "Deepgram".to_string(), local: false });
                return Ok(text);
            }
            Err(e) if is_quota_exhausted(&e) || is_rate_limit(&e) => {
                tracing::warn!("Deepgram quota/rate-limit, trying Groq");
            }
            Err(e) => {
                tracing::warn!("Deepgram error, trying Groq: {}", e);
            }
        }
    }

    // 3. Groq Whisper key 2 — fallback when Deepgram unavailable
    if let Some(key) = groq_key_2 {
        match groq::transcribe(key, pcm).await {
            Ok(text) => {
                inc(call_counts, "Groq Whisper #2");
                tracing::info!("transcription ✓ Groq Whisper #2 — {} chars", text.len());
                let _ = event_tx.send(WsEvent::ProviderUsed { service: "transcription".to_string(), provider: "Groq Whisper #2".to_string(), local: false });
                return Ok(text);
            }
            Err(e) if is_quota_exhausted(&e) || is_rate_limit(&e) => {
                tracing::warn!("Groq key 2 rate-limit/quota, trying key 1");
            }
            Err(e) => {
                tracing::warn!("Groq key 2 error, trying key 1: {}", e);
            }
        }
    }

    // 3. Groq Whisper key 1 — circuit breaker: 5 consecutive rate-limits → skip 15min
    if let Some(key) = groq_key {
        if groq_key1_cb().is_open() {
            tracing::debug!("Groq key 1 circuit open ({} failures) — skipping", groq_key1_cb().failure_count());
        } else {
            match groq::transcribe(key, pcm).await {
                Ok(text) => {
                    groq_key1_cb().record_success();
                    inc(call_counts, "Groq Whisper");
                    tracing::info!("transcription ✓ Groq Whisper — {} chars", text.len());
                    let _ = event_tx.send(WsEvent::ProviderUsed { service: "transcription".to_string(), provider: "Groq Whisper".to_string(), local: false });
                    return Ok(text);
                }
                Err(e) if is_quota_exhausted(&e) || is_rate_limit(&e) => {
                    groq_key1_cb().record_failure();
                    tracing::warn!("Groq key 1 rate-limit/quota (failure #{}), falling back to Gemini", groq_key1_cb().failure_count());
                }
                Err(e) => {
                    groq_key1_cb().record_failure();
                    tracing::warn!("Groq key 1 error (failure #{}), falling back to Gemini: {}", groq_key1_cb().failure_count(), e);
                }
            }
        }
    }

    // 5. Gemini — final fallback; conserves vision quota for sentiment
    let result = with_retry(rate_limiter, || {
        let k = gemini_key.to_string();
        let p = pcm.to_vec();
        async move { gemini::transcribe(&k, &p).await }
    })
    .await;

    match result {
        Ok(text) => {
            inc(call_counts, "Gemini Transcription");
            tracing::info!("transcription ✓ Gemini — {} chars", text.len());
            let _ = event_tx.send(WsEvent::ProviderUsed { service: "transcription".to_string(), provider: "Gemini".to_string(), local: false });
            return Ok(text);
        }
        Err(e) if is_quota_exhausted(&e) => {
            tracing::warn!("Gemini transcription quota exhausted");
        }
        Err(e) => return Err(e),
    }

    anyhow::bail!("All transcription providers exhausted")
}

// ── Heuristic speaker classifier (fallback when diarization unavailable) ──────

/// Layered heuristic speaker classifier.
///
/// `default_speaker`: the assumed speaker when no strong signal is found.
/// - `"Interviewer"` for the system-audio stream (speakers output = other person)
/// - `"You"` for the mic stream (but mic agent never calls this — always labels "You")
fn classify_speaker(text: &str, prev_speaker: &str, prev_word_count: usize, default_speaker: &'static str) -> &'static str {
    let lower = text.trim().to_lowercase();
    let word_count = text.split_whitespace().count();

    // 1. Strong candidate self-reference → You (works regardless of stream)
    if word_count > 10 {
        let candidate_signals = [
            "in my experience", "at my previous", "at my last", "in my last",
            "when i was at", "when i worked", "i've worked", "i worked on",
            "i've been", "i led", "i built", "i created", "i designed",
            "i implemented", "i managed", "i developed", "my approach",
            "my background", "my role", "my team", "my project",
        ];
        for sig in &candidate_signals { if lower.contains(sig) { return "You"; } }
    }

    // 2. Explicit interviewer-turn patterns → Interviewer
    if is_interviewer_turn(&lower, word_count) { return "Interviewer"; }

    // 3. Very short acknowledgements → Interviewer
    if word_count <= 6 {
        let acks = ["mm", "hmm", "right", "okay", "ok", "good", "great", "perfect",
                    "i see", "sure", "yes", "yep", "got it", "interesting",
                    "absolutely", "exactly", "fair enough", "makes sense"];
        for ack in &acks {
            if lower.contains(ack) { return "Interviewer"; }
        }
    }

    // 4. Very long answer-style response → You
    //    (only flip default when clearly a long monologue response)
    if default_speaker == "Interviewer" && word_count > 80 { return "You"; }

    // 5. Alternation bias: short segment after a long You response → Interviewer
    if prev_speaker == "You" && prev_word_count > 40 && word_count < 40 {
        return "Interviewer";
    }

    // 6. Default (stream-dependent)
    default_speaker
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
            assert_eq!(classify_speaker(text, "You", 50, "Interviewer"), "Interviewer", "failed: {text}");
        }
    }

    #[test]
    fn candidate_self_reference_in_long_text_is_you() {
        let text = "In my experience building distributed systems at my previous company, \
                    I led a team of five engineers and we delivered the project on time.";
        assert_eq!(classify_speaker(text, "Interviewer", 0, "Interviewer"), "You");
    }

    #[test]
    fn very_long_monologue_on_system_audio_is_you() {
        // >80 words on system audio stream → must be the candidate answering at length
        let text = "word ".repeat(85);
        assert_eq!(classify_speaker(text.trim(), "Interviewer", 10, "Interviewer"), "You");
    }

    #[test]
    fn explicit_question_classifies_as_interviewer() {
        assert_eq!(
            classify_speaker("Tell me about yourself.", "You", 80, "Interviewer"),
            "Interviewer"
        );
        assert_eq!(
            classify_speaker("What is your experience with Rust?", "You", 5, "Interviewer"),
            "Interviewer"
        );
    }

    #[test]
    fn alternation_bias_after_long_you_turn() {
        // Short ambiguous segment after a long You turn → Interviewer
        let short_ambiguous = "That makes sense. Let's move on.";
        assert_eq!(
            classify_speaker(short_ambiguous, "You", 80, "Interviewer"),
            "Interviewer"
        );
    }
}

// ── Mic agent (always "You") ──────────────────────────────────────────────────

/// Transcribes microphone audio and labels every segment as "You".
/// Never triggers suggestion generation — the candidate's own speech is not a prompt.
pub async fn run_mic_agent(
    mut audio_rx: mpsc::Receiver<Vec<u8>>,
    event_tx: broadcast::Sender<WsEvent>,
    transcript: Arc<RwLock<Vec<TranscriptSegment>>>,
    gemini_key: String,
    groq_key: Option<String>,
    groq_key_2: Option<String>,
    deepgram_key: Option<String>,
    whisper_url: Option<String>,
    whisper_model: String,
    rate_limiter: RateLimiter,
    call_counts: Option<CallCounts>,
) {
    let mut ring_buf = buffer::RingBuffer::new();
    let mut mic_chunks_received: u64 = 0;

    loop {
        match audio_rx.recv().await {
            Some(chunk) => {
                mic_chunks_received += 1;
                if mic_chunks_received == 1 {
                    tracing::info!("mic: first audio chunk received ({} bytes)", chunk.len());
                }
                ring_buf.push(&chunk);
                if ring_buf.should_flush() {
                    if !ring_buf.has_speech() {
                        tracing::debug!("mic: silent segment discarded ({:.1}s, peak energy {:.0}, threshold 5)",
                            ring_buf.duration_secs(), ring_buf.peak_energy);
                        ring_buf.drain_segment();
                        continue;
                    }
                    let pcm = ring_buf.drain_segment();
                    if pcm.is_empty() { continue; }
                    tracing::info!("mic: sending {:.1}s segment to transcription", pcm.len() as f32 / (16000.0 * 2.0));

                    let gkey = gemini_key.clone();
                    let grkey = groq_key.clone();
                    let grkey2 = groq_key_2.clone();
                    let dgkey = deepgram_key.clone();
                    let wurl = whisper_url.clone();
                    let wmodel = whisper_model.clone();
                    let etx = event_tx.clone();
                    let tr = transcript.clone();
                    let rl = rate_limiter.clone();
                    let cc = call_counts.clone();

                    tokio::spawn(async move {
                        match transcribe_with_fallback(&gkey, grkey.as_deref(), grkey2.as_deref(), dgkey.as_deref(), wurl.as_deref(), &wmodel, &pcm, &rl, &cc, &etx).await {
                            Ok(text) if !text.trim().is_empty() => {
                                let ts = SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_millis() as u64;
                                let seg = TranscriptSegment {
                                    text: text.clone(),
                                    timestamp_ms: ts,
                                    speaker: "You".to_string(),
                                };
                                {
                                    let mut t = tr.write().await;
                                    t.push(seg);
                                    if t.len() > 100 { t.remove(0); }
                                }
                                let _ = etx.send(WsEvent::Transcript {
                                    text,
                                    timestamp_ms: ts,
                                    speaker: "You".to_string(),
                                });
                            }
                            Ok(_) => {}
                            Err(e) => {
                                tracing::error!("Mic transcription error: {}", e);
                                let _ = etx.send(WsEvent::Error {
                                    message: format!("Transcription error: {}", e),
                                });
                            }
                        }
                    });
                }
            }
            None => {
                tracing::warn!("mic: audio channel closed after {} chunks — WebSocket disconnected?", mic_chunks_received);
                break;
            }
        }
    }
}

// ── System audio agent ────────────────────────────────────────────────────────

pub async fn run_agent(
    mut audio_rx: mpsc::Receiver<Vec<u8>>,
    question_tx: mpsc::Sender<String>,
    event_tx: broadcast::Sender<WsEvent>,
    transcript: Arc<RwLock<Vec<TranscriptSegment>>>,
    gemini_key: String,
    groq_key: Option<String>,
    groq_key_2: Option<String>,
    deepgram_key: Option<String>,
    whisper_url: Option<String>,
    whisper_model: String,
    diarize_url: Option<String>,
    rate_limiter: RateLimiter,
    call_counts: Option<CallCounts>,
) {
    let mut ring_buf = buffer::RingBuffer::new();
    let speaker_tracker: diarize::SharedTracker =
        Arc::new(std::sync::Mutex::new(diarize::SpeakerTracker::default()));

    // Heuristic fallback state
    let mut prev_speaker = String::from("You");
    let mut prev_word_count: usize = 0;
    let mut sys_chunks_received: u64 = 0;

    loop {
        match audio_rx.recv().await {
            Some(pcm_chunk) => {
                sys_chunks_received += 1;
                if sys_chunks_received == 1 {
                    tracing::info!("system: first audio chunk received ({} bytes)", pcm_chunk.len());
                }
                ring_buf.push(&pcm_chunk);
                if ring_buf.should_flush() {
                    if !ring_buf.has_speech() {
                        tracing::debug!("system: silent segment discarded ({:.1}s, peak energy {:.0}, threshold 5)",
                            ring_buf.duration_secs(), ring_buf.peak_energy);
                        ring_buf.drain_segment();
                        continue;
                    }
                    let segment_pcm = ring_buf.drain_segment();
                    if segment_pcm.is_empty() { continue; }
                    tracing::info!("system: sending {:.1}s segment to transcription", segment_pcm.len() as f32 / (16000.0 * 2.0));

                    let gkey = gemini_key.clone();
                    let grkey = groq_key.clone();
                    let grkey2 = groq_key_2.clone();
                    let dgkey = deepgram_key.clone();
                    let wurl = whisper_url.clone();
                    let wmodel = whisper_model.clone();
                    let durl = diarize_url.clone();
                    let qtx = question_tx.clone();
                    let etx = event_tx.clone();
                    let tr = transcript.clone();
                    let rl = rate_limiter.clone();
                    let tracker = speaker_tracker.clone();
                    let ps = prev_speaker.clone();
                    let pwc = prev_word_count;
                    let cc = call_counts.clone();

                    tokio::spawn(async move {
                        // Build WAV bytes once — used for diarization
                        let wav_for_diarize = diarize::pcm_to_wav(&segment_pcm).ok();

                        // Run transcription and diarization concurrently
                        let (transcription_result, diarization_result) = tokio::join!(
                            transcribe_with_fallback(&gkey, grkey.as_deref(), grkey2.as_deref(), dgkey.as_deref(), wurl.as_deref(), &wmodel, &segment_pcm, &rl, &cc, &etx),
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
                                classify_speaker(&text, &ps, pwc, "Interviewer").to_string()
                            }
                        } else {
                            classify_speaker(&text, &ps, pwc, "Interviewer").to_string()
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
                        if speaker == "Interviewer" && wc > 2 {
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
            None => {
                tracing::warn!("system: audio channel closed after {} chunks — WebSocket disconnected?", sys_chunks_received);
                break;
            }
        }
    }
}
