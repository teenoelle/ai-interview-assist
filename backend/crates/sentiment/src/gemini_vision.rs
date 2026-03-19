use anyhow::{anyhow, Result};
use base64::Engine;
use serde_json::{json, Value};

pub struct PresenceResult {
    pub issues: Vec<String>,
    pub positive: Option<String>,
}

pub async fn analyze_presence(api_key: &str, jpeg_bytes: &[u8]) -> Result<PresenceResult> {
    let b64 = base64::engine::general_purpose::STANDARD.encode(jpeg_bytes);

    let body = json!({
        "contents": [{
            "parts": [
                { "inlineData": { "mimeType": "image/jpeg", "data": b64 } },
                { "text": "This is a webcam screenshot of a job interview candidate. Analyze their visible presence for real-time coaching.\n\nCheck for these issues: not looking at camera, slouching or closed posture, tense/flat facial expression, too far from camera (face too small), too close to camera, poor lighting (face in shadow or overexposed), visible background distractions.\n\nRespond in exactly this format (two lines only):\nISSUES: <up to 2 most important issues separated by |, or 'none'>\nPOSITIVE: <one short positive observation, or 'none'>\n\nExample:\nISSUES: Not looking at camera | Slouching\nPOSITIVE: Good lighting and clean background" }
            ]
        }]
    });

    let client = reqwest::Client::new();
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={}",
        api_key
    );

    let resp = client
        .post(&url)
        .json(&body)
        .timeout(std::time::Duration::from_secs(20))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(anyhow!("Gemini presence API error {}: {}", status, text));
    }

    let json: Value = resp.json().await?;
    let raw = json["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();

    let mut issues = Vec::new();
    let mut positive: Option<String> = None;
    for line in raw.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("ISSUES:") {
            let val = rest.trim();
            if val != "none" && !val.is_empty() {
                issues = val.split('|').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
            }
        } else if let Some(rest) = line.strip_prefix("POSITIVE:") {
            let val = rest.trim().to_string();
            if val != "none" && !val.is_empty() { positive = Some(val); }
        }
    }

    Ok(PresenceResult { issues, positive })
}

const VALID_EMOTIONS: &[&str] = &[
    "engaged", "curious", "neutral", "skeptical", "confused", "bored", "pleased",
];

pub struct SentimentResult {
    pub emotion: String,
    pub reason: Option<String>,
    pub coaching: Option<String>,
    pub coaching_why: Option<String>,
}

pub async fn analyze_sentiment(api_key: &str, jpeg_bytes: &[u8]) -> Result<SentimentResult> {
    let b64 = base64::engine::general_purpose::STANDARD.encode(jpeg_bytes);

    let body = json!({
        "contents": [{
            "parts": [
                { "inlineData": { "mimeType": "image/jpeg", "data": b64 } },
                { "text": "This is a screenshot of a video interview call. Focus on the interviewer (not the self-preview thumbnail). Analyze their facial expression, body language, and engagement level.\n\nRespond in exactly this format (four lines, nothing else):\nEMOTION: <one word: engaged, curious, neutral, skeptical, confused, bored, pleased>\nREASON: <brief visual cue — 5-8 words, e.g. 'leaning forward, nodding frequently'>\nCOACHING: <one specific body language or expression adjustment the candidate should make RIGHT NOW>\nCOACHING_WHY: <one sentence — why this adjustment will help in this specific moment>\n\nExample:\nEMOTION: skeptical\nREASON: arms crossed, raised eyebrow\nCOACHING: Lean forward slightly and nod while making your next point\nCOACHING_WHY: Mirroring openness signals you welcome scrutiny and builds rapport with a doubting interviewer." }
            ]
        }]
    });

    let client = reqwest::Client::new();
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={}",
        api_key
    );

    let resp = client
        .post(&url)
        .json(&body)
        .timeout(std::time::Duration::from_secs(20))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(anyhow!("Gemini Vision API error {}: {}", status, text));
    }

    let json: Value = resp.json().await?;
    let raw = json["candidates"][0]["content"]["parts"][0]["text"]
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
