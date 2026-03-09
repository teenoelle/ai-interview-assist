/// Integration tests for external API connectivity.
/// These are ignored by default — they make real network calls and consume quota.
///
/// Run all: cargo test -p server --test api_integration -- --ignored --nocapture
/// Run one: cargo test -p server --test api_integration gemini -- --ignored --nocapture

use std::time::Duration;

fn load_env() {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    // Walk up to workspace root and load .env
    let env_path = manifest.ancestors().find_map(|p| {
        let f = p.join(".env");
        if f.exists() { Some(f) } else { None }
    });
    if let Some(path) = env_path {
        let _ = dotenvy::from_path(path);
    }
}

fn get_key(var: &str) -> Option<String> {
    std::env::var(var).ok().filter(|k| !k.trim().is_empty())
}

fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap()
}

// ── Gemini ────────────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore]
async fn gemini_api_reachable() {
    load_env();
    let key = get_key("GEMINI_API_KEY").expect("GEMINI_API_KEY not set");

    let body = serde_json::json!({
        "contents": [{"parts": [{"text": "Reply with exactly one word: hello"}]}],
        "generationConfig": {"maxOutputTokens": 5}
    });

    let resp = http_client()
        .post(format!(
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={}",
            key
        ))
        .json(&body)
        .send()
        .await
        .expect("request failed");

    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    assert!(status.is_success(), "Gemini HTTP {status}: {text}");
    println!("Gemini OK: {}", &text[..text.len().min(120)]);
}

// ── Groq LLM ──────────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore]
async fn groq_llm_api_reachable() {
    load_env();
    let key = get_key("GROQ_API_KEY").expect("GROQ_API_KEY not set");

    let body = serde_json::json!({
        "model": "llama-3.3-70b-versatile",
        "messages": [{"role": "user", "content": "Reply with one word: hello"}],
        "max_tokens": 5
    });

    let resp = http_client()
        .post("https://api.groq.com/openai/v1/chat/completions")
        .bearer_auth(&key)
        .json(&body)
        .send()
        .await
        .expect("request failed");

    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    assert!(status.is_success(), "Groq LLM HTTP {status}: {text}");
    println!("Groq LLM OK: {}", &text[..text.len().min(120)]);
}

// ── Groq Whisper ──────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore]
async fn groq_whisper_api_reachable() {
    load_env();
    let key = get_key("GROQ_API_KEY").expect("GROQ_API_KEY not set");

    let wav = build_silent_wav(16000, 1);
    let form = reqwest::multipart::Form::new()
        .part(
            "file",
            reqwest::multipart::Part::bytes(wav)
                .file_name("audio.wav")
                .mime_str("audio/wav")
                .unwrap(),
        )
        .text("model", "whisper-large-v3")
        .text("response_format", "text");

    let resp = http_client()
        .post("https://api.groq.com/openai/v1/audio/transcriptions")
        .bearer_auth(&key)
        .multipart(form)
        .send()
        .await
        .expect("request failed");

    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    assert!(status.is_success(), "Groq Whisper HTTP {status}: {text}");
    println!("Groq Whisper OK: {:?}", text.trim());
}

// ── OpenRouter ────────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore]
async fn openrouter_api_reachable() {
    load_env();
    let key = get_key("OPENROUTER_API_KEY").expect("OPENROUTER_API_KEY not set");

    // Test the primary free model
    let body = serde_json::json!({
        "model": "meta-llama/llama-3.3-70b-instruct:free",
        "messages": [{"role": "user", "content": "Reply with one word: hello"}],
        "max_tokens": 5
    });

    let resp = http_client()
        .post("https://openrouter.ai/api/v1/chat/completions")
        .bearer_auth(&key)
        .json(&body)
        .send()
        .await
        .expect("request failed");

    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    assert!(status.is_success(), "OpenRouter HTTP {status}: {text}");
    println!("OpenRouter OK: {}", &text[..text.len().min(120)]);
}

// ── Cerebras ──────────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore]
async fn cerebras_api_reachable() {
    load_env();
    let key = get_key("CEREBRAS_API_KEY").expect("CEREBRAS_API_KEY not set");

    let body = serde_json::json!({
        "model": "llama-3.3-70b",
        "messages": [{"role": "user", "content": "Reply with one word: hello"}],
        "max_tokens": 5
    });

    let resp = http_client()
        .post("https://api.cerebras.ai/v1/chat/completions")
        .bearer_auth(&key)
        .json(&body)
        .send()
        .await
        .expect("request failed");

    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    assert!(status.is_success(), "Cerebras HTTP {status}: {text}");
    println!("Cerebras OK: {}", &text[..text.len().min(120)]);
}

// ── Mistral ───────────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore]
async fn mistral_api_reachable() {
    load_env();
    let key = get_key("MISTRAL_API_KEY").expect("MISTRAL_API_KEY not set");

    let body = serde_json::json!({
        "model": "mistral-small-latest",
        "messages": [{"role": "user", "content": "Reply with one word: hello"}],
        "max_tokens": 5
    });

    let resp = http_client()
        .post("https://api.mistral.ai/v1/chat/completions")
        .bearer_auth(&key)
        .json(&body)
        .send()
        .await
        .expect("request failed");

    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    assert!(status.is_success(), "Mistral HTTP {status}: {text}");
    println!("Mistral OK: {}", &text[..text.len().min(120)]);
}

// ── Web crawler ───────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore]
async fn crawler_fetches_known_site() {
    let result = context::crawler::crawl_website("https://example.com", 2)
        .await
        .expect("crawl failed");
    assert!(!result.is_empty(), "crawl returned empty content");
    println!("Crawler OK ({} chars)", result.len());
}

// ── PDF parser ────────────────────────────────────────────────────────────────

#[test]
fn docx_parser_handles_empty_bytes() {
    // Should return an error gracefully, not panic
    let result = context::pdf::extract_docx_text(b"not a zip");
    assert!(result.is_err());
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn build_silent_wav(sample_rate: u32, seconds: u32) -> Vec<u8> {
    let num_samples = sample_rate * seconds;
    let data_size = num_samples * 2;
    let file_size = 36 + data_size;
    let mut wav = Vec::with_capacity((file_size + 8) as usize);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&file_size.to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_size.to_le_bytes());
    wav.extend(std::iter::repeat(0u8).take(data_size as usize));
    wav
}
