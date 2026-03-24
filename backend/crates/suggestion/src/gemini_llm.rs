use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use tokio::sync::broadcast;
use common::messages::{WsEvent, SuggestionMode};
use futures::StreamExt;

pub async fn stream_suggestions(
    api_key: &str,
    system_prompt: &str,
    user_prompt: &str,
    mode: SuggestionMode,
    event_tx: broadcast::Sender<WsEvent>,
) -> Result<()> {
    let body = json!({
        "system_instruction": {
            "parts": [{ "text": system_prompt }]
        },
        "contents": [{
            "role": "user",
            "parts": [{ "text": user_prompt }]
        }],
        "generationConfig": { "maxOutputTokens": 1000 }
    });

    let client = reqwest::Client::new();
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:streamGenerateContent?key={}&alt=sse",
        api_key
    );

    let resp = client
        .post(&url)
        .json(&body)
        .timeout(std::time::Duration::from_secs(60))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(anyhow!("Gemini LLM API error {}: {}", status, text));
    }

    let mut full_text = String::new();
    let mut buf = String::new();
    let mut stream = resp.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        let text = String::from_utf8_lossy(&chunk);
        buf.push_str(&text);

        // Process complete SSE events
        while let Some(pos) = buf.find("\n\n") {
            let event = buf[..pos].to_string();
            buf = buf[pos + 2..].to_string();

            for line in event.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        continue;
                    }
                    if let Ok(json) = serde_json::from_str::<Value>(data) {
                        if let Some(token) = json["candidates"][0]["content"]["parts"][0]["text"].as_str() {
                            full_text.push_str(token);
                            let _ = event_tx.send(WsEvent::SuggestionToken {
                                token: token.to_string(), mode,
                            });
                        }
                    }
                }
            }
        }
    }

    let _ = event_tx.send(WsEvent::SuggestionComplete { full_text, mode });
    Ok(())
}
