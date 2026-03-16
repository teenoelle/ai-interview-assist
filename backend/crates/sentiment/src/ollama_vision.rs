use anyhow::{anyhow, Result};
use base64::Engine;
use serde_json::{json, Value};

const VALID_EMOTIONS: &[&str] = &[
    "engaged", "curious", "neutral", "skeptical", "confused", "bored", "pleased",
];

pub struct SentimentResult {
    pub emotion: String,
    pub reason: Option<String>,
    pub coaching: Option<String>,
    pub coaching_why: Option<String>,
}

/// Analyze sentiment using a local Ollama vision model (e.g. llava).
/// Calls the native Ollama /api/chat endpoint with base64-encoded image.
pub async fn analyze_sentiment(
    base_url: &str,
    model: &str,
    jpeg_bytes: &[u8],
) -> Result<SentimentResult> {
    let b64 = base64::engine::general_purpose::STANDARD.encode(jpeg_bytes);

    let prompt = "This is a screenshot of a video interview call. Focus on the interviewer (not the self-preview thumbnail). Analyze their facial expression, body language, and engagement level.\n\nRespond in exactly this format (four lines, nothing else):\nEMOTION: <one word: engaged, curious, neutral, skeptical, confused, bored, pleased>\nREASON: <brief visual cue — 5-8 words, e.g. 'leaning forward, nodding frequently'>\nCOACHING: <one specific body language or expression adjustment the candidate should make RIGHT NOW>\nCOACHING_WHY: <one sentence — why this adjustment will help in this specific moment>";

    let body = json!({
        "model": model,
        "messages": [{
            "role": "user",
            "content": prompt,
            "images": [b64]
        }],
        "stream": false
    });

    let url = format!("{}/api/chat", base_url.trim_end_matches('/'));
    let resp = reqwest::Client::new()
        .post(&url)
        .json(&body)
        .timeout(std::time::Duration::from_secs(45))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(anyhow!("Ollama vision error {}: {}", status, text));
    }

    let json: Value = resp.json().await?;
    let raw = json["message"]["content"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();

    let (emotion, reason, coaching, coaching_why) = parse_sentiment_response(&raw);
    Ok(SentimentResult { emotion, reason, coaching, coaching_why })
}

fn parse_sentiment_response(raw: &str) -> (String, Option<String>, Option<String>, Option<String>) {
    let mut emotion = "neutral".to_string();
    let mut reason: Option<String> = None;
    let mut coaching: Option<String> = None;
    let mut coaching_why: Option<String> = None;

    for line in raw.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("EMOTION:") {
            let word = rest.trim().to_lowercase();
            emotion = VALID_EMOTIONS
                .iter()
                .find(|&&e| word.contains(e))
                .copied()
                .unwrap_or("neutral")
                .to_string();
        } else if let Some(rest) = line.strip_prefix("REASON:") {
            let r = rest.trim().to_string();
            if !r.is_empty() { reason = Some(r); }
        } else if let Some(rest) = line.strip_prefix("COACHING_WHY:") {
            let w = rest.trim().to_string();
            if !w.is_empty() { coaching_why = Some(w); }
        } else if let Some(rest) = line.strip_prefix("COACHING:") {
            let tip = rest.trim().to_string();
            if !tip.is_empty() { coaching = Some(tip); }
        }
    }

    (emotion, reason, coaching, coaching_why)
}
