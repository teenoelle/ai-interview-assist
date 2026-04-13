use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use tokio::sync::broadcast;
use common::messages::{WsEvent, SuggestionMode};
use futures::StreamExt;
use reqwest::Client;
use std::sync::OnceLock;

static CLIENT: OnceLock<Client> = OnceLock::new();
fn client() -> &'static Client {
    CLIENT.get_or_init(|| Client::builder().pool_max_idle_per_host(0).build().expect("bonsai client"))
}

pub async fn stream_suggestions(
    base_url: &str,
    model: &str,
    system_prompt: &str,
    user_prompt: &str,
    mode: SuggestionMode,
    event_tx: broadcast::Sender<WsEvent>,
) -> Result<()> {
    let url = format!("{}/v1/chat/completions", base_url.trim_end_matches('/'));
    let body = json!({
        "model": model,
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user",   "content": user_prompt }
        ],
        "max_tokens": 1500,
        "temperature": 0.6,
        "stream": true,
        "options": {
            "num_ctx": 8192
        }
    });

    let resp = client()
        .post(&url)
        .json(&body)
        .timeout(std::time::Duration::from_secs(60))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(anyhow!("Bonsai {} error {}: {}", model, status, text));
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
                    if data.trim() == "[DONE]" { continue; }
                    if let Ok(json) = serde_json::from_str::<Value>(data) {
                        if let Some(token) = json["choices"][0]["delta"]["content"].as_str() {
                            full_text.push_str(token);
                            let _ = event_tx.send(WsEvent::SuggestionToken { token: token.to_string(), mode });
                        }
                    }
                }
            }
        }
    }

    tracing::info!("suggestion ✓ Bonsai ({}) — {} chars", model, full_text.len());
    let _ = event_tx.send(WsEvent::SuggestionComplete { full_text, mode });
    Ok(())
}
