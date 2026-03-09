use anyhow::{anyhow, Result};
use base64::Engine;
use serde_json::{json, Value};

const VALID_EMOTIONS: &[&str] = &[
    "engaged", "curious", "neutral", "skeptical", "confused", "bored", "pleased",
];

pub async fn analyze_sentiment(api_key: &str, jpeg_bytes: &[u8]) -> Result<String> {
    let b64 = base64::engine::general_purpose::STANDARD.encode(jpeg_bytes);

    let body = json!({
        "contents": [{
            "parts": [
                { "inlineData": { "mimeType": "image/jpeg", "data": b64 } },
                { "text": "This is a screenshot of a video interview call. Focus on the interviewer (not the self-preview thumbnail). In one word, what emotion are they showing? Choose from: engaged, curious, neutral, skeptical, confused, bored, pleased. Reply with ONLY the one word." }
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
        .unwrap_or("neutral")
        .trim()
        .to_lowercase();

    // Find valid emotion in response
    let emotion = VALID_EMOTIONS
        .iter()
        .find(|&&e| raw.contains(e))
        .copied()
        .unwrap_or("neutral")
        .to_string();

    Ok(emotion)
}
