use anyhow::Result;
use serde_json::{json, Value};

/// Call Claude (if key available) or Gemini to get a plain text response.
pub async fn call_ai(
    prompt: &str,
    gemini_key: &str,
    anthropic_key: Option<&str>,
    max_tokens: u32,
) -> Result<String> {
    if let Some(key) = anthropic_key {
        let body = json!({
            "model": "claude-haiku-4-5-20251001",
            "max_tokens": max_tokens,
            "messages": [{ "role": "user", "content": prompt }]
        });
        let resp = reqwest::Client::new()
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await?;
        if resp.status().is_success() {
            let j: Value = resp.json().await?;
            return Ok(j["content"][0]["text"].as_str().unwrap_or("").trim().to_string());
        }
    }

    // Gemini fallback
    let body = json!({
        "contents": [{ "parts": [{ "text": prompt }] }],
        "generationConfig": { "maxOutputTokens": max_tokens }
    });
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={}",
        gemini_key
    );
    let resp = reqwest::Client::new()
        .post(&url)
        .json(&body)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await?;
    let j: Value = resp.json().await?;
    Ok(j["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string())
}

/// Call AI with a system prompt (for practice question hints).
pub async fn call_ai_simple(
    system_prompt: &str,
    user_prompt: &str,
    gemini_key: &str,
    anthropic_key: Option<&str>,
) -> Result<String> {
    if let Some(key) = anthropic_key {
        let body = json!({
            "model": "claude-haiku-4-5-20251001",
            "max_tokens": 400,
            "system": system_prompt,
            "messages": [{ "role": "user", "content": user_prompt }]
        });
        let resp = reqwest::Client::new()
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await?;
        if resp.status().is_success() {
            let j: Value = resp.json().await?;
            return Ok(j["content"][0]["text"].as_str().unwrap_or("").trim().to_string());
        }
    }
    // Gemini fallback
    let body = json!({
        "system_instruction": { "parts": [{ "text": system_prompt }] },
        "contents": [{ "role": "user", "parts": [{ "text": user_prompt }] }],
        "generationConfig": { "maxOutputTokens": 400 }
    });
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={}",
        gemini_key
    );
    let resp = reqwest::Client::new()
        .post(&url)
        .json(&body)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await?;
    let j: Value = resp.json().await?;
    Ok(j["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string())
}

/// Generate 8 likely interview questions from the system prompt context.
pub async fn predict_questions(
    system_prompt: &str,
    gemini_key: &str,
    anthropic_key: Option<&str>,
) -> Vec<String> {
    let prompt = format!(
        "Based on the candidate background and job description below, list exactly 8 likely interview questions the interviewer might ask. Output one question per line, numbered 1-8. Mix behavioral, technical, and culture-fit questions. Focus on areas where the candidate's experience intersects with the role requirements.\n\n{}",
        &system_prompt[..system_prompt.len().min(4000)]
    );

    match call_ai(&prompt, gemini_key, anthropic_key, 600).await {
        Ok(text) => text
            .lines()
            .filter_map(|l| {
                let l = l.trim();
                // Strip leading number+dot: "1. question" or "1) question"
                let stripped = l
                    .trim_start_matches(|c: char| c.is_ascii_digit())
                    .trim_start_matches(['.', ')', ' '].as_ref())
                    .trim();
                if stripped.len() > 10 { Some(stripped.to_string()) } else { None }
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
}

/// Generate a post-interview debrief from transcript and suggestions.
pub async fn generate_debrief(
    transcript_text: &str,
    suggestions_text: &str,
    gemini_key: &str,
    anthropic_key: Option<&str>,
) -> Result<DebriefResult> {
    let prompt = format!(
        "You are analyzing a completed job interview. Based on the transcript and AI suggestions below, write a concise debrief.\n\nRespond in EXACTLY this format (use these exact section headers):\n\nSUMMARY:\n[2-3 sentence overall assessment]\n\nSTRONG:\n• [specific thing done well]\n• [specific thing done well]\n\nIMPROVE:\n• [specific area to improve]\n• [specific area to improve]\n\nFOLLOWUP:\n• [point to include in thank-you email]\n• [point to include in thank-you email]\n\n---\nTRANSCRIPT:\n{}\n\nAI SUGGESTIONS PROVIDED:\n{}",
        &transcript_text[..transcript_text.len().min(4000)],
        &suggestions_text[..suggestions_text.len().min(2000)]
    );

    let text = call_ai(&prompt, gemini_key, anthropic_key, 800).await?;
    Ok(parse_debrief(&text))
}

fn parse_debrief(text: &str) -> DebriefResult {
    let mut summary = String::new();
    let mut strong = Vec::new();
    let mut improve = Vec::new();
    let mut followup = Vec::new();
    let mut section = "";

    for line in text.lines() {
        let t = line.trim();
        match t {
            "SUMMARY:" => { section = "summary"; continue; }
            "STRONG:" => { section = "strong"; continue; }
            "IMPROVE:" => { section = "improve"; continue; }
            "FOLLOWUP:" => { section = "followup"; continue; }
            _ => {}
        }
        if t.is_empty() { continue; }
        match section {
            "summary" => {
                if !summary.is_empty() { summary.push(' '); }
                summary.push_str(t);
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
            _ => {}
        }
    }

    DebriefResult {
        summary: if summary.is_empty() { "Interview completed.".to_string() } else { summary },
        strong_points: strong,
        improvement_areas: improve,
        followup_email: followup,
    }
}
