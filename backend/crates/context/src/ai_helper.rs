use anyhow::Result;
use serde_json::{json, Value};

/// Safely truncate a UTF-8 string to at most `chars` characters.
/// Prevents panics when slicing strings containing multi-byte Unicode characters.
fn trunc(s: &str, chars: usize) -> &str {
    match s.char_indices().nth(chars) {
        Some((i, _)) => &s[..i],
        None => s,
    }
}

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
        trunc(&system_prompt, 4000)
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
    pub followup_email_draft: String,
}

/// Generate a post-interview debrief from transcript and suggestions.
pub async fn generate_debrief(
    transcript_text: &str,
    suggestions_text: &str,
    gemini_key: &str,
    anthropic_key: Option<&str>,
) -> Result<DebriefResult> {
    let prompt = format!(
        "You are analyzing a completed job interview. Based on the transcript and AI suggestions below, write a concise debrief.\n\nRespond in EXACTLY this format (use these exact section headers):\n\nSUMMARY:\n[2-3 sentence overall assessment]\n\nSTRONG:\n• [specific thing done well]\n• [specific thing done well]\n\nIMPROVE:\n• [specific area to improve]\n• [specific area to improve]\n\nFOLLOWUP:\n• [point to include in thank-you email]\n• [point to include in thank-you email]\n\nEMAIL:\n[Complete thank-you email, ready to copy and send. Include: Subject line on the first line starting with 'Subject: ', then a blank line, then a proper greeting, 2-3 warm paragraphs referencing specific topics from the interview, a forward-looking close, and a sign-off. Use [Your Name] and [Interviewer Name] as placeholders.]\n\n---\nTRANSCRIPT:\n{}\n\nAI SUGGESTIONS PROVIDED:\n{}",
        trunc(&transcript_text, 4000),
        trunc(&suggestions_text, 2000)
    );

    let text = call_ai(&prompt, gemini_key, anthropic_key, 1400).await?;
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
    gemini_key: &str,
    anthropic_key: Option<&str>,
) -> Result<AnswerFeedbackResult> {
    let prompt = format!(
        "The interviewer asked: \"{}\"\n\nThe AI suggested the candidate say:\n{}\n\nThe candidate actually said: \"{}\"\n\nAnalyze the candidate's answer. Respond in EXACTLY this format:\n\nCOACHING: [1-2 sentence coaching note — be specific, reference what they said or missed]\nMISSED_FOLLOWUP: [yes/no — did they forget to ask a follow-up question to the interviewer?]\nMISSED_METRIC: [yes/no — did they fail to mention a specific number, metric, or measurable outcome?]",
        question, suggestion, candidate_answer
    );

    let text = call_ai(&prompt, gemini_key, anthropic_key, 200).await?;

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
    pub rapport_tip: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SalaryTactics {
    pub deflect: String,
    pub anchor: String,
    pub counter_range: String,
    pub never_say: String,
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

pub async fn generate_company_brief(
    company_info: &str,
    gemini_key: &str,
    anthropic_key: Option<&str>,
) -> CompanyBrief {
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
    let text = call_ai(&prompt, gemini_key, anthropic_key, 400).await.unwrap_or_default();
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

pub async fn generate_interviewer_summary(
    linkedin_text: &str,
    gemini_key: &str,
    anthropic_key: Option<&str>,
) -> Vec<InterviewerSummary> {
    if linkedin_text.trim().is_empty() { return vec![]; }
    // Split by separator if multiple interviewers
    let profiles: Vec<&str> = linkedin_text.split("---INTERVIEWER---")
        .map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    let mut results = Vec::new();
    for profile in profiles.iter().take(3) {
        let prompt = format!(
            "Based on this LinkedIn profile, create a brief for a job candidate to help build rapport.\n\nRespond in EXACTLY this format:\nNAME: [person's name or 'Unknown']\nROLE: [current job title]\nBACKGROUND: [1 sentence on their career background]\nTENURE: [how long at current company, or 'Unknown']\nRAPPORT: [1 specific rapport-building tip based on their background, e.g. 'They came from engineering — mention technical details']\n\n---\n{}",
            trunc(&profile, 2000)
        );
        let text = call_ai(&prompt, gemini_key, anthropic_key, 250).await.unwrap_or_default();
        let mut s = InterviewerSummary {
            name: "Unknown".to_string(), role: String::new(),
            background: String::new(), tenure: String::new(), rapport_tip: String::new(),
        };
        for line in text.lines() {
            if let Some(v) = line.strip_prefix("NAME:") { s.name = v.trim().to_string(); }
            else if let Some(v) = line.strip_prefix("ROLE:") { s.role = v.trim().to_string(); }
            else if let Some(v) = line.strip_prefix("BACKGROUND:") { s.background = v.trim().to_string(); }
            else if let Some(v) = line.strip_prefix("TENURE:") { s.tenure = v.trim().to_string(); }
            else if let Some(v) = line.strip_prefix("RAPPORT:") { s.rapport_tip = v.trim().to_string(); }
        }
        results.push(s);
    }
    results
}

pub async fn extract_jd_keywords(
    job_description: &str,
    gemini_key: &str,
    anthropic_key: Option<&str>,
) -> Vec<String> {
    if job_description.trim().is_empty() { return vec![]; }
    let prompt = format!(
        "Extract the 12 most important keywords and skill phrases from this job description that a candidate should mention during their interview.\n\nOutput ONLY one keyword/phrase per line, no numbers, no bullets, no extra text. Focus on: technical skills, soft skills, domain knowledge, and key responsibilities.\n\n---\n{}",
        trunc(&job_description, 3000)
    );
    match call_ai(&prompt, gemini_key, anthropic_key, 300).await {
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
    gemini_key: &str,
    anthropic_key: Option<&str>,
) -> Vec<String> {
    let prompt = format!(
        "Based on this interview conversation so far and the candidate's background, predict the 3 most likely questions the interviewer will ask next.\n\nOutput ONLY 3 questions, one per line, no numbers or bullets.\n\nCANDIDATE BACKGROUND (summary):\n{}\n\nCONVERSATION SO FAR:\n{}",
        trunc(&system_prompt, 1500),
        trunc(&transcript_context, 2000)
    );
    match call_ai(&prompt, gemini_key, anthropic_key, 300).await {
        Ok(text) => text.lines()
            .map(|l| l.trim().trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ')' || c == ' ').to_string())
            .filter(|l| l.len() > 10)
            .take(3)
            .collect(),
        Err(_) => vec![],
    }
}

pub async fn generate_salary_tactics(
    role_context: &str,
    gemini_key: &str,
    anthropic_key: Option<&str>,
) -> SalaryTactics {
    let prompt = format!(
        "Generate salary negotiation tactics for a candidate interviewing for this role.\n\nRespond in EXACTLY this format:\nDEFLECT: [exact words to use to deflect salary question early — e.g. 'I'm flexible and focused on fit — can you share the budgeted range?']\nANCHOR: [exact words to anchor high when asked for a number — 1 sentence]\nCOUNTER: [suggested counter-offer approach — 1 sentence with placeholder like '15% above initial offer']\nNEVER: [the one thing never to say — e.g. 'Never give a number first if you can avoid it']\n\nROLE CONTEXT:\n{}",
        trunc(&role_context, 800)
    );
    let text = call_ai(&prompt, gemini_key, anthropic_key, 300).await.unwrap_or_default();
    let mut t = SalaryTactics {
        deflect: String::new(), anchor: String::new(),
        counter_range: String::new(), never_say: String::new(),
    };
    for line in text.lines() {
        if let Some(v) = line.strip_prefix("DEFLECT:") { t.deflect = v.trim().to_string(); }
        else if let Some(v) = line.strip_prefix("ANCHOR:") { t.anchor = v.trim().to_string(); }
        else if let Some(v) = line.strip_prefix("COUNTER:") { t.counter_range = v.trim().to_string(); }
        else if let Some(v) = line.strip_prefix("NEVER:") { t.never_say = v.trim().to_string(); }
    }
    if t.deflect.is_empty() { t.deflect = "I'm flexible on compensation — can you share the range you have budgeted?".to_string(); }
    if t.never_say.is_empty() { t.never_say = "Never give a specific number first if you can avoid it.".to_string(); }
    t
}

pub async fn score_practice_answer(
    question: &str,
    answer: &str,
    system_prompt: &str,
    gemini_key: &str,
    anthropic_key: Option<&str>,
) -> PracticeScore {
    let prompt = format!(
        "Score this practice interview answer.\n\nQuestion: \"{}\"\nAnswer: \"{}\"\n\nCandidate background:\n{}\n\nRespond in EXACTLY this format:\nSCORE: [0-100 integer]\nSTAR: [yes/no — does the answer follow Situation/Task/Action/Result structure?]\nMETRIC: [yes/no — does it include a specific number, %, or measurable outcome?]\nLENGTH: [yes/no — is the answer an appropriate length, under 90 seconds to speak?]\nSTRONG: [1 sentence on what was done well]\nCOACH: [1-2 sentences of specific coaching to improve this answer]",
        question, answer, trunc(&system_prompt, 1000)
    );
    let text = call_ai(&prompt, gemini_key, anthropic_key, 300).await.unwrap_or_default();
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

pub async fn extract_next_steps(
    transcript_text: &str,
    gemini_key: &str,
    anthropic_key: Option<&str>,
) -> Vec<String> {
    if transcript_text.trim().is_empty() { return vec![]; }
    let prompt = format!(
        "From this interview transcript, extract all mentioned next steps, timelines, and follow-up actions. Include things like: when they'll get back to you, who to contact, what the hiring process looks like, any requested follow-ups.\n\nOutput one item per line, no bullets or numbers. If none found, output 'No specific next steps mentioned.'\n\n---\n{}",
        trunc(&transcript_text, 4000)
    );
    match call_ai(&prompt, gemini_key, anthropic_key, 300).await {
        Ok(text) => text.lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .take(6)
            .collect(),
        Err(_) => vec![],
    }
}
