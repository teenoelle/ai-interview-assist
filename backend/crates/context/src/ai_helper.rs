use anyhow::Result;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex, OnceLock};

static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
fn client() -> &'static reqwest::Client {
    CLIENT.get_or_init(reqwest::Client::new)
}

/// Pre-warm the TLS connection used by call_ai / call_ai_fast (Anthropic endpoint).
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

/// Safely truncate a UTF-8 string to at most `chars` characters.
/// Prevents panics when slicing strings containing multi-byte Unicode characters.
fn trunc(s: &str, chars: usize) -> &str {
    match s.char_indices().nth(chars) {
        Some((i, _)) => &s[..i],
        None => s,
    }
}

pub type UsageCounter = Arc<Mutex<std::collections::HashMap<String, u64>>>;

/// Bundled AI provider config passed to all helper functions.
pub struct AiConfig<'a> {
    pub gemini_key: &'a str,
    pub anthropic_key: Option<&'a str>,
    pub mistral_key: Option<&'a str>,
    pub bonsai_url: Option<&'a str>,
    pub bonsai_model: &'a str,
    pub groq_key: Option<&'a str>,
    pub groq_key_2: Option<&'a str>,
    pub ollama_url: &'a str,
    pub ollama_model: &'a str,
    pub usage: Option<UsageCounter>,
}

fn inc(usage: &Option<UsageCounter>, provider: &str) {
    if let Some(map) = usage {
        if let Ok(mut m) = map.lock() {
            *m.entry(provider.to_string()).or_insert(0) += 1;
        }
    }
}

async fn try_groq_text(key: &str, prompt: &str, max_tokens: u32) -> Option<String> {
    let body = json!({
        "model": "llama-3.3-70b-versatile",
        "max_tokens": max_tokens,
        "messages": [{ "role": "user", "content": prompt }]
    });
    let resp = client()
        .post("https://api.groq.com/openai/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", key))
        .json(&body)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await.ok()?;
    if !resp.status().is_success() {
        tracing::debug!("Groq text returned {}", resp.status());
        return None;
    }
    let j: Value = resp.json().await.ok()?;
    let text = j["choices"][0]["message"]["content"].as_str()?.trim().to_string();
    if text.is_empty() { None } else { Some(text) }
}

async fn try_groq_text_with_system(key: &str, system: &str, user: &str, max_tokens: u32) -> Option<String> {
    let body = json!({
        "model": "llama-3.3-70b-versatile",
        "max_tokens": max_tokens,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": user }
        ]
    });
    let resp = client()
        .post("https://api.groq.com/openai/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", key))
        .json(&body)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await.ok()?;
    if !resp.status().is_success() {
        tracing::debug!("Groq text+system returned {}", resp.status());
        return None;
    }
    let j: Value = resp.json().await.ok()?;
    let text = j["choices"][0]["message"]["content"].as_str()?.trim().to_string();
    if text.is_empty() { None } else { Some(text) }
}

async fn try_ollama_text(url: &str, model: &str, prompt: &str, max_tokens: u32) -> Option<String> {
    let body = json!({
        "model": model,
        "max_tokens": max_tokens,
        "messages": [{ "role": "user", "content": prompt }]
    });
    let resp = client()
        .post(format!("{}/v1/chat/completions", url))
        .json(&body)
        .timeout(std::time::Duration::from_secs(60))
        .send()
        .await.ok()?;
    if !resp.status().is_success() {
        tracing::debug!("Ollama text returned {}", resp.status());
        return None;
    }
    let j: Value = resp.json().await.ok()?;
    let text = j["choices"][0]["message"]["content"].as_str()?.trim().to_string();
    if text.is_empty() { None } else { Some(text) }
}

async fn try_ollama_text_with_system(url: &str, model: &str, system: &str, user: &str, max_tokens: u32) -> Option<String> {
    let body = json!({
        "model": model,
        "max_tokens": max_tokens,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": user }
        ]
    });
    let resp = client()
        .post(format!("{}/v1/chat/completions", url))
        .json(&body)
        .timeout(std::time::Duration::from_secs(60))
        .send()
        .await.ok()?;
    if !resp.status().is_success() {
        tracing::debug!("Ollama text+system returned {}", resp.status());
        return None;
    }
    let j: Value = resp.json().await.ok()?;
    let text = j["choices"][0]["message"]["content"].as_str()?.trim().to_string();
    if text.is_empty() { None } else { Some(text) }
}

async fn try_bonsai_text(url: &str, model: &str, prompt: &str, max_tokens: u32) -> Option<String> {
    // Try /health first (LM Studio, custom servers), fall back to / (Ollama).
    let base = url.trim_end_matches('/');
    let reachable = {
        let r = client().get(format!("{}/health", base)).timeout(std::time::Duration::from_secs(2)).send().await;
        match r {
            Ok(resp) if resp.status().is_success() => true,
            _ => client().get(base).timeout(std::time::Duration::from_secs(2)).send().await
                    .map(|r| r.status().is_success()).unwrap_or(false),
        }
    };
    if !reachable { return None; }
    let body = json!({
        "model": model,
        "max_tokens": max_tokens,
        "messages": [{ "role": "user", "content": prompt }],
        "stream": false
    });
    let resp = client()
        .post(format!("{}/v1/chat/completions", url.trim_end_matches('/')))
        .json(&body)
        .timeout(std::time::Duration::from_secs(60))
        .send().await.ok()?;
    if !resp.status().is_success() { return None; }
    let j: Value = resp.json().await.ok()?;
    let text = j["choices"][0]["message"]["content"].as_str()?.trim().to_string();
    if text.is_empty() { None } else { Some(text) }
}

async fn try_claude_cli_text(prompt: &str, max_tokens: u32) -> Option<String> {
    use tokio::process::Command;
    use tokio::io::{AsyncWriteExt, AsyncReadExt, BufReader};
    use std::process::Stdio;
    let mut child = Command::new("claude")
        .args(["--print", "--output-format", "text", "--model", "claude-haiku-4-5-20251001",
               "--permission-mode", "bypassPermissions"])
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null())
        .current_dir(std::env::temp_dir())
        .spawn().ok()?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(prompt.as_bytes()).await;
    }
    let stdout = child.stdout.take()?;
    let mut reader = BufReader::new(stdout);
    let mut out = String::new();
    let _ = reader.read_to_string(&mut out).await;
    let status = child.wait().await.ok()?;
    if !status.success() || out.trim().is_empty() { return None; }
    // Honour max_tokens roughly by word count
    let words: Vec<&str> = out.split_whitespace().collect();
    let approx = words[..words.len().min(max_tokens as usize * 3 / 4)].join(" ");
    Some(if approx.len() < out.trim().len() { approx } else { out.trim().to_string() })
}

