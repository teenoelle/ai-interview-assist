use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use tokio::sync::broadcast;
use common::messages::{WsEvent, SuggestionMode};
use futures::StreamExt;
use std::sync::OnceLock;

static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
fn client() -> &'static reqwest::Client {
    CLIENT.get_or_init(reqwest::Client::new)
}

/// Pre-warm the TLS connection to api.anthropic.com so the first real suggestion
/// request doesn't pay the full DNS + TCP + TLS handshake cost (~10-20s on Windows).
pub async fn prewarm(api_key: &str) {
    let _ = client()
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&serde_json::json!({
            "model": "claude-haiku-4-5-20251001",
            "max_tokens": 1,
            "messages": [{ "role": "user", "content": "hi" }]
        }))
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await;
}

pub async fn stream_suggestions(
    api_key: &str,
    system_prompt: &str,
    user_prompt: &str,
    mode: SuggestionMode,
    event_tx: broadcast::Sender<WsEvent>,
) -> Result<()> {
    let body = json!({
        "model": "claude-haiku-4-5-20251001",
        "max_tokens": 1500,
        "stream": true,
        "system": system_prompt,
        "messages": [{ "role": "user", "content": user_prompt }]
    });

    let resp = client()
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .timeout(std::time::Duration::from_secs(60))
        .send()
        .await?;

    // Read rate limit headers before consuming body
    let requests_remaining = resp.headers()
        .get("anthropic-ratelimit-requests-remaining")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u32>().ok());
    let requests_limit = resp.headers()
        .get("anthropic-ratelimit-requests-limit")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u32>().ok());

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(anyhow!("Claude LLM error {}: {}", status, text));
    }

    let mut full_text = String::new();
    let mut buf = String::new();
    let mut stream = resp.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        buf.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(pos) = buf.find("\n\n") {
            let event = buf[..pos].to_string();
            buf = buf[pos + 2..].to_string();
            for line in event.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if let Ok(json) = serde_json::from_str::<Value>(data) {
                        // Claude streaming: content_block_delta events contain the text
                        if json["type"] == "content_block_delta" {
                            if let Some(token) = json["delta"]["text"].as_str() {
                                full_text.push_str(token);
                                let _ = event_tx.send(WsEvent::SuggestionToken { token: token.to_string(), mode });
                            }
                        }
                    }
                }
            }
        }
    }

    tracing::info!("suggestion ✓ Claude API (claude-haiku-4-5-20251001) — {} chars", full_text.len());
    let _ = event_tx.send(WsEvent::SuggestionComplete { full_text, mode });

    if let (Some(remaining), Some(limit)) = (requests_remaining, requests_limit) {
        let _ = event_tx.send(WsEvent::RateLimit {
            provider: "Claude API".to_string(),
            requests_remaining: remaining,
            requests_limit: limit,
        });
    }

    Ok(())
}
