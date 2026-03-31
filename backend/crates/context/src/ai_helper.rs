use anyhow::Result;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

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
    let resp = reqwest::Client::new()
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
    let resp = reqwest::Client::new()
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
    let resp = reqwest::Client::new()
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
    let resp = reqwest::Client::new()
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

/// Call AI providers in order: Groq key1 → Groq key2 → Ollama → Claude → Gemini.
pub async fn call_ai(cfg: &AiConfig<'_>, prompt: &str, max_tokens: u32) -> Result<String> {
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

    // Ollama (local, no quota)
    if let Some(text) = try_ollama_text(cfg.ollama_url, cfg.ollama_model, prompt, max_tokens).await {
        inc(&cfg.usage, "Ollama");
        return Ok(text);
    }

    // Claude
    if let Some(key) = cfg.anthropic_key {
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
            inc(&cfg.usage, "Claude");
            return Ok(j["content"][0]["text"].as_str().unwrap_or("").trim().to_string());
        }
        tracing::debug!("Claude returned {}, trying next provider", resp.status());
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
    let resp = reqwest::Client::new()
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
        let resp = reqwest::Client::new()
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
        let resp = reqwest::Client::new()
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
        let resp = reqwest::Client::new()
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
            inc(&cfg.usage, "Claude");
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
    let resp = reqwest::Client::new()
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

/// Call AI with a system prompt: Claude Haiku → Groq key1 → Groq key2 → Ollama → Gemini.
pub async fn call_ai_simple(cfg: &AiConfig<'_>, system_prompt: &str, user_prompt: &str) -> Result<String> {
    let max_tokens = 400u32;

    // Claude Haiku — primary
    if let Some(key) = cfg.anthropic_key {
        let body = json!({
            "model": "claude-haiku-4-5-20251001",
            "max_tokens": max_tokens,
            "system": [{"type": "text", "text": system_prompt, "cache_control": {"type": "ephemeral"}}],
            "messages": [{ "role": "user", "content": user_prompt }]
        });
        let resp = reqwest::Client::new()
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
            inc(&cfg.usage, "Claude");
            return Ok(j["content"][0]["text"].as_str().unwrap_or("").trim().to_string());
        }
        tracing::debug!("Claude returned {}, trying next provider", resp.status());
    }

    // Groq key1 — fallback
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
    let resp = reqwest::Client::new()
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

    match call_ai(cfg, &prompt, 600).await {
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

/// Generate a post-interview debrief from transcript and suggestions.
pub async fn generate_debrief(
    transcript_text: &str,
    suggestions_text: &str,
    cfg: &AiConfig<'_>,
) -> Result<DebriefResult> {
    let prompt = format!(
        "You are analyzing a completed job interview. Based on the transcript and AI suggestions below, write a concise debrief.\n\nRespond in EXACTLY this format (use these exact section headers):\n\nSUMMARY:\n[2-3 sentence overall assessment]\n\nSTRONG:\n• [specific thing done well]\n• [specific thing done well]\n\nIMPROVE:\n• [specific area to improve]\n• [specific area to improve]\n\nFOLLOWUP:\n• [point to include in thank-you email]\n• [point to include in thank-you email]\n\nEMAIL:\n[Complete thank-you email, ready to copy and send. Include: Subject line on the first line starting with 'Subject: ', then a blank line, then a proper greeting, 2-3 warm paragraphs referencing specific topics from the interview, a forward-looking close, and a sign-off. Use [Your Name] and [Interviewer Name] as placeholders.]\n\n---\nTRANSCRIPT:\n{}\n\nAI SUGGESTIONS PROVIDED:\n{}",
        trunc(&transcript_text, 4000),
        trunc(&suggestions_text, 2000)
    );

    let text = call_ai(cfg, &prompt, 1400).await?;
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
            .map(|l| l.trim().to_string())
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

pub async fn generate_salary_tactics(role_context: &str, location: &str, jd_snippet: &str, cfg: &AiConfig<'_>) -> SalaryTactics {
    let location_line = if location.is_empty() {
        String::new()
    } else {
        format!("LOCATION: {} — use local market rates and the salary period convention for this location (e.g. annual in the US/UK, monthly in Israel/parts of Europe). Express all ranges using the correct period and local currency symbol.\n", location)
    };
    let jd_line = if jd_snippet.is_empty() {
        String::new()
    } else {
        format!("JOB DESCRIPTION EXCERPT:\n{}\n\n", trunc(jd_snippet, 600))
    };
    let prompt = format!(
        "Generate tactful salary alignment language for a candidate interviewing for this role. \
The tone must be collaborative and confident — never evasive, never demanding, never telling the interviewer how to do their job.\n\n\
{}{}\
Respond in EXACTLY this format:\n\
EARLY: [For early rounds when it's too soon to anchor. Warm and genuine, no specific number. Express focus on finding the right fit and openness to learning more about the full package. 1-2 sentences. Do NOT ask the interviewer to reveal their budget.]\n\
REVEAL: [For when they press for a number but the candidate hasn't anchored yet. The candidate expresses, in their own words, where they are in their thinking — self-referential, not asking the interviewer to do anything. 1 sentence.]\n\
DIRECT: [For when they still want a number. Three beats: (1) acknowledge warmly, (2) give a confident market-researched range using the correct salary period and currency for the location above — e.g. '₪X–₪Y per month' for Israel, '$X–$Y per year' for the US — use a placeholder for the numbers, (3) briefly invite dialogue about the full package. 2-3 sentences.]\n\
PACKAGE: [A genuine expression of how the candidate thinks about comp holistically — base, equity, benefits together. Phrased as 'the way I think about it...' — self-referential, not redirecting the interviewer. 1-2 sentences.]\n\
COUNTER: [If the initial offer comes in below expectations. Name the gap calmly as something to work through together, never an ultimatum. 2 sentences.]\n\n\
ROLE CONTEXT:\n{}",
        location_line, jd_line, trunc(role_context, 400)
    );
    let text = call_ai(cfg, &prompt, 350).await.unwrap_or_default();
    let mut t = SalaryTactics {
        early_round: String::new(), reveal: String::new(),
        direct_ask: String::new(), total_package: String::new(), counter: String::new(),
    };
    for line in text.lines() {
        if let Some(v) = line.strip_prefix("EARLY:")        { t.early_round   = v.trim().to_string(); }
        else if let Some(v) = line.strip_prefix("REVEAL:")  { t.reveal        = v.trim().to_string(); }
        else if let Some(v) = line.strip_prefix("DIRECT:")  { t.direct_ask    = v.trim().to_string(); }
        else if let Some(v) = line.strip_prefix("PACKAGE:") { t.total_package = v.trim().to_string(); }
        else if let Some(v) = line.strip_prefix("COUNTER:") { t.counter       = v.trim().to_string(); }
    }
    if t.early_round.is_empty()   { t.early_round   = "I'm focused on finding the right fit — I'm open to learning more about the full package as things progress.".to_string(); }
    if t.reveal.is_empty()        { t.reveal        = "I've shared where I'm anchored in my thinking — I'd love to understand the range you have in mind so we can make sure we're aligned.".to_string(); }
    if t.direct_ask.is_empty()    { t.direct_ask    = "I've researched the market and based on the scope of the role, I'd expect something in the range of $X–$Y. I'm also thinking about the full picture, so I'd love to understand the complete package.".to_string(); }
    if t.total_package.is_empty() { t.total_package = "I think about compensation holistically — base, equity, and benefits together. I'd welcome understanding the full package before landing on a specific number.".to_string(); }
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
