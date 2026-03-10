use anyhow::{anyhow, Result};
use base64::Engine;
use serde_json::{json, Value};

const VALID_EMOTIONS: &[&str] = &[
    "engaged", "curious", "neutral", "skeptical", "confused", "bored", "pleased",
];

pub struct SentimentResult {
    pub emotion: String,
    pub coaching: Option<String>,
    pub requests_remaining: Option<u32>,
    pub requests_limit: Option<u32>,
}

pub async fn analyze_sentiment(api_key: &str, jpeg_bytes: &[u8]) -> Result<SentimentResult> {
    let b64 = base64::engine::general_purpose::STANDARD.encode(jpeg_bytes);

    let body = json!({
        "model": "claude-haiku-4-5-20251001",
        "max_tokens": 10,
        "messages": [{
            "role": "user",
            "content": [
                {
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": "image/jpeg",
                        "data": b64
                    }
                },
                {
                    "type": "text",
                    "text": "This is a screenshot of a video interview call. Focus on the interviewer (not the self-preview thumbnail). Analyze their facial expression, body language, and engagement level.\n\nRespond in exactly this format (two lines, nothing else):\nEMOTION: <one word: engaged, curious, neutral, skeptical, confused, bored, pleased>\nCOACHING: <one specific action the candidate should take right now based on what you observe>\n\nExamples:\nEMOTION: skeptical\nCOACHING: Add a concrete number or specific example to back up your last point.\n\nEMOTION: bored\nCOACHING: Pick up your energy and pace — ask the interviewer a question to re-engage them."
                }
            ]
        }]
    });

    let resp = reqwest::Client::new()
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .timeout(std::time::Duration::from_secs(20))
        .send()
        .await?;

    let requests_remaining = resp
        .headers()
        .get("anthropic-ratelimit-requests-remaining")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u32>().ok());

    let requests_limit = resp
        .headers()
        .get("anthropic-ratelimit-requests-limit")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u32>().ok());

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(anyhow!("Claude Vision API error {}: {}", status, text));
    }

    let json: Value = resp.json().await?;
    let raw = json["content"][0]["text"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();

    let (emotion, coaching) = parse_sentiment_response(&raw);
    Ok(SentimentResult { emotion, coaching, requests_remaining, requests_limit })
}

fn parse_sentiment_response(raw: &str) -> (String, Option<String>) {
    let mut emotion = "neutral".to_string();
    let mut coaching: Option<String> = None;

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
        } else if let Some(rest) = line.strip_prefix("COACHING:") {
            let tip = rest.trim().to_string();
            if !tip.is_empty() {
                coaching = Some(tip);
            }
        }
    }

    (emotion, coaching)
}
