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

pub async fn stream_suggestions(
    api_key: &str,
    system_prompt: &str,
    user_prompt: &str,
    mode: SuggestionMode,
    event_tx: broadcast::Sender<WsEvent>,
) -> Result<()> {
    stream_openai_compat(
        api_key,
        "https://api.groq.com/openai/v1/chat/completions",
        "llama-3.1-8b-instant",
        "Groq",
        system_prompt,
        user_prompt,
        mode,
        event_tx,
    )
    .await
}

pub async fn stream_openai_compat(
    api_key: &str,
    base_url: &str,
    model: &str,
    provider: &str,
    system_prompt: &str,
    user_prompt: &str,
    mode: SuggestionMode,
    event_tx: broadcast::Sender<WsEvent>,
) -> Result<()> {
    let body = json!({
        "model": model,
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user",   "content": user_prompt }
        ],
        "max_tokens": 1000,
        "stream": true
    });

    let resp = client()
        .post(base_url)
        .bearer_auth(api_key)
        .json(&body)
        .timeout(std::time::Duration::from_secs(60))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(anyhow!("{} LLM error {}: {}", model, status, text));
    }

    // Extract rate-limit headers before consuming the response as a stream.
    // Most OpenAI-compatible APIs (Groq, Cerebras, OpenRouter, Mistral) return
    // x-ratelimit-remaining-requests / x-ratelimit-limit-requests.
    // Mistral also uses ratelimit-remaining / ratelimit-limit (no x- prefix).
    let rl_remaining: Option<u32> = resp.headers()
        .get("x-ratelimit-remaining-requests")
        .or_else(|| resp.headers().get("ratelimit-remaining"))
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok());
    let rl_limit: Option<u32> = resp.headers()
        .get("x-ratelimit-limit-requests")
        .or_else(|| resp.headers().get("ratelimit-limit"))
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok());
    if let (Some(remaining), Some(limit)) = (rl_remaining, rl_limit) {
        let _ = event_tx.send(WsEvent::RateLimit {
            provider: provider.to_string(),
            requests_remaining: remaining,
            requests_limit: limit,
        });
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
                    if data.trim() == "[DONE]" {
                        continue;
                    }
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

    let _ = event_tx.send(WsEvent::SuggestionComplete { full_text, mode });
    Ok(())
}