async fn try_mistral_text(key: &str, prompt: &str, max_tokens: u32) -> Option<String> {
    let body = json!({
        "model": "mistral-large-latest",
        "max_tokens": max_tokens,
        "messages": [{ "role": "user", "content": prompt }]
    });
    let resp = client()
        .post("https://api.mistral.ai/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", key))
        .json(&body)
        .timeout(std::time::Duration::from_secs(30))
        .send().await.ok()?;
    if !resp.status().is_success() { return None; }
    let j: Value = resp.json().await.ok()?;
    let text = j["choices"][0]["message"]["content"].as_str()?.trim().to_string();
    if text.is_empty() { None } else { Some(text) }
}

/// Claude-first AI call for important one-shot generation (debrief, analysis).
/// Order: Claude → Groq key1 → Groq key2 → Gemini. Skips Ollama (too slow for long outputs).
pub async fn call_ai_quality(cfg: &AiConfig<'_>, prompt: &str, max_tokens: u32) -> Result<String> {
    // Claude first — pre-warmed at startup, fast, no TPM rate limit
    if let Some(key) = cfg.anthropic_key {
        let body = json!({
            "model": "claude-haiku-4-5-20251001",
            "max_tokens": max_tokens,
            "messages": [{ "role": "user", "content": prompt }]
        });
        if let Ok(resp) = client()
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
        {
            if resp.status().is_success() {
                if let Ok(j) = resp.json::<Value>().await {
                    let text = j["content"][0]["text"].as_str().unwrap_or("").trim().to_string();
                    if !text.is_empty() {
                        inc(&cfg.usage, "Claude API");
                        return Ok(text);
                    }
                }
            } else {
                tracing::debug!("Claude quality returned {}, trying Groq", resp.status());
            }
        }
    }

    // Groq fallback
    if let Some(key) = cfg.groq_key {
        if let Some(text) = try_groq_text(key, prompt, max_tokens).await {
            inc(&cfg.usage, "Groq");
            return Ok(text);
        }
    }
    if let Some(key) = cfg.groq_key_2 {
        if let Some(text) = try_groq_text(key, prompt, max_tokens).await {
            inc(&cfg.usage, "Groq #2");
            return Ok(text);
        }
    }

    // Gemini final fallback
    let body = json!({
        "contents": [{ "parts": [{ "text": prompt }] }],
        "generationConfig": { "maxOutputTokens": max_tokens }
    });
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={}",
        cfg.gemini_key
    );
    let resp = client()
        .post(&url)
        .json(&body)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await?;
    let j: Value = resp.json().await?;
    inc(&cfg.usage, "Gemini");
    Ok(j["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string())
}

/// Call AI providers in order: Bonsai → Claude CLI → Claude API → Mistral → Groq → Gemini.
pub async fn call_ai(cfg: &AiConfig<'_>, prompt: &str, max_tokens: u32) -> Result<String> {
    // Bonsai — local LAN model
    if let Some(url) = cfg.bonsai_url {
        if let Some(text) = try_bonsai_text(url, cfg.bonsai_model, prompt, max_tokens).await {
            inc(&cfg.usage, "Bonsai");
            return Ok(text);
        }
    }

    // Claude CLI
    if let Some(text) = try_claude_cli_text(prompt, max_tokens).await {
        inc(&cfg.usage, "Claude CLI");
        return Ok(text);
    }

    // Claude API
    if let Some(key) = cfg.anthropic_key {
        let body = json!({
            "model": "claude-haiku-4-5-20251001",
            "max_tokens": max_tokens,
            "messages": [{ "role": "user", "content": prompt }]
        });
        let resp = client()
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await?;
        if resp.status().is_success() {
            let j: Value = resp.json().await?;
            inc(&cfg.usage, "Claude API");
            return Ok(j["content"][0]["text"].as_str().unwrap_or("").trim().to_string());
        }
        tracing::debug!("Claude API returned {}, trying next provider", resp.status());
    }

    // Mistral
    if let Some(key) = cfg.mistral_key {
        if let Some(text) = try_mistral_text(key, prompt, max_tokens).await {
            inc(&cfg.usage, "Mistral");
            return Ok(text);
        }
    }

    // Groq key1
    if let Some(key) = cfg.groq_key {
        if let Some(text) = try_groq_text(key, prompt, max_tokens).await {
            inc(&cfg.usage, "Groq");
            return Ok(text);
        }
    }

    // Groq key2
    if let Some(key) = cfg.groq_key_2 {
        if let Some(text) = try_groq_text(key, prompt, max_tokens).await {
            inc(&cfg.usage, "Groq #2");
            return Ok(text);
        }
    }

    // Gemini fallback
    let body = json!({
        "contents": [{ "parts": [{ "text": prompt }] }],
        "generationConfig": { "maxOutputTokens": max_tokens }
    });
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={}",
        cfg.gemini_key
    );
    let resp = client()
        .post(&url)
        .json(&body)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await?;
    let j: Value = resp.json().await?;
    inc(&cfg.usage, "Gemini");
    Ok(j["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string())
}

/// Call AI with a system prompt, Groq-first order: Groq key1 → Groq key2 → Claude → Ollama → Gemini.
/// Uses llama-3.1-8b-instant for sub-second latency on short-generation tasks.
pub async fn call_ai_fast(cfg: &AiConfig<'_>, system_prompt: &str, user_prompt: &str) -> Result<String> {
    let max_tokens = 400u32;

    // Groq key1 — 8b-instant: fastest model on Groq (~200ms TTFT for short outputs)
    if let Some(key) = cfg.groq_key {
        let body = serde_json::json!({
            "model": "llama-3.1-8b-instant",
            "max_tokens": max_tokens,
            "messages": [
                { "role": "system", "content": system_prompt },
                { "role": "user", "content": user_prompt }
            ]
        });
        let resp = client()
            .post("https://api.groq.com/openai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", key))
            .json(&body)
            .timeout(std::time::Duration::from_secs(15))
            .send()
            .await;
        if let Ok(r) = resp {
            if r.status().is_success() {
                if let Ok(j) = r.json::<serde_json::Value>().await {
                    let text = j["choices"][0]["message"]["content"].as_str().unwrap_or("").trim().to_string();
                    if !text.is_empty() {
                        inc(&cfg.usage, "Groq");
                        return Ok(text);
                    }
                }
            } else {
                tracing::debug!("Groq fast returned {}, trying key2", r.status());
            }
        }
    }

    // Groq key2
    if let Some(key) = cfg.groq_key_2 {
        let body = serde_json::json!({
            "model": "llama-3.1-8b-instant",
            "max_tokens": max_tokens,
            "messages": [
                { "role": "system", "content": system_prompt },
                { "role": "user", "content": user_prompt }
            ]
        });
        let resp = client()
            .post("https://api.groq.com/openai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", key))
            .json(&body)
            .timeout(std::time::Duration::from_secs(15))
            .send()
            .await;
        if let Ok(r) = resp {
            if r.status().is_success() {
                if let Ok(j) = r.json::<serde_json::Value>().await {
                    let text = j["choices"][0]["message"]["content"].as_str().unwrap_or("").trim().to_string();
                    if !text.is_empty() {
                        inc(&cfg.usage, "Groq #2");
                        return Ok(text);
                    }
                }
            }
        }
    }

    // Claude Haiku — fallback
    if let Some(key) = cfg.anthropic_key {
        let body = json!({
            "model": "claude-haiku-4-5-20251001",
            "max_tokens": max_tokens,
            "system": [{"type": "text", "text": system_prompt, "cache_control": {"type": "ephemeral"}}],
            "messages": [{ "role": "user", "content": user_prompt }]
        });
        let resp = client()
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", key)
            .header("anthropic-version", "2023-06-01")
            .header("anthropic-beta", "prompt-caching-2024-07-31")
            .json(&body)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await?;
        if resp.status().is_success() {
            let j: Value = resp.json().await?;
            inc(&cfg.usage, "Claude API");
            return Ok(j["content"][0]["text"].as_str().unwrap_or("").trim().to_string());
        }
        tracing::debug!("Claude returned {}, trying next provider", resp.status());
    }

    // Ollama (local, no quota)
    if let Some(text) = try_ollama_text_with_system(cfg.ollama_url, cfg.ollama_model, system_prompt, user_prompt, max_tokens).await {
        inc(&cfg.usage, "Ollama");
        return Ok(text);
    }

    // Gemini fallback
    let body = json!({
        "system_instruction": { "parts": [{ "text": system_prompt }] },
        "contents": [{ "role": "user", "parts": [{ "text": user_prompt }] }],
        "generationConfig": { "maxOutputTokens": max_tokens }
    });
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={}",
        cfg.gemini_key
    );
    let resp = client()
        .post(&url)
        .json(&body)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await?;
    let j: Value = resp.json().await?;
    inc(&cfg.usage, "Gemini");
    Ok(j["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string())
}

/// Call AI with a system prompt: Bonsai → Claude CLI → Claude API → Mistral → Groq → Gemini.
pub async fn call_ai_simple(cfg: &AiConfig<'_>, system_prompt: &str, user_prompt: &str) -> Result<String> {
    let max_tokens = 400u32;
    let combined = format!("<system>\n{}\n</system>\n\n{}", system_prompt, user_prompt);

    // Bonsai — local LAN model
    if let Some(url) = cfg.bonsai_url {
        if let Some(text) = try_bonsai_text(url, cfg.bonsai_model, &combined, max_tokens).await {
            inc(&cfg.usage, "Bonsai");
            return Ok(text);
        }
    }

    // Claude CLI
    if let Some(text) = try_claude_cli_text(&combined, max_tokens).await {
        inc(&cfg.usage, "Claude CLI");
        return Ok(text);
    }

    // Claude API
    if let Some(key) = cfg.anthropic_key {
        let body = json!({
            "model": "claude-haiku-4-5-20251001",
            "max_tokens": max_tokens,
            "system": [{"type": "text", "text": system_prompt, "cache_control": {"type": "ephemeral"}}],
            "messages": [{ "role": "user", "content": user_prompt }]
        });
        let resp = client()
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", key)
            .header("anthropic-version", "2023-06-01")
            .header("anthropic-beta", "prompt-caching-2024-07-31")
            .json(&body)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await?;
        if resp.status().is_success() {
            let j: Value = resp.json().await?;
            inc(&cfg.usage, "Claude API");
            return Ok(j["content"][0]["text"].as_str().unwrap_or("").trim().to_string());
        }
        tracing::debug!("Claude API returned {}, trying next provider", resp.status());
    }

    // Mistral
    if let Some(key) = cfg.mistral_key {
        if let Some(text) = try_mistral_text(key, &combined, max_tokens).await {
            inc(&cfg.usage, "Mistral");
            return Ok(text);
        }
    }

    // Groq key1
    if let Some(key) = cfg.groq_key {
        if let Some(text) = try_groq_text_with_system(key, system_prompt, user_prompt, max_tokens).await {
            inc(&cfg.usage, "Groq");
            return Ok(text);
        }
    }

    // Groq key2
    if let Some(key) = cfg.groq_key_2 {
        if let Some(text) = try_groq_text_with_system(key, system_prompt, user_prompt, max_tokens).await {
            inc(&cfg.usage, "Groq #2");
            return Ok(text);
        }
    }

    // Gemini fallback
    let body = json!({
        "system_instruction": { "parts": [{ "text": system_prompt }] },
        "contents": [{ "role": "user", "parts": [{ "text": user_prompt }] }],
        "generationConfig": { "maxOutputTokens": max_tokens }
    });
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={}",
        cfg.gemini_key
    );
    let resp = client()
        .post(&url)
        .json(&body)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await?;
    let j: Value = resp.json().await?;
    inc(&cfg.usage, "Gemini");
    Ok(j["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string())
}

/// Generate 8 likely interview questions from the system prompt context.
pub async fn predict_questions(system_prompt: &str, cfg: &AiConfig<'_>) -> Vec<String> {
    let prompt = format!(
        "Based on the candidate background and job description below, list exactly 8 likely interview questions the interviewer might ask. Output one question per line, numbered 1-8. Mix behavioral, technical, and culture-fit questions. Focus on areas where the candidate's specific experience intersects with the role requirements — tailor questions to their actual background.\n\n{}",
        trunc(&system_prompt, 8000)
    );

    match call_ai_quality(cfg, &prompt, 600).await {
        Ok(text) => text
            .lines()
            .filter_map(|l| {
                let l = l.trim();
                // Strip leading number+dot: "1. question" or "1) question"
                let stripped = l
                    .trim_start_matches(|c: char| c.is_ascii_digit())
                    .trim_start_matches(['.', ')', ' '].as_ref())
                    .trim();
                if stripped.len() > 10 && stripped.contains('?') { Some(stripped.to_string()) } else { None }
            })
            .take(8)
            .collect(),
        Err(e) => {
            tracing::warn!("Question prediction failed: {}", e);
            vec![]
        }
    }
}

#[derive(serde::Serialize)]
pub struct DebriefResult {
    pub summary: String,
    pub strong_points: Vec<String>,
    pub improvement_areas: Vec<String>,
    pub followup_email: Vec<String>,
    pub followup_email_draft: String,
}

/// Generate a post-interview debrief from transcript, suggestions, and session stats.
/// Does NOT generate the email draft (call generate_followup_email separately for that).
pub async fn generate_debrief(
    transcript_text: &str,
    suggestions_text: &str,
    session_context: &str,
    cfg: &AiConfig<'_>,
) -> Result<DebriefResult> {
    let has_real_transcript = transcript_text.lines()
        .any(|l| l.starts_with("You:") && l.len() > 10);

    let focus_note = if !has_real_transcript {
        "Note: This appears to be a preparation/practice session — no live interview transcript was captured. Focus your feedback on the questions reviewed and the preparation quality rather than actual spoken answers."
    } else {
        "This is a live interview session."
    };

    let prompt = format!(
        "You are analyzing a job interview session. {}\n\nRespond in EXACTLY this format (use these exact section headers, no extra text outside them):\n\nSUMMARY:\n[2-3 sentence overall assessment. If no live transcript, assess preparation quality and readiness.]\n\nSTRONG:\n• [specific strength observed]\n• [specific strength observed]\n\nIMPROVE:\n• [specific, actionable improvement]\n• [specific, actionable improvement]\n\nFOLLOWUP:\n• [key point to reference in thank-you email]\n• [key point to reference in thank-you email]\n\n---\nSESSION DATA:\n{}\n\nTRANSCRIPT:\n{}\n\nQUESTIONS & AI SUGGESTIONS:\n{}",
        focus_note,
        trunc(session_context, 800),
        trunc(transcript_text, 3000),
        trunc(suggestions_text, 1500)
    );

    let text = call_ai_quality(cfg, &prompt, 650).await?;
    Ok(parse_debrief(&text))
}

fn parse_debrief(text: &str) -> DebriefResult {
    let mut summary = String::new();
    let mut strong = Vec::new();
    let mut improve = Vec::new();
    let mut followup = Vec::new();
    let mut email_lines: Vec<String> = Vec::new();
    let mut section = "";

    for line in text.lines() {
        let t = line.trim();
        match t {
            "SUMMARY:" => { section = "summary"; continue; }
            "STRONG:" => { section = "strong"; continue; }
            "IMPROVE:" => { section = "improve"; continue; }
            "FOLLOWUP:" => { section = "followup"; continue; }
            "EMAIL:" => { section = "email"; continue; }
            _ => {}
        }
        // Separator between sections — stop email section
        if t == "---" { section = ""; continue; }

        match section {
            "summary" => {
                if !t.is_empty() {
                    if !summary.is_empty() { summary.push(' '); }
                    summary.push_str(t);
                }
            }
            "strong" => {
                let item = t.trim_start_matches(['•', '-', '*', ' '].as_ref()).trim();
                if !item.is_empty() { strong.push(item.to_string()); }
            }
            "improve" => {
                let item = t.trim_start_matches(['•', '-', '*', ' '].as_ref()).trim();
                if !item.is_empty() { improve.push(item.to_string()); }
            }
            "followup" => {
                let item = t.trim_start_matches(['•', '-', '*', ' '].as_ref()).trim();
                if !item.is_empty() { followup.push(item.to_string()); }
            }
            "email" => {
                // Preserve lines as-is (including blank lines for spacing)
                email_lines.push(line.to_string());
            }
            _ => {}
        }
    }

    // Trim leading/trailing blank lines from email
    while email_lines.first().map(|l| l.trim().is_empty()).unwrap_or(false) {
        email_lines.remove(0);
    }
    while email_lines.last().map(|l| l.trim().is_empty()).unwrap_or(false) {
        email_lines.pop();
    }
    let email_draft = email_lines.join("\n");

    DebriefResult {
        summary: if summary.is_empty() { "Interview completed.".to_string() } else { summary },
        strong_points: strong,
        improvement_areas: improve,
        followup_email: followup,
        followup_email_draft: email_draft,
    }
}

/// Generate a follow-up/thank-you email draft. Called lazily after the initial debrief.
pub async fn generate_followup_email(
    transcript_text: &str,
    followup_bullets: &[String],
    cfg: &AiConfig<'_>,
) -> Result<String> {
    let bullets = if followup_bullets.is_empty() {
        "Reference specific topics from the interview.".to_string()
    } else {
        followup_bullets.iter().map(|b| format!("• {b}")).collect::<Vec<_>>().join("\n")
    };
    let transcript_snippet = trunc(transcript_text, 600);
    let prompt = format!(
        "Write a concise professional thank-you email after a job interview. Use [Your Name] and [Interviewer Name] as placeholders.\n\nKey points:\n{}\n\nInterview context:\n{}\n\nOutput format — line 1: Subject: ..., blank line, then the email body. Be warm but brief (3 short paragraphs max).",
        bullets,
        transcript_snippet
    );
    call_ai_quality(cfg, &prompt, 420).await
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct AnswerFeedbackResult {
    pub coaching: String,
    pub missed_followup: bool,
    pub missed_metric: bool,
}

/// Evaluate what the candidate said against the question and suggestion.
pub async fn generate_answer_feedback(
    question: &str,
    candidate_answer: &str,
    suggestion: &str,
    cfg: &AiConfig<'_>,
) -> Result<AnswerFeedbackResult> {
    let prompt = format!(
        "The interviewer asked: \"{}\"\n\nThe AI suggested the candidate say:\n{}\n\nThe candidate actually said: \"{}\"\n\nAnalyze the candidate's answer. Respond in EXACTLY this format:\n\nCOACHING: [1-2 sentence coaching note — be specific, reference what they said or missed]\nMISSED_FOLLOWUP: [yes/no — did they forget to ask a follow-up question to the interviewer?]\nMISSED_METRIC: [yes/no — did they fail to mention a specific number, metric, or measurable outcome?]",
        question, suggestion, candidate_answer
    );

    let text = call_ai(cfg, &prompt, 200).await?;

    let mut coaching = String::new();
    let mut missed_followup = false;
    let mut missed_metric = false;

    for line in text.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("COACHING:") {
            coaching = rest.trim().to_string();
        } else if let Some(rest) = t.strip_prefix("MISSED_FOLLOWUP:") {
            missed_followup = rest.trim().to_lowercase().contains("yes");
        } else if let Some(rest) = t.strip_prefix("MISSED_METRIC:") {
            missed_metric = rest.trim().to_lowercase().contains("yes");
        }
    }

    Ok(AnswerFeedbackResult {
        coaching: if coaching.is_empty() { "Good answer.".to_string() } else { coaching },
        missed_followup,
        missed_metric,
    })
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct CompanyBrief {
    pub name: String,
    pub what_they_do: String,
    pub products: Vec<String>,
    pub culture: String,
    pub recent_news: String,
    pub why_join: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct InterviewerSummary {
    pub name: String,
    pub role: String,
    pub background: String,
    pub tenure: String,
    pub rapport_tips: Vec<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SalaryTactics {
    pub early_round: String,
    pub reveal: String,
    pub direct_ask: String,
    pub total_package: String,
    pub counter: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct PracticeScore {
    pub score: u8,          // 0-100
    pub star_complete: bool,
    pub has_metric: bool,
    pub length_ok: bool,
    pub coaching: String,
    pub strong: String,
}

pub async fn generate_company_brief(company_info: &str, cfg: &AiConfig<'_>) -> CompanyBrief {
    if company_info.trim().is_empty() {
        return CompanyBrief {
            name: String::new(), what_they_do: String::new(),
            products: vec![], culture: String::new(),
            recent_news: String::new(), why_join: String::new(),
        };
    }
    let prompt = format!(
        "Based on this company website content, extract a structured brief for a job candidate preparing for an interview.\n\nRespond in EXACTLY this format:\nNAME: [company name]\nWHAT: [1-2 sentences on what the company does]\nPRODUCTS: [product1] | [product2] | [product3]\nCULTURE: [1 sentence on work culture/values]\nNEWS: [1 sentence on recent notable news or achievements, or 'Not found']\nJOIN: [1 compelling reason why someone would want to work there]\n\n---\n{}",
        trunc(&company_info, 5000)
    );
    let text = call_ai(cfg, &prompt, 400).await.unwrap_or_default();
    let mut brief = CompanyBrief {
        name: String::new(), what_they_do: String::new(),
        products: vec![], culture: String::new(),
        recent_news: String::new(), why_join: String::new(),
    };
    for line in text.lines() {
        if let Some(v) = line.strip_prefix("NAME:") { brief.name = v.trim().to_string(); }
        else if let Some(v) = line.strip_prefix("WHAT:") { brief.what_they_do = v.trim().to_string(); }
        else if let Some(v) = line.strip_prefix("PRODUCTS:") {
            brief.products = v.split('|').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        }
        else if let Some(v) = line.strip_prefix("CULTURE:") { brief.culture = v.trim().to_string(); }
        else if let Some(v) = line.strip_prefix("NEWS:") { brief.recent_news = v.trim().to_string(); }
        else if let Some(v) = line.strip_prefix("JOIN:") { brief.why_join = v.trim().to_string(); }
    }
    brief
}

pub async fn generate_interviewer_summary(linkedin_text: &str, cfg: &AiConfig<'_>) -> Vec<InterviewerSummary> {
    if linkedin_text.trim().is_empty() { return vec![]; }
    // Split by separator if multiple interviewers
    let profiles: Vec<&str> = linkedin_text.split("---INTERVIEWER---")
        .map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    let mut results = Vec::new();
    for profile in profiles.iter() {
        // Heuristic name fallback: first non-empty line
        let heuristic_name: String = profile.lines()
            .map(|l| l.trim())
            .find(|l| !l.is_empty())
            .unwrap_or("")
            .to_string();
        let prompt = format!(
            "Based on this LinkedIn profile, create a brief for a job candidate to help build rapport.\n\nRespond in EXACTLY this format:\nNAME: [person's full name — extract from the profile text, do NOT output 'Unknown']\nROLE: [current job title]\nBACKGROUND: [1 sentence on their career background]\nTENURE: [how long at current company, or 'Unknown']\nRAPPORT1: [keyword] specific rapport tip based on their background or interests\nRAPPORT2: [keyword] different rapport tip — a shared topic, career journey angle, or company connection\nRAPPORT3: [keyword] a third rapport tip — e.g. a question to ask them, or a compliment on an achievement\n\nFor each RAPPORT line, '[keyword]' must be a 1-3 word phrase summarising the tip topic (e.g. '[engineering roots]', '[shared industry]', '[recent promotion]').\n\n---\n{}",
            trunc(&profile, 2000)
        );
        let text = call_ai(cfg, &prompt, 350).await.unwrap_or_default();
        let mut s = InterviewerSummary {
            name: String::new(), role: String::new(),
            background: String::new(), tenure: String::new(), rapport_tips: vec![],
        };
        for line in text.lines() {
            if let Some(v) = line.strip_prefix("NAME:") { s.name = v.trim().to_string(); }
            else if let Some(v) = line.strip_prefix("ROLE:") { s.role = v.trim().to_string(); }
            else if let Some(v) = line.strip_prefix("BACKGROUND:") { s.background = v.trim().to_string(); }
            else if let Some(v) = line.strip_prefix("TENURE:") { s.tenure = v.trim().to_string(); }
            else if let Some(v) = line.strip_prefix("RAPPORT1:") { let t = v.trim().to_string(); if !t.is_empty() { s.rapport_tips.push(t); } }
            else if let Some(v) = line.strip_prefix("RAPPORT2:") { let t = v.trim().to_string(); if !t.is_empty() { s.rapport_tips.push(t); } }
            else if let Some(v) = line.strip_prefix("RAPPORT3:") { let t = v.trim().to_string(); if !t.is_empty() { s.rapport_tips.push(t); } }
        }
        // Fallback: use heuristic name if AI returned nothing useful
        if s.name.is_empty() || s.name.to_lowercase() == "unknown" {
            s.name = heuristic_name;
        }
        results.push(s);
    }
    results
}

pub async fn extract_jd_keywords(job_description: &str, cfg: &AiConfig<'_>) -> Vec<String> {
    if job_description.trim().is_empty() { return vec![]; }
    let prompt = format!(
        "Extract the 12 most important keywords and skill phrases from this job description that a candidate should mention during their interview.\n\nOutput ONLY one keyword/phrase per line, no numbers, no bullets, no extra text. Focus on: technical skills, soft skills, domain knowledge, and key responsibilities.\n\n---\n{}",
        trunc(&job_description, 3000)
    );
    match call_ai(cfg, &prompt, 300).await {
        Ok(text) => text.lines()
            .map(|l| {
                let l = l.trim();
                // Strip bullet/number prefixes
                let l = l.trim_start_matches(|c: char| c == '-' || c == '*' || c == '•' || c == '·' || c.is_ascii_digit() || c == '.' || c == ')');
                let l = l.trim();
                // Strip "Key skill: ", "Technical skill: ", "Soft skill: " etc.
                // that the LLM adds despite being told not to
                if let Some(colon) = l.find(':') {
                    let prefix = l[..colon].to_lowercase();
                    if prefix.contains("skill") || prefix.contains("keyword") || prefix.contains("competency")
                        || prefix.contains("knowledge") || prefix.contains("responsibilit")
                        || prefix.contains("experience") || prefix.contains("qualification")
                        || prefix.contains("domain") || prefix.contains("role")
                    {
                        return l[colon + 1..].trim().to_string();
                    }
                }
                l.to_string()
            })
            .filter(|l| !l.is_empty() && l.len() < 60)
            .take(15)
            .collect(),
        Err(_) => vec![],
    }
}

pub async fn predict_next_questions(
    transcript_context: &str,
    system_prompt: &str,
    cfg: &AiConfig<'_>,
) -> Vec<String> {
    let prompt = format!(
        "Based on this interview conversation so far and the candidate's background, predict the 3 most likely questions the interviewer will ask next.\n\nOutput ONLY 3 questions, one per line, no numbers or bullets.\n\nCANDIDATE BACKGROUND (summary):\n{}\n\nCONVERSATION SO FAR:\n{}",
        trunc(&system_prompt, 1500),
        trunc(&transcript_context, 2000)
    );
    match call_ai(cfg, &prompt, 300).await {
        Ok(text) => text.lines()
            .map(|l| l.trim().trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ')' || c == ' ').to_string())
            .filter(|l| l.len() > 10)
            .take(3)
            .collect(),
        Err(_) => vec![],
    }
}

pub async fn extract_jd_location(job_description: &str, cfg: &AiConfig<'_>) -> String {
    if job_description.trim().is_empty() { return String::new(); }
    let prompt = format!(
        "Extract the job location from this job description. Return ONLY the location as a city, region, or 'Remote' — nothing else. If no location is mentioned, return an empty string.\n\nExamples of valid output: 'San Francisco, CA' | 'London, UK' | 'Remote' | 'New York, NY (Hybrid)'\n\n---\n{}",
        trunc(job_description, 1500)
    );
    match call_ai(cfg, &prompt, 30).await {
        Ok(text) => {
            let loc = text.trim().to_string();
            if loc.len() > 80 { String::new() } else { loc }
        }
        Err(_) => String::new(),
    }
}

pub async fn generate_salary_tactics(role_context: &str, location: &str, jd_snippet: &str, candidate_context: &str, company_info: &str, cfg: &AiConfig<'_>) -> SalaryTactics {
    // Determine salary period AND market anchor so the LLM has a realistic reference range.
    // Small models hallucinate without a concrete anchor — these figures are conservative midpoints.
    let (period_label, period_instruction) = if location.is_empty() {
        ("per year".to_string(), "Express the range as an annual figure. Typical mid-level range: $70,000–$100,000 per year. Adjust for seniority and company stage.".to_string())
    } else {
        let loc = location.to_lowercase();
        // Market-specific anchors: (match string, period, instruction with realistic range)
        let market_anchors: &[(&str, &str, &str)] = &[
            ("israel",      "per month", "CRITICAL: MONTHLY salaries. Typical ranges (ILS/month): junior ₪15,000–₪20,000 · mid ₪20,000–₪35,000 · senior ₪30,000–₪50,000. Do NOT output an annual figure or exceed these realistic ceilings."),
            ("tel aviv",    "per month", "CRITICAL: MONTHLY salaries. Typical ranges (ILS/month): junior ₪15,000–₪20,000 · mid ₪22,000–₪38,000 · senior ₪35,000–₪55,000 (tech premium). Do NOT output an annual figure."),
            ("netherlands", "per month", "CRITICAL: MONTHLY salaries. Typical ranges (EUR/month): junior €2,500–€3,500 · mid €3,500–€5,500 · senior €5,000–€8,000. Do NOT output an annual figure."),
            ("germany",     "per month", "CRITICAL: MONTHLY salaries. Typical ranges (EUR/month): junior €2,800–€3,800 · mid €3,800–€5,500 · senior €5,000–€7,500. Do NOT output an annual figure."),
            ("france",      "per month", "CRITICAL: MONTHLY salaries. Typical ranges (EUR/month): junior €2,300–€3,200 · mid €3,200–€4,800 · senior €4,500–€6,500. Do NOT output an annual figure."),
            ("spain",       "per month", "CRITICAL: MONTHLY salaries. Typical ranges (EUR/month): junior €1,800–€2,500 · mid €2,500–€3,800 · senior €3,500–€5,500. Do NOT output an annual figure."),
            ("dubai",       "per month", "CRITICAL: MONTHLY salaries (tax-free). Typical ranges (AED/month): junior AED 8,000–12,000 · mid AED 12,000–22,000 · senior AED 20,000–35,000. Do NOT output an annual figure."),
            ("uae",         "per month", "CRITICAL: MONTHLY salaries (tax-free). Typical ranges (AED/month): junior AED 8,000–12,000 · mid AED 12,000–22,000 · senior AED 20,000–35,000. Do NOT output an annual figure."),
            ("qatar",       "per month", "CRITICAL: MONTHLY salaries (tax-free). Typical ranges (QAR/month): junior QAR 8,000–14,000 · mid QAR 14,000–25,000 · senior QAR 22,000–40,000. Do NOT output an annual figure."),
            ("saudi",       "per month", "CRITICAL: MONTHLY salaries. Typical ranges (SAR/month): junior SAR 8,000–14,000 · mid SAR 14,000–25,000 · senior SAR 22,000–38,000. Do NOT output an annual figure."),
            ("japan",       "per month", "CRITICAL: MONTHLY salaries. Typical ranges (JPY/month): junior ¥250,000–¥350,000 · mid ¥350,000–¥550,000 · senior ¥500,000–¥800,000. Do NOT output an annual figure."),
            ("australia",   "per year",  "CRITICAL: ANNUAL salaries. Typical ranges (AUD/year): junior A$55,000–A$75,000 · mid A$75,000–A$110,000 · senior A$100,000–A$150,000. Do NOT output a monthly figure."),
            ("canada",      "per year",  "CRITICAL: ANNUAL salaries. Typical ranges (CAD/year): junior C$50,000–C$70,000 · mid C$70,000–C$100,000 · senior C$95,000–C$140,000. Do NOT output a monthly figure."),
            ("united kingdom", "per year", "CRITICAL: ANNUAL salaries. Typical ranges (GBP/year): junior £28,000–£38,000 · mid £38,000–£60,000 · senior £55,000–£85,000. Do NOT output a monthly figure."),
            ("london",      "per year",  "CRITICAL: ANNUAL salaries. Typical ranges (GBP/year): junior £32,000–£45,000 · mid £45,000–£70,000 · senior £65,000–£100,000 (London premium). Do NOT output a monthly figure."),
        ];
        let monthly_fallback_markets = ["switzerland", "austria", "poland", "czech", "hungary",
            "romania", "sweden", "norway", "denmark", "finland", "korea", "china",
            "brazil", "argentina", "mexico", "belgium", "portugal", "croatia", "slovakia", "italy"];
        if let Some((_, period, instr)) = market_anchors.iter().find(|(m, _, _)| loc.contains(m)) {
            (period.to_string(), instr.to_string())
        } else if monthly_fallback_markets.iter().any(|m| loc.contains(m)) {
            ("per month".to_string(), "CRITICAL: This location uses MONTHLY salaries. Output a realistic MONTHLY figure in local currency. Do NOT output an annual figure.".to_string())
        } else {
            ("per year".to_string(), "CRITICAL: This location uses ANNUAL salaries. Output a realistic ANNUAL figure in local currency (e.g. £45,000–£55,000 per year). Do NOT output a monthly figure.".to_string())
        }
    };
    let location_line = if location.is_empty() {
        format!("SALARY PERIOD: Annual. {}\n", period_instruction)
    } else {
        format!("LOCATION: {}. Local currency and market rates apply. SALARY PERIOD: {}. {}\n",
            location, period_label, period_instruction)
    };

    // Quick call: conversational tactics only (no number needed — small context, fast)
    let quick_prompt = format!(
        "Generate tactful salary negotiation language. Tone: collaborative and confident, never evasive.\n\
{}\
ROLE: {}\n\n\
Respond in EXACTLY this format (one line each):\n\
EARLY: [Early rounds — too soon to anchor. Warm, no number. Focus on fit and openness to learning about the full package. 1-2 sentences.]\n\
REVEAL: [They press for a number. Flip the ask — politely invite the interviewer to share their budgeted range first. Warm, not a refusal. 1-2 sentences.]\n\
PACKAGE: [How the candidate thinks about comp holistically — base, equity, benefits. Phrased as 'the way I think about it...'. 1-2 sentences.]\n\
COUNTER: [Offer came in below expectations. Name the gap calmly as something to work through together, never an ultimatum. 2 sentences.]",
        location_line, trunc(role_context, 200)
    );

    // Direct Ask call: needs calibrated range — full context
    let jd_line = if jd_snippet.is_empty() { String::new() } else {
        format!("JOB DESCRIPTION:\n{}\n\n", trunc(jd_snippet, 1500))
    };
    let candidate_line = if candidate_context.is_empty() { String::new() } else {
        format!("CANDIDATE BACKGROUND (use to calibrate seniority and range — do not anchor above the candidate's actual level):\n{}\n\n", trunc(candidate_context, 2000))
    };
    let company_line = if company_info.is_empty() { String::new() } else {
        format!("COMPANY INFO (infer stage and pay band — startup/scale-up/enterprise):\n{}\n\n", trunc(company_info, 800))
    };
    let direct_prompt = format!(
        "Generate one salary negotiation line for when the interviewer insists on a number.\n\
Tone: confident and collaborative.\n\n\
CALIBRATION RULES (follow in order):\n\
1. SENIORITY: Count years of relevant experience from the candidate background. < 2 yrs = junior · 2–5 yrs = mid · 5+ yrs = senior. Pick ONLY that one tier from the market range table — do NOT blend tiers or anchor at the top of a higher tier.\n\
2. COMPANY TYPE: Agencies, SMEs, and non-tech companies pay 20–30% less than pure-tech firms for equivalent roles. Adjust down if the company is an agency or marketing firm.\n\
3. JD BAND: If the JD lists a salary range or band, use that as the primary anchor and ignore the market table.\n\
4. OUTPUT: A narrow 2-number range (width ≤ ₪5,000 / €2,000 / £3,000). No placeholders — real currency + real numbers only.\n\n\
{}{}{}{}\
Output ONLY this single line, nothing before or after it:\n\
DIRECT: [Warm acknowledgement. Then: 'Based on my research and the scope of this role, I'd expect something around [REAL RANGE].' Then invite dialogue on the full package. 2-3 sentences total.]",
        location_line, jd_line, candidate_line, company_line
    );

    // Run both calls in parallel using the fast path (8b-instant on Groq, Ollama last not second)
    let (quick_text, direct_text) = tokio::join!(
        call_ai_fast(cfg, "You are a salary negotiation coach. Output only the exact labeled lines requested, no preamble.", &quick_prompt),
        call_ai_fast(cfg, "You are a salary range estimator. Output only the single DIRECT: line requested, no preamble.", &direct_prompt),
    );
    let quick_text = quick_text.unwrap_or_default();
    let direct_text = direct_text.unwrap_or_default();

    let mut t = SalaryTactics {
        early_round: String::new(), reveal: String::new(),
        direct_ask: String::new(), total_package: String::new(), counter: String::new(),
    };
    for line in quick_text.lines().chain(direct_text.lines()) {
        if let Some(v) = line.strip_prefix("EARLY:")        { t.early_round   = v.trim().to_string(); }
        else if let Some(v) = line.strip_prefix("REVEAL:")  { t.reveal        = v.trim().to_string(); }
        else if let Some(v) = line.strip_prefix("DIRECT:")  { t.direct_ask    = v.trim().to_string(); }
        else if let Some(v) = line.strip_prefix("PACKAGE:") { t.total_package = v.trim().to_string(); }
        else if let Some(v) = line.strip_prefix("COUNTER:") { t.counter       = v.trim().to_string(); }
    }
    if t.early_round.is_empty()   { t.early_round   = "I'm focused on finding the right fit — I'm open to learning more about the full package as things progress.".to_string(); }
    if t.reveal.is_empty()        { t.reveal        = "Before I give you a number, could you share the range you have budgeted for this role? That way we can make sure we're starting from the same place.".to_string(); }
    if t.direct_ask.is_empty()    { t.direct_ask    = "I've researched the market for this kind of role and have a figure in mind based on the scope and my experience — I'd love to share that once I understand the full package, including bonus and equity.".to_string(); }
    if t.total_package.is_empty() { t.total_package = "The way I think about it, compensation is the whole package — base, equity, and benefits together. I'd love to understand the full offer before focusing on any single number.".to_string(); }
    if t.counter.is_empty()       { t.counter       = "I appreciate the offer — it's a bit below the range I'd anticipated based on the scope and market. I'd love to find a way to bridge that gap together.".to_string(); }
    t
}

pub async fn score_practice_answer(
    question: &str,
    answer: &str,
    system_prompt: &str,
    cfg: &AiConfig<'_>,
) -> PracticeScore {
    let prompt = format!(
        "Score this practice interview answer.\n\nQuestion: \"{}\"\nAnswer: \"{}\"\n\nCandidate background:\n{}\n\nRespond in EXACTLY this format:\nSCORE: [0-100 integer]\nSTAR: [yes/no — does the answer follow Situation/Task/Action/Result structure?]\nMETRIC: [yes/no — does it include a specific number, %, or measurable outcome?]\nLENGTH: [yes/no — is the answer an appropriate length, under 90 seconds to speak?]\nSTRONG: [1 sentence on what was done well]\nCOACH: [1-2 sentences of specific coaching to improve this answer]",
        question, answer, trunc(&system_prompt, 1000)
    );
    let text = call_ai(cfg, &prompt, 300).await.unwrap_or_default();
    let mut score = PracticeScore {
        score: 50, star_complete: false, has_metric: false,
        length_ok: true, coaching: String::new(), strong: String::new(),
    };
    for line in text.lines() {
        let t = line.trim();
        if let Some(v) = t.strip_prefix("SCORE:") {
            score.score = v.trim().parse().unwrap_or(50);
        } else if let Some(v) = t.strip_prefix("STAR:") {
            score.star_complete = v.trim().to_lowercase().contains("yes");
        } else if let Some(v) = t.strip_prefix("METRIC:") {
            score.has_metric = v.trim().to_lowercase().contains("yes");
        } else if let Some(v) = t.strip_prefix("LENGTH:") {
            score.length_ok = v.trim().to_lowercase().contains("yes");
        } else if let Some(v) = t.strip_prefix("STRONG:") {
            score.strong = v.trim().to_string();
        } else if let Some(v) = t.strip_prefix("COACH:") {
            score.coaching = v.trim().to_string();
        }
    }
    score
}

pub async fn extract_next_steps(transcript_text: &str, cfg: &AiConfig<'_>) -> Vec<String> {
    if transcript_text.trim().is_empty() { return vec![]; }
    let prompt = format!(
        "From this interview transcript, extract all mentioned next steps, timelines, and follow-up actions. Include things like: when they'll get back to you, who to contact, what the hiring process looks like, any requested follow-ups.\n\nOutput one item per line, no bullets or numbers. If none found, output 'No specific next steps mentioned.'\n\n---\n{}",
        trunc(&transcript_text, 4000)
    );
    match call_ai(cfg, &prompt, 300).await {
        Ok(text) => text.lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .take(6)
            .collect(),
        Err(_) => vec![],
    }
}

#[derive(serde::Serialize)]
pub struct VocalSentiment {
    pub tone: String,           // e.g. "confident", "hesitant", "nervous"
    pub pace: String,           // e.g. "good pace (145 wpm)", "too fast (210 wpm)"
    pub confidence_score: u8,   // 0-100
    pub coaching: String,       // 1-2 sentence delivery coaching
    pub fillers_noted: String,  // e.g. "3 fillers detected (um ×2, like ×1)" or ""
}

pub async fn assess_vocal_delivery(
    question: &str,
    transcript: &str,
    duration_seconds: f32,
    word_count: u32,
    filler_count: u32,
    filler_detail: &str,
    cfg: &AiConfig<'_>,
) -> VocalSentiment {
    let wpm = if duration_seconds > 0.0 {
        (word_count as f32 / duration_seconds * 60.0).round() as u32
    } else { 0 };

    let pace_note = match wpm {
        0..=80 => "too slow",
        81..=120 => "slightly slow",
        121..=160 => "good pace",
        161..=200 => "slightly fast",
        _ => "too fast",
    };

    let prompt = format!(
        "Assess the vocal delivery of this practice interview answer.\n\nQuestion asked: \"{}\"\n\nCandidate said:\n\"{}\"\n\nSpeaking metrics:\n- Duration: {:.0}s\n- Words per minute: {} ({})\n- Filler words: {} ({})\n\nRespond in EXACTLY this format:\nTONE: [one word: confident / hesitant / nervous / enthusiastic / flat]\nSCORE: [0-100 integer — overall vocal delivery score]\nCOACHING: [1-2 specific sentences coaching the candidate on their delivery — reference what they said or the metrics]\n\nFocus on: confidence in language choices (hedging words like 'I think', 'maybe', 'sort of' vs. direct statements), filler word usage, answer structure, and whether they sounded prepared.",
        question, transcript, duration_seconds, wpm, pace_note, filler_count, filler_detail
    );

    let text = call_ai(cfg, &prompt, 200).await.unwrap_or_default();

    let mut tone = "neutral".to_string();
    let mut confidence_score: u8 = 50;
    let mut coaching = String::new();

    for line in text.lines() {
        let t = line.trim();
        if let Some(v) = t.strip_prefix("TONE:") { tone = v.trim().to_lowercase(); }
        else if let Some(v) = t.strip_prefix("SCORE:") { confidence_score = v.trim().parse().unwrap_or(50); }
        else if let Some(v) = t.strip_prefix("COACHING:") { coaching = v.trim().to_string(); }
    }

    let pace = format!("{} ({} wpm)", pace_note, wpm);
    let fillers_noted = if filler_count > 0 {
        format!("{} filler{} detected ({})", filler_count, if filler_count == 1 { "" } else { "s" }, filler_detail)
    } else {
        String::new()
    };

    VocalSentiment { tone, pace, confidence_score, coaching, fillers_noted }
}
