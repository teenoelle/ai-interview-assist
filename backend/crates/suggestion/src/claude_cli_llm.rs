use anyhow::Result;
use tokio::sync::broadcast;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use std::process::Stdio;
use serde_json::Value;
use common::messages::{WsEvent, SuggestionMode};

pub async fn stream_suggestions(
    system_prompt: &str,
    user_prompt: &str,
    mode: SuggestionMode,
    event_tx: broadcast::Sender<WsEvent>,
) -> Result<()> {
    let mut child = Command::new("claude")
        .args([
            "--print",
            "--output-format", "stream-json",
            "--verbose",
            "--model", "claude-haiku-4-5-20251001",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // Neutral dir so it doesn't load project CLAUDE.md / memory
        .current_dir(std::env::temp_dir())
        .spawn()?;

    // Pass system + user prompt via stdin to avoid Windows command line length limit
    if let Some(mut stdin) = child.stdin.take() {
        let combined = format!("<system>\n{}\n</system>\n\n{}", system_prompt, user_prompt);
        stdin.write_all(combined.as_bytes()).await?;
    }

    let stdout = child.stdout.take().expect("stdout piped");
    let stderr = child.stderr.take().expect("stderr piped");
    let reader = BufReader::new(stdout);
    let mut lines = reader.lines();
    let mut full_text = String::new();
    let mut last_error: Option<String> = None;

    while let Some(line) = lines.next_line().await? {
        if line.is_empty() { continue; }
        let Ok(v) = serde_json::from_str::<Value>(&line) else { continue };
        match v["type"].as_str() {
            Some("assistant") => {
                if let Some(blocks) = v["message"]["content"].as_array() {
                    for block in blocks {
                        if block["type"].as_str() == Some("text") {
                            if let Some(text) = block["text"].as_str() {
                                full_text.push_str(text);
                                let _ = event_tx.send(WsEvent::SuggestionToken {
                                    token: text.to_string(),
                                    mode,
                                });
                            }
                        }
                    }
                }
            }
            Some("result") => {
                // Check for API-level error (e.g. rate limit, overload)
                if v["is_error"].as_bool() == Some(true) {
                    let msg = v["result"].as_str()
                        .or_else(|| v["error"]["message"].as_str())
                        .unwrap_or("unknown API error");
                    last_error = Some(msg.to_string());
                    tracing::warn!("Claude CLI API error in result event: {}", msg);
                } else if full_text.is_empty() {
                    // Fallback: use result field if no assistant events emitted text
                    if let Some(result) = v["result"].as_str() {
                        full_text = result.to_string();
                        let _ = event_tx.send(WsEvent::SuggestionToken {
                            token: full_text.clone(),
                            mode,
                        });
                    }
                }
            }
            _ => {}
        }
    }

    // Drain stderr for diagnostics
    let mut stderr_reader = BufReader::new(stderr);
    let mut stderr_text = String::new();
    let _ = stderr_reader.read_to_string(&mut stderr_text).await;

    let status = child.wait().await?;
    if !status.success() {
        // Prefer the structured error from the JSON stream, then stderr, then generic
        let detail = last_error
            .as_deref()
            .or_else(|| if stderr_text.is_empty() { None } else { Some(stderr_text.trim()) })
            .map(|s| format!(": {}", s))
            .unwrap_or_default();
        anyhow::bail!("claude CLI exited {}{}", status, detail);
    }
    if full_text.is_empty() {
        let detail = last_error.as_deref().unwrap_or(stderr_text.trim());
        anyhow::bail!("claude CLI returned empty response ({})", detail);
    }

    tracing::info!("suggestion ✓ Claude CLI (haiku) — {} chars", full_text.len());
    let _ = event_tx.send(WsEvent::SuggestionComplete { full_text, mode });
    Ok(())
}
