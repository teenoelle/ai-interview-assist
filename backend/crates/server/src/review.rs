use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::process::Command;
use tokio::sync::watch;
use common::messages::TranscriptSegment;
use context::ai_helper::{AiConfig, generate_answer_feedback, AnswerFeedbackResult};

// ── Data types ────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
pub struct ReviewReport {
    pub id: String,
    pub created_at: u64,
    pub duration_secs: f64,
    pub source_filename: String,
    pub source_type: String, // "upload" | "live"
    pub transcript: Vec<ReviewSegment>,
    pub qa_pairs: Vec<QaPair>,
    pub vocal_summary: VocalSummary,
    pub speaker_summary: SpeakerSummary,
    pub keywords_mentioned: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ReviewSegment {
    pub speaker: String,
    pub text: String,
    pub start_ms: u64,
    pub end_ms: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct QaPair {
    pub question: String,
    pub answer_text: String,
    pub coaching: String,
    pub missed_followup: bool,
    pub missed_metric: bool,
    pub wpm: u32,
    pub duration_secs: f32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct VocalSummary {
    pub avg_wpm: u32,
    pub total_answers: usize,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SpeakerSummary {
    pub you_pct: f32,
    pub them_pct: f32,
    pub you_word_count: u32,
    pub them_word_count: u32,
    pub turn_count: usize,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ReviewProgress {
    pub pct: u8,
    pub step: String,
    pub done: bool,
    pub error: Option<String>,
}

// ── Config ────────────────────────────────────────────────────────────────────

pub struct ReviewConfig {
    pub gemini_key: String,
    pub anthropic_key: Option<String>,
    pub groq_key: Option<String>,
    pub groq_key_2: Option<String>,
    pub ollama_url: String,
    pub ollama_model: String,
    pub whisper_url: Option<String>,
    pub whisper_model: String,
    pub diarize_url: Option<String>,
    pub keywords: Vec<String>,
    pub reviews_dir: PathBuf,
}

// ── Utilities ─────────────────────────────────────────────────────────────────

pub fn new_id() -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let h = ts.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    format!("{:016x}{:016x}", ts as u64, h as u64)
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn send_progress(tx: &watch::Sender<ReviewProgress>, pct: u8, step: &str) {
    let _ = tx.send(ReviewProgress { pct, step: step.to_string(), done: false, error: None });
}

// ── Disk I/O ──────────────────────────────────────────────────────────────────

pub fn review_dir(reviews_dir: &Path, id: &str) -> PathBuf {
    reviews_dir.join(id)
}

pub async fn save_report(reviews_dir: &Path, report: &ReviewReport) -> Result<()> {
    let dir = review_dir(reviews_dir, &report.id);
    tokio::fs::create_dir_all(&dir).await?;
    let json = serde_json::to_string_pretty(report)?;
    tokio::fs::write(dir.join("report.json"), json).await?;
    Ok(())
}

pub async fn load_report(reviews_dir: &Path, id: &str) -> Result<ReviewReport> {
    let path = review_dir(reviews_dir, id).join("report.json");
    let json = tokio::fs::read_to_string(path).await?;
    Ok(serde_json::from_str(&json)?)
}

pub async fn list_reports(reviews_dir: &Path) -> Vec<ReviewReport> {
    let mut reports = Vec::new();
    let Ok(mut rd) = tokio::fs::read_dir(reviews_dir).await else { return reports; };
    while let Ok(Some(entry)) = rd.next_entry().await {
        let dir = entry.path();
        let report_path = dir.join("report.json");
        let processing_path = dir.join("processing.json");
        if let Ok(json) = tokio::fs::read_to_string(&report_path).await {
            if let Ok(r) = serde_json::from_str::<ReviewReport>(&json) {
                reports.push(r);
            }
        } else if tokio::fs::metadata(&processing_path).await.is_ok() {
            // processing.json exists but no report.json = crashed/interrupted; clean up
            tracing::warn!("Removing interrupted review at {:?}", dir);
            let _ = tokio::fs::remove_dir_all(&dir).await;
        }
    }
    reports.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    reports
}

pub async fn delete_review(reviews_dir: &Path, id: &str) -> Result<()> {
    tokio::fs::remove_dir_all(review_dir(reviews_dir, id)).await?;
    Ok(())
}

pub async fn delete_all_reviews(reviews_dir: &Path) -> Result<()> {
    let Ok(mut rd) = tokio::fs::read_dir(reviews_dir).await else { return Ok(()); };
    while let Ok(Some(entry)) = rd.next_entry().await {
        if entry.path().is_dir() {
            let _ = tokio::fs::remove_dir_all(entry.path()).await;
        }
    }
    Ok(())
}

// ── Audio channel detection ───────────────────────────────────────────────────

enum AudioLayout { Mono, StereoMixed, StereoSeparate }

async fn detect_channels(path: &Path) -> AudioLayout {
    let out = Command::new("ffprobe")
        .args(["-v", "quiet", "-print_format", "json", "-show_streams", "-select_streams", "a:0"])
        .arg(path)
        .output().await;
    let Ok(out) = out else { return AudioLayout::Mono; };
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or_default();
    let channels = json["streams"][0]["channels"].as_u64().unwrap_or(1);
    if channels < 2 { return AudioLayout::Mono; }

    // Compare RMS of left vs right channel over first 30s
    let left = channel_rms(path, "FL", 30).await.unwrap_or(0.0);
    let right = channel_rms(path, "FR", 30).await.unwrap_or(0.0);
    if left == 0.0 || right == 0.0 { return AudioLayout::Mono; }
    let ratio = (left / right).max(right / left);
    // > 4.0 (~12 dB) = separate tracks; otherwise it's a mixed stereo downmix
    if ratio > 4.0 { AudioLayout::StereoSeparate } else { AudioLayout::StereoMixed }
}

async fn channel_rms(path: &Path, channel: &str, secs: u32) -> Option<f64> {
    let filter = format!("pan=mono|c0={},volumedetect", channel);
    let out = Command::new("ffmpeg")
        .args(["-t", &secs.to_string(), "-i"]).arg(path)
        .args(["-af", &filter, "-f", "null", "-"])
        .output().await.ok()?;
    let stderr = String::from_utf8_lossy(&out.stderr);
    for line in stderr.lines() {
        if line.contains("mean_volume:") {
            if let Some(v) = line.split(':').nth(1) {
                if let Ok(db) = v.trim().trim_end_matches(" dBFS").parse::<f64>() {
                    return Some(10f64.powf(db / 20.0));
                }
            }
        }
    }
    None
}

// ── Audio extraction ──────────────────────────────────────────────────────────

/// Extract audio as raw 16-bit LE PCM at 16 kHz mono.
async fn extract_pcm(src: &Path, dst: &Path) -> Result<()> {
    let status = Command::new("ffmpeg")
        .args(["-y", "-i"]).arg(src)
        .args(["-vn", "-ac", "1", "-ar", "16000", "-f", "s16le"])
        .arg(dst)
        .status().await?;
    if !status.success() { return Err(anyhow!("ffmpeg PCM extraction failed")); }
    Ok(())
}

/// Extract a single stereo channel as raw PCM.
async fn extract_channel_pcm(src: &Path, dst: &Path, channel: &str) -> Result<()> {
    let filter = format!("pan=mono|c0={}", channel);
    let status = Command::new("ffmpeg")
        .args(["-y", "-i"]).arg(src)
        .args(["-af", &filter, "-ar", "16000", "-f", "s16le"])
        .arg(dst)
        .status().await?;
    if !status.success() { return Err(anyhow!("ffmpeg channel extraction failed")); }
    Ok(())
}

/// Get duration in seconds via ffprobe.
async fn probe_duration(path: &Path) -> f64 {
    let out = Command::new("ffprobe")
        .args(["-v", "quiet", "-print_format", "json", "-show_format"])
        .arg(path)
        .output().await;
    let Ok(out) = out else { return 0.0; };
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or_default();
    json["format"]["duration"].as_str()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0)
}

// ── Chunking ──────────────────────────────────────────────────────────────────

const CHUNK_SECS: u64 = 480;       // 8 min per chunk
const OVERLAP_SECS: u64 = 15;      // 15 s overlap to avoid cutting mid-sentence
const MAX_DIRECT_SECS: f64 = 720.0; // ≤ 12 min → send whole file directly (~23 MB)

struct AudioChunk { path: PathBuf, start_ms: u64 }

async fn make_chunks(pcm: &Path, work_dir: &Path, duration: f64) -> Result<Vec<AudioChunk>> {
    if duration <= MAX_DIRECT_SECS {
        return Ok(vec![AudioChunk { path: pcm.to_path_buf(), start_ms: 0 }]);
    }
    let mut chunks = Vec::new();
    let mut start: u64 = 0;
    let dur = duration as u64;
    let mut idx = 0u32;
    while start < dur {
        let len = CHUNK_SECS.min(dur.saturating_sub(start));
        let out = work_dir.join(format!("chunk_{:03}.pcm", idx));
        // Byte offsets for 16-bit 16kHz mono: 32000 bytes/s
        let byte_start = start * 32000;
        let byte_len = len * 32000;
        let all = tokio::fs::read(pcm).await.unwrap_or_default();
        let end = (byte_start + byte_len).min(all.len() as u64) as usize;
        if byte_start as usize >= all.len() { break; }
        tokio::fs::write(&out, &all[byte_start as usize..end]).await?;
        chunks.push(AudioChunk { path: out, start_ms: start * 1000 });
        if start + CHUNK_SECS >= dur { break; }
        start += CHUNK_SECS - OVERLAP_SECS;
        idx += 1;
    }
    Ok(chunks)
}

// ── Transcription ─────────────────────────────────────────────────────────────

async fn transcribe_pcm(pcm: &[u8], cfg: &ReviewConfig) -> String {
    if let Some(url) = &cfg.whisper_url {
        let endpoint = format!("{}/v1/audio/transcriptions", url.trim_end_matches('/'));
        if let Ok(t) = transcription::groq::transcribe_openai_asr(&endpoint, "", &cfg.whisper_model, pcm).await {
            if !t.trim().is_empty() { return t; }
        }
    }
    if let Some(key) = &cfg.groq_key {
        if let Ok(t) = transcription::groq::transcribe(key, pcm).await {
            if !t.trim().is_empty() { return t; }
        }
    }
    if let Some(key) = &cfg.groq_key_2 {
        if let Ok(t) = transcription::groq::transcribe(key, pcm).await {
            if !t.trim().is_empty() { return t; }
        }
    }
    transcription::gemini::transcribe(&cfg.gemini_key, pcm).await.unwrap_or_default()
}

// ── Diarization ───────────────────────────────────────────────────────────────

async fn diarize_pcm(pcm: &[u8], cfg: &ReviewConfig) -> Option<Vec<transcription::diarize::Segment>> {
    let url = cfg.diarize_url.as_deref()?;
    let wav = transcription::diarize::pcm_to_wav(pcm).ok()?;
    transcription::diarize::diarize(url, wav).await.ok()
}

// ── Speaker heuristics (fallback) ─────────────────────────────────────────────

fn classify_speaker_text(text: &str) -> &'static str {
    let lower = text.to_lowercase();
    let wc = text.split_whitespace().count();
    if lower.trim_end().ends_with('?') { return "Interviewer"; }
    let iv_starts = ["tell me", "walk me", "walk us", "describe", "explain", "what ", "how ", "why ",
                     "can you", "could you", "would you", "have you", "did you", "are you",
                     "so tell", "so how", "so what", "let's talk", "moving on"];
    for s in &iv_starts { if lower.starts_with(s) { return "Interviewer"; } }
    if wc > 10 {
        let self_refs = ["in my experience", "i've worked", "i led", "i built", "i managed",
                         "my approach", "my role", "my team", "i developed", "i designed"];
        for s in &self_refs { if lower.contains(s) { return "You"; } }
    }
    "Interviewer"
}

// ── Transcribe + diarize a set of chunks into segments ────────────────────────

async fn process_chunks(
    chunks: &[AudioChunk],
    forced_speaker: Option<&str>,
    cfg: &ReviewConfig,
    progress_tx: &watch::Sender<ReviewProgress>,
    pct_start: u8,
    pct_end: u8,
) -> Vec<TranscriptSegment> {
    let mut all: Vec<TranscriptSegment> = Vec::new();
    let total = chunks.len().max(1);

    for (i, chunk) in chunks.iter().enumerate() {
        let pct = pct_start + ((pct_end - pct_start) as usize * i / total) as u8;
        send_progress(progress_tx, pct, &format!("Transcribing segment {} of {}…", i + 1, total));

        let pcm = match tokio::fs::read(&chunk.path).await {
            Ok(b) => b,
            Err(_) => continue,
        };
        let text = transcribe_pcm(&pcm, cfg).await;
        if text.trim().is_empty() { continue; }

        // Build segments from diarization or heuristics
        let segs: Vec<TranscriptSegment> = if let Some(speaker) = forced_speaker {
            // Stereo-separated channel: speaker is known
            vec![TranscriptSegment { speaker: speaker.to_string(), text, timestamp_ms: chunk.start_ms }]
        } else if let Some(diarized) = diarize_pcm(&pcm, cfg).await {
            // Map diarization labels to roles using cumulative time heuristic
            let mut totals: HashMap<String, f64> = HashMap::new();
            for seg in &diarized { *totals.entry(seg.speaker.clone()).or_default() += seg.end - seg.start; }
            let you_label = totals.iter().max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(k, _)| k.clone()).unwrap_or_default();

            let words: Vec<&str> = text.split_whitespace().collect();
            let total_dur: f64 = diarized.iter().map(|s| s.end - s.start).sum::<f64>().max(0.001);
            let mut word_off = 0usize;
            let mut result = Vec::new();
            for seg in &diarized {
                let frac = (seg.end - seg.start) / total_dur;
                let wc = ((frac * words.len() as f64).round() as usize).max(1);
                let end = (word_off + wc).min(words.len());
                let seg_text = words[word_off..end].join(" ");
                if !seg_text.is_empty() {
                    let role = if seg.speaker == you_label { "You" } else { "Interviewer" };
                    result.push(TranscriptSegment {
                        speaker: role.to_string(),
                        text: seg_text,
                        timestamp_ms: chunk.start_ms + (seg.start * 1000.0) as u64,
                    });
                }
                word_off = end;
            }
            result
        } else {
            // Heuristic fallback
            vec![TranscriptSegment {
                speaker: classify_speaker_text(&text).to_string(),
                text,
                timestamp_ms: chunk.start_ms,
            }]
        };

        // Merge, deduplicating overlap with previous segments
        let cutoff = all.last().map(|s| s.timestamp_ms).unwrap_or(0);
        for seg in segs {
            if seg.timestamp_ms >= cutoff || all.is_empty() {
                all.push(seg);
            }
        }
    }
    all
}

// ── Q&A pair detection ────────────────────────────────────────────────────────

struct RawQaPair { question: String, answer: String, start_ms: u64, end_ms: u64, word_count: u32 }

fn detect_qa_pairs(segs: &[TranscriptSegment]) -> Vec<RawQaPair> {
    let mut pairs = Vec::new();
    let mut i = 0;
    while i < segs.len() {
        if segs[i].speaker == "Interviewer" {
            let mut q_parts = vec![segs[i].text.clone()];
            let mut j = i + 1;
            while j < segs.len() && segs[j].speaker == "Interviewer" { q_parts.push(segs[j].text.clone()); j += 1; }
            let question = q_parts.join(" ");
            let mut a_parts = Vec::new();
            let start_ms = segs.get(j).map(|s| s.timestamp_ms).unwrap_or(0);
            let mut end_ms = start_ms;
            while j < segs.len() && segs[j].speaker != "Interviewer" {
                a_parts.push(segs[j].text.clone());
                end_ms = segs[j].timestamp_ms;
                j += 1;
            }
            if !a_parts.is_empty() {
                let answer = a_parts.join(" ");
                let word_count = answer.split_whitespace().count() as u32;
                pairs.push(RawQaPair { question, answer, start_ms, end_ms, word_count });
            }
            i = j;
        } else {
            i += 1;
        }
    }
    pairs
}

// ── Stats helpers ─────────────────────────────────────────────────────────────

fn speaker_stats(segs: &[TranscriptSegment]) -> (u32, u32, usize) {
    let (mut you, mut them, mut turns) = (0u32, 0u32, 0usize);
    let mut last = "";
    for seg in segs {
        let wc = seg.text.split_whitespace().count() as u32;
        if seg.speaker == "You" { you += wc; } else { them += wc; }
        if seg.speaker != last { turns += 1; }
        last = &seg.speaker;
    }
    (you, them, turns)
}

fn detect_keywords(segs: &[TranscriptSegment], keywords: &[String]) -> Vec<String> {
    let text: String = segs.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join(" ").to_lowercase();
    keywords.iter().filter(|kw| text.contains(&kw.to_lowercase())).cloned().collect()
}

fn segs_to_review(segs: &[TranscriptSegment]) -> Vec<ReviewSegment> {
    segs.iter().enumerate().map(|(i, s)| {
        let end_ms = segs.get(i + 1).map(|n| n.timestamp_ms).unwrap_or(s.timestamp_ms + 4000);
        ReviewSegment { speaker: s.speaker.clone(), text: s.text.clone(), start_ms: s.timestamp_ms, end_ms }
    }).collect()
}

// ── Score Q&A pairs ───────────────────────────────────────────────────────────

async fn score_pairs(raw: &[RawQaPair], ai_cfg: &AiConfig<'_>) -> Vec<QaPair> {
    let mut pairs = Vec::new();
    for r in raw {
        let duration_secs = ((r.end_ms.saturating_sub(r.start_ms)) as f32 / 1000.0).max(1.0);
        let wpm = (r.word_count as f32 / duration_secs * 60.0).round() as u32;
        let feedback = generate_answer_feedback(&r.question, &r.answer, "", ai_cfg)
            .await
            .unwrap_or_else(|_| AnswerFeedbackResult { coaching: String::new(), missed_followup: false, missed_metric: false });
        pairs.push(QaPair {
            question: r.question.clone(),
            answer_text: r.answer.clone(),
            coaching: feedback.coaching,
            missed_followup: feedback.missed_followup,
            missed_metric: feedback.missed_metric,
            wpm,
            duration_secs,
        });
    }
    pairs
}

// ── Main upload processing pipeline ──────────────────────────────────────────

pub async fn process_review(
    id: String,
    source_path: PathBuf,
    source_filename: String,
    cfg: ReviewConfig,
    progress_tx: watch::Sender<ReviewProgress>,
) -> Result<ReviewReport> {
    let work_dir = cfg.reviews_dir.join(&id);
    tokio::fs::create_dir_all(&work_dir).await?;

    // Mark as in-progress so we can detect crashes on restart
    let _ = tokio::fs::write(
        work_dir.join("processing.json"),
        serde_json::json!({ "id": id, "source_filename": source_filename, "started_at": now_ms() }).to_string(),
    ).await;

    // 1. Probe duration + channel layout
    send_progress(&progress_tx, 5, "Detecting audio layout…");
    let layout = detect_channels(&source_path).await;
    let duration = probe_duration(&source_path).await;

    // 2. Extract audio as PCM
    send_progress(&progress_tx, 10, "Extracting audio…");
    let segments = match layout {
        AudioLayout::StereoSeparate => {
            let you_pcm = work_dir.join("you.pcm");
            let them_pcm = work_dir.join("them.pcm");
            extract_channel_pcm(&source_path, &you_pcm, "FL").await?;
            extract_channel_pcm(&source_path, &them_pcm, "FR").await?;

            send_progress(&progress_tx, 15, "Chunking audio…");
            let you_chunks = make_chunks(&you_pcm, &work_dir, duration).await?;
            let them_chunks = make_chunks(&them_pcm, &work_dir, duration).await?;

            let mut you_segs = process_chunks(&you_chunks, Some("You"), &cfg, &progress_tx, 20, 50).await;
            let mut them_segs = process_chunks(&them_chunks, Some("Interviewer"), &cfg, &progress_tx, 50, 80).await;
            you_segs.append(&mut them_segs);
            you_segs.sort_by_key(|s| s.timestamp_ms);
            you_segs
        }
        _ => {
            let pcm_path = work_dir.join("audio.pcm");
            extract_pcm(&source_path, &pcm_path).await?;

            send_progress(&progress_tx, 15, "Chunking audio…");
            let chunks = make_chunks(&pcm_path, &work_dir, duration).await?;
            process_chunks(&chunks, None, &cfg, &progress_tx, 20, 80).await
        }
    };

    // 3. Q&A detection + scoring
    send_progress(&progress_tx, 82, "Detecting questions and answers…");
    let raw_pairs = detect_qa_pairs(&segments);

    let ai_cfg = AiConfig {
        gemini_key: &cfg.gemini_key,
        anthropic_key: cfg.anthropic_key.as_deref(),
        groq_key: cfg.groq_key.as_deref(),
        groq_key_2: cfg.groq_key_2.as_deref(),
        ollama_url: &cfg.ollama_url,
        ollama_model: &cfg.ollama_model,
        usage: None,
    };

    let total_pairs = raw_pairs.len();
    let mut qa_pairs = Vec::new();
    for (i, r) in raw_pairs.iter().enumerate() {
        let pct = 84 + (i * 12 / total_pairs.max(1)) as u8;
        send_progress(&progress_tx, pct, &format!("Scoring answer {} of {}…", i + 1, total_pairs));
        let duration_secs = ((r.end_ms.saturating_sub(r.start_ms)) as f32 / 1000.0).max(1.0);
        let wpm = (r.word_count as f32 / duration_secs * 60.0).round() as u32;
        let feedback = generate_answer_feedback(&r.question, &r.answer, "", &ai_cfg)
            .await
            .unwrap_or_else(|_| AnswerFeedbackResult { coaching: String::new(), missed_followup: false, missed_metric: false });
        qa_pairs.push(QaPair {
            question: r.question.clone(),
            answer_text: r.answer.clone(),
            coaching: feedback.coaching,
            missed_followup: feedback.missed_followup,
            missed_metric: feedback.missed_metric,
            wpm,
            duration_secs,
        });
    }

    // 4. Compile report
    send_progress(&progress_tx, 97, "Compiling report…");
    let (you_wc, them_wc, turns) = speaker_stats(&segments);
    let total_wc = (you_wc + them_wc).max(1) as f32;
    let wpms: Vec<u32> = qa_pairs.iter().filter(|p| p.wpm > 0).map(|p| p.wpm).collect();
    let avg_wpm = if wpms.is_empty() { 0 } else { wpms.iter().sum::<u32>() / wpms.len() as u32 };

    let report = ReviewReport {
        id: id.clone(),
        created_at: now_ms(),
        duration_secs: duration,
        source_filename,
        source_type: "upload".to_string(),
        transcript: segs_to_review(&segments),
        qa_pairs: qa_pairs.clone(),
        vocal_summary: VocalSummary { avg_wpm, total_answers: qa_pairs.len() },
        speaker_summary: SpeakerSummary {
            you_pct: (you_wc as f32 / total_wc * 100.0).round(),
            them_pct: (them_wc as f32 / total_wc * 100.0).round(),
            you_word_count: you_wc,
            them_word_count: them_wc,
            turn_count: turns,
        },
        keywords_mentioned: detect_keywords(&segments, &cfg.keywords),
    };

    save_report(&cfg.reviews_dir, &report).await?;
    // Remove processing marker now that report is saved
    let _ = tokio::fs::remove_file(work_dir.join("processing.json")).await;
    let _ = progress_tx.send(ReviewProgress { pct: 100, step: "Done".to_string(), done: true, error: None });
    Ok(report)
}

// ── Live session report ───────────────────────────────────────────────────────

pub async fn generate_live_report(
    id: String,
    transcript: Vec<TranscriptSegment>,
    keywords: Vec<String>,
    cfg: ReviewConfig,
) -> Result<ReviewReport> {
    let ai_cfg = AiConfig {
        gemini_key: &cfg.gemini_key,
        anthropic_key: cfg.anthropic_key.as_deref(),
        groq_key: cfg.groq_key.as_deref(),
        groq_key_2: cfg.groq_key_2.as_deref(),
        ollama_url: &cfg.ollama_url,
        ollama_model: &cfg.ollama_model,
        usage: None,
    };

    let raw_pairs = detect_qa_pairs(&transcript);
    let qa_pairs = score_pairs(&raw_pairs, &ai_cfg).await;

    let first_ms = transcript.first().map(|s| s.timestamp_ms).unwrap_or(0);
    let last_ms = transcript.last().map(|s| s.timestamp_ms).unwrap_or(0);
    let duration_secs = (last_ms.saturating_sub(first_ms)) as f64 / 1000.0;

    let (you_wc, them_wc, turns) = speaker_stats(&transcript);
    let total_wc = (you_wc + them_wc).max(1) as f32;
    let wpms: Vec<u32> = qa_pairs.iter().filter(|p| p.wpm > 0).map(|p| p.wpm).collect();
    let avg_wpm = if wpms.is_empty() { 0 } else { wpms.iter().sum::<u32>() / wpms.len() as u32 };

    let report = ReviewReport {
        id: id.clone(),
        created_at: now_ms(),
        duration_secs,
        source_filename: "Live session".to_string(),
        source_type: "live".to_string(),
        transcript: segs_to_review(&transcript),
        keywords_mentioned: detect_keywords(&transcript, &keywords),
        qa_pairs: qa_pairs.clone(),
        vocal_summary: VocalSummary { avg_wpm, total_answers: qa_pairs.len() },
        speaker_summary: SpeakerSummary {
            you_pct: (you_wc as f32 / total_wc * 100.0).round(),
            them_pct: (them_wc as f32 / total_wc * 100.0).round(),
            you_word_count: you_wc,
            them_word_count: them_wc,
            turn_count: turns,
        },
    };

    save_report(&cfg.reviews_dir, &report).await?;
    Ok(report)
}

// ── Markdown export ───────────────────────────────────────────────────────────

pub fn format_markdown(r: &ReviewReport) -> String {
    let mins = (r.duration_secs / 60.0) as u32;
    let secs = (r.duration_secs % 60.0) as u32;
    let date_ms = r.created_at;
    let secs_since_epoch = date_ms / 1000;
    // Approximate calendar (good enough for display)
    let days = secs_since_epoch / 86400;
    let year = 1970u64 + days / 365;
    let doy = days % 365;
    let month = (doy / 30 + 1).min(12);
    let day = doy % 30 + 1;

    let mut md = format!(
        "# Interview Report\n\n**Source:** {}  \n**Duration:** {}:{:02}  \n**Date:** {}-{:02}-{:02}  \n**Type:** {}\n\n",
        r.source_filename, mins, secs, year, month, day, r.source_type
    );

    md += &format!(
        "## Speaker Summary\n\n| | Words | Share |\n|---|---|---|\n| **You** | {} | {}% |\n| **Interviewer** | {} | {}% |\n\n**Turn count:** {}\n\n",
        r.speaker_summary.you_word_count, r.speaker_summary.you_pct,
        r.speaker_summary.them_word_count, r.speaker_summary.them_pct,
        r.speaker_summary.turn_count,
    );

    if r.vocal_summary.avg_wpm > 0 {
        md += &format!("**Average pace:** {} wpm\n\n", r.vocal_summary.avg_wpm);
    }

    if !r.keywords_mentioned.is_empty() {
        md += &format!("## Keywords Mentioned\n\n{}\n\n", r.keywords_mentioned.join(", "));
    }

    if !r.qa_pairs.is_empty() {
        md += "## Q&A Coaching\n\n";
        for (i, qa) in r.qa_pairs.iter().enumerate() {
            md += &format!("### Q{}: {}\n\n", i + 1, qa.question);
            md += &format!("**Your answer:** {}\n\n", qa.answer_text);
            if !qa.coaching.is_empty() {
                md += &format!("**Coaching:** {}\n\n", qa.coaching);
            }
            let mut flags = Vec::new();
            if qa.missed_followup { flags.push("Missing follow-up question"); }
            if qa.missed_metric { flags.push("Missing specific metric/number"); }
            if !flags.is_empty() { md += &format!("**Watch:** {}\n\n", flags.join(" · ")); }
            if qa.wpm > 0 { md += &format!("**Pace:** {} wpm ({:.0}s)\n\n", qa.wpm, qa.duration_secs); }
            md += "---\n\n";
        }
    }
    md
}
