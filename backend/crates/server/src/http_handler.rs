use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    Json,
};
use sentiment::gemini_vision::analyze_presence;
use common::messages::SetupPayload;
use context::ai_helper::{generate_debrief, predict_questions, call_ai_simple, call_ai_fast, generate_company_brief, generate_interviewer_summary, extract_jd_keywords, assess_vocal_delivery, AiConfig};
use context::builder::build_system_prompt;
use context::crawler::{crawl_website, extract_github_username, fetch_github_portfolio};
use context::linkedin::parse_all_linkedin_profiles;
use context::pdf::{describe_image_with_gemini, extract_docx_text, extract_pdf_text, extract_pptx_text, extract_xlsx_text};
use crate::state::AppState;

#[derive(serde::Serialize)]
pub struct SetupResponse {
    pub success: bool,
    pub message: String,
}

pub async fn handle_setup_finalize(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<SetupResponse>, (StatusCode, String)> {
    let mut payload = SetupPayload::default();
    let mut cv_bytes: Option<Vec<u8>> = None;
    let mut cv_filename = String::new();
    let mut extra_file_bytes: Option<Vec<u8>> = None;
    let mut extra_filename = String::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "job_description" => {
                payload.job_description = field.text().await.unwrap_or_default();
            }
            "company_url" => {
                payload.company_url = field.text().await.unwrap_or_default();
            }
            "portfolio_url" => {
                payload.portfolio_url = field.text().await.unwrap_or_default();
            }
            "linkedin_text" => {
                payload.linkedin_text = field.text().await.unwrap_or_default();
            }
            "interviewee_linkedin" => {
                payload.interviewee_linkedin = field.text().await.unwrap_or_default();
            }
            "extra_experience" => {
                payload.extra_experience = field.text().await.unwrap_or_default();
            }
            "cv_file" => {
                cv_filename = field.file_name().unwrap_or("").to_string();
                cv_bytes = Some(field.bytes().await.unwrap_or_default().to_vec());
            }
            "extra_file" => {
                extra_filename = field.file_name().unwrap_or("").to_string();
                extra_file_bytes = Some(field.bytes().await.unwrap_or_default().to_vec());
            }
            _ => {}
        }
    }

    // Extract CV and extra file in parallel (images hit Gemini API, which is slow)
    let gemini_key = state.gemini_key.clone();
    let (cv_text, extra_text) = tokio::join!(
        async {
            if let Some(bytes) = cv_bytes {
                extract_file_text(&bytes, &cv_filename.to_lowercase(), &gemini_key).await
            } else { String::new() }
        },
        async {
            if let Some(bytes) = extra_file_bytes {
                extract_file_text(&bytes, &extra_filename.to_lowercase(), &gemini_key).await
            } else { String::new() }
        },
    );
    payload.cv_text = cv_text;
    if !extra_text.is_empty() {
        if !payload.extra_experience.is_empty() { payload.extra_experience.push_str("\n\n"); }
        payload.extra_experience.push_str(&extra_text);
    }

    // Parse LinkedIn
    let interviewer_profiles = parse_all_linkedin_profiles(&payload.linkedin_text);

    // Build system prompt immediately from JD + CV + LinkedIn (no crawl — fast path)
    // Company info and portfolio will be appended by /api/enrich in the background
    let system_prompt = build_system_prompt(&payload, "", &interviewer_profiles);

    // Store everything needed for background enrichment endpoints
    {
        let mut sp = state.system_prompt.write().await; *sp = system_prompt.clone();
    }
    {
        let mut lt = state.linkedin_text.write().await; *lt = payload.linkedin_text.clone();
    }
    {
        let mut cu = state.company_url.write().await; *cu = payload.company_url.clone();
    }
    {
        let mut pu = state.portfolio_url.write().await; *pu = payload.portfolio_url.clone();
    }
    {
        let mut jd = state.jd_text.write().await; *jd = payload.job_description.clone();
    }

    // Build prediction context (used by /api/predict-questions)
    let mut prediction_ctx = String::new();
    if !payload.job_description.is_empty() {
        prediction_ctx.push_str("JOB DESCRIPTION:\n");
        prediction_ctx.push_str(&payload.job_description);
        prediction_ctx.push_str("\n\n");
    }
    if !payload.cv_text.is_empty() {
        prediction_ctx.push_str("CANDIDATE CV:\n");
        prediction_ctx.push_str(&payload.cv_text);
        prediction_ctx.push_str("\n\n");
    }
    if !payload.interviewee_linkedin.is_empty() {
        prediction_ctx.push_str("CANDIDATE LINKEDIN:\n");
        prediction_ctx.push_str(&payload.interviewee_linkedin);
        prediction_ctx.push_str("\n\n");
    }
    if !payload.extra_experience.is_empty() {
        prediction_ctx.push_str("ADDITIONAL EXPERIENCE:\n");
        prediction_ctx.push_str(&payload.extra_experience);
        prediction_ctx.push_str("\n\n");
    }
    {
        let mut pc = state.prediction_context.write().await; *pc = prediction_ctx;
    }

    let _ = state.event_tx.send(common::messages::WsEvent::Status {
        message: "Setup complete. Ready for interview.".to_string(),
    });

    Ok(Json(SetupResponse {
        success: true,
        message: "Setup complete".to_string(),
    }))
}

/// Extract text from an uploaded file based on its extension.
async fn extract_file_text(bytes: &[u8], name_lower: &str, gemini_key: &str) -> String {
    if name_lower.ends_with(".pdf") {
        extract_pdf_text(bytes).unwrap_or_default()
    } else if name_lower.ends_with(".docx") {
        extract_docx_text(bytes).unwrap_or_default()
    } else if name_lower.ends_with(".pptx") {
        extract_pptx_text(bytes).unwrap_or_default()
    } else if name_lower.ends_with(".xlsx") {
        extract_xlsx_text(bytes).unwrap_or_default()
    } else if name_lower.ends_with(".jpg")
        || name_lower.ends_with(".jpeg")
        || name_lower.ends_with(".png")
        || name_lower.ends_with(".gif")
        || name_lower.ends_with(".webp")
    {
        let mime = if name_lower.ends_with(".png") {
            "image/png"
        } else if name_lower.ends_with(".gif") {
            "image/gif"
        } else if name_lower.ends_with(".webp") {
            "image/webp"
        } else {
            "image/jpeg"
        };
        match describe_image_with_gemini(bytes, mime, gemini_key).await {
            Ok(desc) => desc,
            Err(e) => {
                tracing::warn!("Image description failed: {}", e);
                String::new()
            }
        }
    } else {
        // .txt, .md, .csv, .rtf, etc — treat as plain UTF-8
        String::from_utf8_lossy(bytes).to_string()
    }
}

/// POST /api/extract-file — extract text from a single uploaded file (for preview/preset saving)
#[derive(serde::Serialize)]
pub struct ExtractFileResponse {
    pub text: String,
    pub filename: String,
}

pub async fn handle_extract_file(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<ExtractFileResponse>, (StatusCode, String)> {
    let mut filename = String::new();
    let mut bytes: Vec<u8> = Vec::new();
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("file") {
            filename = field.file_name().unwrap_or("").to_string();
            bytes = field.bytes().await.unwrap_or_default().to_vec();
        }
    }
    if bytes.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "No file provided".to_string()));
    }
    let text = extract_file_text(&bytes, &filename.to_lowercase(), &state.gemini_key).await;
    Ok(Json(ExtractFileResponse { text, filename }))
}

#[derive(serde::Deserialize)]
pub struct DebriefRequest {
    pub transcript: Vec<TranscriptEntry>,
    pub suggestions: Vec<SuggestionItem>,
}

#[derive(serde::Deserialize)]
pub struct TranscriptEntry {
    pub speaker: String,
    pub text: String,
}

#[derive(serde::Deserialize)]
pub struct SuggestionItem {
    pub question: String,
    pub suggestion: String,
}

pub async fn handle_debrief(
    State(state): State<AppState>,
    Json(req): Json<DebriefRequest>,
) -> Result<Json<context::ai_helper::DebriefResult>, (StatusCode, String)> {
    let transcript_text = req.transcript
        .iter()
        .map(|e| format!("{}: {}", e.speaker, e.text))
        .collect::<Vec<_>>()
        .join("\n");

    let suggestions_text = req.suggestions
        .iter()
        .map(|s| format!("Q: {}\nA: {}", s.question, s.suggestion))
        .collect::<Vec<_>>()
        .join("\n\n");

    let cfg = AiConfig {
        gemini_key: &state.gemini_key,
        anthropic_key: state.anthropic_key.as_deref(),
        groq_key: state.groq_key.as_deref(),
        groq_key_2: state.groq_key_2.as_deref(),
        ollama_url: &state.ollama_url,
        ollama_model: &state.ollama_model,
        usage: Some(state.call_counts.clone()),
    };
    generate_debrief(&transcript_text, &suggestions_text, &cfg)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

#[derive(serde::Deserialize)]
pub struct PracticeQuestionRequest {
    pub question: String,
}

#[derive(serde::Serialize)]
pub struct PracticeQuestionResponse {
    pub suggestion: String,
}

pub async fn handle_practice_question(
    State(state): State<AppState>,
    Json(req): Json<PracticeQuestionRequest>,
) -> Result<Json<PracticeQuestionResponse>, (StatusCode, String)> {
    let sp = state.system_prompt.read().await.clone();
    let user_prompt = suggestion::prompt::build_user_prompt(&req.question, &[]);
    let cfg = AiConfig {
        gemini_key: &state.gemini_key,
        anthropic_key: state.anthropic_key.as_deref(),
        groq_key: state.groq_key.as_deref(),
        groq_key_2: state.groq_key_2.as_deref(),
        ollama_url: &state.ollama_url,
        ollama_model: &state.ollama_model,
        usage: Some(state.call_counts.clone()),
    };
    let suggestion = call_ai_simple(&cfg, &sp, &user_prompt)
        .await
        .unwrap_or_else(|_| "Could not generate hints at this time.".to_string());

    Ok(Json(PracticeQuestionResponse { suggestion }))
}

#[derive(serde::Deserialize)]
pub struct AnswerFeedbackRequest {
    pub question: String,
    pub answer: String,
    pub suggestion: String,
}

pub async fn handle_answer_feedback(
    State(state): State<AppState>,
    Json(req): Json<AnswerFeedbackRequest>,
) -> Result<Json<context::ai_helper::AnswerFeedbackResult>, (StatusCode, String)> {
    let cfg = AiConfig {
        gemini_key: &state.gemini_key,
        anthropic_key: state.anthropic_key.as_deref(),
        groq_key: state.groq_key.as_deref(),
        groq_key_2: state.groq_key_2.as_deref(),
        ollama_url: &state.ollama_url,
        ollama_model: &state.ollama_model,
        usage: Some(state.call_counts.clone()),
    };
    context::ai_helper::generate_answer_feedback(&req.question, &req.answer, &req.suggestion, &cfg)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

#[derive(serde::Deserialize)]
pub struct NextQuestionRequest {
    pub transcript: Vec<TranscriptEntry>,
}

#[derive(serde::Serialize)]
pub struct NextQuestionResponse {
    pub questions: Vec<String>,
}

pub async fn handle_next_question(
    State(state): State<AppState>,
    Json(req): Json<NextQuestionRequest>,
) -> Result<Json<NextQuestionResponse>, (StatusCode, String)> {
    let transcript_text = req.transcript.iter()
        .map(|e| format!("{}: {}", e.speaker, e.text))
        .collect::<Vec<_>>()
        .join("\n");
    let sp = state.system_prompt.read().await.clone();
    let cfg = AiConfig {
        gemini_key: &state.gemini_key,
        anthropic_key: state.anthropic_key.as_deref(),
        groq_key: state.groq_key.as_deref(),
        groq_key_2: state.groq_key_2.as_deref(),
        ollama_url: &state.ollama_url,
        ollama_model: &state.ollama_model,
        usage: Some(state.call_counts.clone()),
    };
    let questions = context::ai_helper::predict_next_questions(&transcript_text, &sp, &cfg).await;
    Ok(Json(NextQuestionResponse { questions }))
}

#[derive(serde::Deserialize)]
pub struct SalaryCoachRequest {
    pub role_context: String,
}

pub async fn handle_salary_coach(
    State(state): State<AppState>,
    Json(req): Json<SalaryCoachRequest>,
) -> Result<Json<context::ai_helper::SalaryTactics>, (StatusCode, String)> {
    let cfg = AiConfig {
        gemini_key: &state.gemini_key,
        anthropic_key: state.anthropic_key.as_deref(),
        groq_key: state.groq_key.as_deref(),
        groq_key_2: state.groq_key_2.as_deref(),
        ollama_url: &state.ollama_url,
        ollama_model: &state.ollama_model,
        usage: Some(state.call_counts.clone()),
    };
    let tactics = context::ai_helper::generate_salary_tactics(&req.role_context, &cfg).await;
    Ok(Json(tactics))
}

#[derive(serde::Deserialize)]
pub struct ScorePracticeRequest {
    pub question: String,
    pub answer: String,
}

pub async fn handle_score_practice(
    State(state): State<AppState>,
    Json(req): Json<ScorePracticeRequest>,
) -> Result<Json<context::ai_helper::PracticeScore>, (StatusCode, String)> {
    let sp = state.system_prompt.read().await.clone();
    let cfg = AiConfig {
        gemini_key: &state.gemini_key,
        anthropic_key: state.anthropic_key.as_deref(),
        groq_key: state.groq_key.as_deref(),
        groq_key_2: state.groq_key_2.as_deref(),
        ollama_url: &state.ollama_url,
        ollama_model: &state.ollama_model,
        usage: Some(state.call_counts.clone()),
    };
    let score = context::ai_helper::score_practice_answer(&req.question, &req.answer, &sp, &cfg).await;
    Ok(Json(score))
}

#[derive(serde::Deserialize)]
pub struct NextStepsRequest {
    pub transcript: Vec<TranscriptEntry>,
}

#[derive(serde::Serialize)]
pub struct NextStepsResponse {
    pub steps: Vec<String>,
}

pub async fn handle_next_steps(
    State(state): State<AppState>,
    Json(req): Json<NextStepsRequest>,
) -> Result<Json<NextStepsResponse>, (StatusCode, String)> {
    let transcript_text = req.transcript.iter()
        .map(|e| format!("{}: {}", e.speaker, e.text))
        .collect::<Vec<_>>()
        .join("\n");
    let cfg = AiConfig {
        gemini_key: &state.gemini_key,
        anthropic_key: state.anthropic_key.as_deref(),
        groq_key: state.groq_key.as_deref(),
        groq_key_2: state.groq_key_2.as_deref(),
        ollama_url: &state.ollama_url,
        ollama_model: &state.ollama_model,
        usage: Some(state.call_counts.clone()),
    };
    let steps = context::ai_helper::extract_next_steps(&transcript_text, &cfg).await;
    Ok(Json(NextStepsResponse { steps }))
}

#[derive(serde::Deserialize)]
pub struct VocalSentimentRequest {
    pub question: String,
    pub transcript: String,
    pub duration_seconds: f32,
    pub word_count: u32,
    pub filler_count: u32,
    pub filler_detail: String,
}

pub async fn handle_vocal_sentiment(
    State(state): State<AppState>,
    Json(req): Json<VocalSentimentRequest>,
) -> Result<Json<context::ai_helper::VocalSentiment>, (StatusCode, String)> {
    let cfg = AiConfig {
        gemini_key: &state.gemini_key,
        anthropic_key: state.anthropic_key.as_deref(),
        groq_key: state.groq_key.as_deref(),
        groq_key_2: state.groq_key_2.as_deref(),
        ollama_url: &state.ollama_url,
        ollama_model: &state.ollama_model,
        usage: Some(state.call_counts.clone()),
    };
    let result = assess_vocal_delivery(
        &req.question, &req.transcript, req.duration_seconds,
        req.word_count, req.filler_count, &req.filler_detail, &cfg,
    ).await;
    Ok(Json(result))
}

#[derive(serde::Deserialize)]
pub struct KeywordDefinitionRequest {
    pub keyword: String,
}

#[derive(serde::Serialize)]
pub struct KeywordDefinitionResponse {
    pub definition: String,
    pub tip: String,
}

pub async fn handle_keyword_definition(
    State(state): State<AppState>,
    Json(req): Json<KeywordDefinitionRequest>,
) -> Result<Json<KeywordDefinitionResponse>, (StatusCode, String)> {
    let sp = state.system_prompt.read().await.clone();
    let user_prompt = format!(
        "The candidate is interviewing for a role where \"{}\" is a key requirement.\n\nRespond with EXACTLY two plain-text sentences on two separate lines:\n1. Begin with \"{} refers to\" then explain what it means for this specific role (max 20 words total, no jargon, grammatically correct)\n2. One concrete tip for naturally weaving it into an interview answer (max 20 words, start with an action verb)\n\nNo numbers, no labels, no markdown, no blank lines between them.",
        req.keyword, req.keyword
    );
    let cfg = AiConfig {
        gemini_key: &state.gemini_key,
        anthropic_key: state.anthropic_key.as_deref(),
        groq_key: state.groq_key.as_deref(),
        groq_key_2: state.groq_key_2.as_deref(),
        ollama_url: &state.ollama_url,
        ollama_model: &state.ollama_model,
        usage: Some(state.call_counts.clone()),
    };
    let raw = call_ai_simple(&cfg, &sp, &user_prompt)
        .await
        .unwrap_or_else(|_| format!("A key skill or concept relevant to this role.\nMention a specific example where you applied {} in a past project.", req.keyword));
    let lines: Vec<&str> = raw.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    let definition = lines.first().map(|s| s.to_string()).unwrap_or_else(|| format!("Key skill for this role: {}.", req.keyword));
    let tip = lines.get(1).map(|s| s.to_string()).unwrap_or_default();
    Ok(Json(KeywordDefinitionResponse { definition, tip }))
}

#[derive(serde::Deserialize)]
pub struct ExpandCueRequest {
    pub question: String,
    pub cue: String,
}

#[derive(serde::Serialize)]
pub struct ExpandCueResponse {
    pub sentence: String,
}

pub async fn handle_expand_cue(
    State(state): State<AppState>,
    Json(req): Json<ExpandCueRequest>,
) -> Result<Json<ExpandCueResponse>, (StatusCode, String)> {
    let sp = state.system_prompt.read().await.clone();
    let context_line = if sp.is_empty() {
        String::new()
    } else {
        format!("Candidate background:\n{}\n\n", &sp[..sp.len().min(800)])
    };
    let user_prompt = format!(
        "{}Interview question: \"{}\"\nTalking point cue: \"{}\"\n\nIMPORTANT: Output ONLY the spoken text. No intro, no preamble, no framing like 'Here are...' or 'Sure!' or 'Certainly!'.\n\nExpand this cue into short sentences. Output each sentence on its own line.\nRules:\n- For [Example] and [General Answer] cues: every sentence starts with 'I'. Max 10 words per sentence — use MORE sentences instead of making one longer.\n- For [Ask] cues: output a question to the interviewer. Starts with 'How', 'What', 'Which', 'When', or 'Can you'. Max 15 words. Ends with '?'.\n- Use as many sentences as needed — never sacrifice specificity or completeness for brevity\n- Grammatically correct and natural to say out loud\n- No adjectives or adverbs — facts and actions only\n- No 'utilize' — use 'use' instead\n- Always use 'I' — never 'we', 'our team', or 'we found'. The candidate speaks only about their own actions and decisions.\n- Acronyms: write in full on first use followed by the abbreviation in parentheses — e.g. 'Customer Acquisition Cost (CAC)', 'Return on Ad Spend (ROAS)'. Never use a bare acronym without first defining it.\n- NEVER invent metrics, percentages, numbers, company names, or roles. Use only facts explicitly stated in the candidate background. If an outcome improved but the magnitude is unknown, say 'measurably improved' or 'reduced noticeably' — never fabricate a number.\n- NEVER use vague language: avoid 'this', 'it', 'that', 'here', 'the issue', 'the problem', 'the strategy', or similar placeholders. Always name the specific metric, channel, tool, process, or concept explicitly (e.g. 'paid social CPA' not 'this metric', 'customer acquisition cost' not 'the problem').\n\n- If the cue is an [Example], follow this exact structure (each step is one or two short sentences):\n  Step 1 — INTRODUCE: Name the project or situation so the interviewer knows which example you mean.\n  Step 2 — PROBLEM + BUSINESS IMPACT: State what the problem was and how it was hurting the specific business outcome at stake (e.g. raising customer acquisition cost, reducing retention, blocking revenue growth). Name the outcome explicitly.\n  Step 3 — STRATEGY + ACTIONS: State what you specifically did — name the tools, channels, or processes explicitly.\n  Step 4 — RESULT + BOTTOM LINE: State what improved and how it resolved the specific business problem. Name the exact outcome — not 'improved performance' but 'reduced customer acquisition cost' or 'improved paid social ROAS'. Use only metrics from the candidate background — if none are stated, use directional language like 'measurably reduced customer acquisition cost'.\n\n- If the cue is a [General Answer], structure as: (1) Lead sentence names the specific business outcome this approach achieves; (2) Explain the specific method — name tools, channels, or processes; (3) One inline illustration: 'So if [specific trigger relevant to this company or their clients], I [specific action], which would [directional outcome].' — e.g. 'So if a client's CPA rises, I audit targeting changes and form messaging, which would bring conversion costs down.' Directional language only — never a percentage or fabricated number; (4) Closing sentence names the exact business impact. Never end on the method — always close on the impact.\n- If the cue is an [Ask] topic, generate a natural follow-up question the candidate asks the interviewer. Start with 'How', 'What', 'Which', 'When', or 'Can you'. Name the specific metric, tool, or process — no vague pronouns. 1-2 sentences max 15 words each. End with '?'. Output only the question, no preamble.
- If the cue is [Pivot], generate a short recovery phrase the candidate uses when interrupted or asked an unexpected follow-up. 1-2 sentences max 10 words each. Bridges back to the topic naturally. Starts with 'That's a great point —' or 'Building on that —' or 'To add to that —'. Never a question. Output only the phrase, no preamble.\n- Use ONLY facts from the candidate background — never invent details\n- Do not include any help text, labels, or instructions in the output",
        context_line, req.question, req.cue
    );
    let cfg = AiConfig {
        gemini_key: &state.gemini_key,
        anthropic_key: state.anthropic_key.as_deref(),
        groq_key: state.groq_key.as_deref(),
        groq_key_2: state.groq_key_2.as_deref(),
        ollama_url: &state.ollama_url,
        ollama_model: &state.ollama_model,
        usage: Some(state.call_counts.clone()),
    };
    let sentence = call_ai_fast(&cfg, &sp, &user_prompt)
        .await
        .unwrap_or_else(|_| req.cue.clone());
    let sentence = if sentence.is_empty() { req.cue.clone() } else { sentence };
    Ok(Json(ExpandCueResponse { sentence }))
}

#[derive(serde::Serialize)]
pub struct PresenceCheckResponse {
    pub issues: Vec<String>,
    pub positive: Option<String>,
}

pub async fn handle_presence_check(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<PresenceCheckResponse>, (StatusCode, String)> {
    let mut jpeg_bytes = Vec::new();
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("image") {
            jpeg_bytes = field.bytes().await.unwrap_or_default().to_vec();
        }
    }
    if jpeg_bytes.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "No image".to_string()));
    }
    let result = analyze_presence(&state.gemini_key, &jpeg_bytes)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(PresenceCheckResponse { issues: result.issues, positive: result.positive }))
}

pub async fn handle_usage(
    State(state): State<AppState>,
) -> Json<std::collections::HashMap<String, u64>> {
    let map = state.call_counts.lock()
        .map(|m| m.clone())
        .unwrap_or_default();
    Json(map)
}

#[derive(serde::Deserialize)]
pub struct SimulateQuestionRequest {
    pub question: String,
}

pub async fn handle_simulate_question(
    State(state): State<AppState>,
    Json(req): Json<SimulateQuestionRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    if req.question.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Question is empty".to_string()));
    }
    state.question_tx.send(req.question.trim().to_string()).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(StatusCode::OK)
}

#[derive(serde::Deserialize)]
pub struct SuggestModeRequest {
    pub question: String,
    pub mode: String, // "primary" | "secondary"
}

pub async fn handle_suggest_mode(
    State(state): State<AppState>,
    Json(req): Json<SuggestModeRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let q = req.question.trim().to_string();
    if q.is_empty() { return Err((StatusCode::BAD_REQUEST, "Question is empty".to_string())); }
    let mode = match req.mode.as_str() {
        "secondary" => common::messages::SuggestionMode::Secondary,
        _ => common::messages::SuggestionMode::Primary,
    };
    let sp = state.system_prompt.read().await.clone();
    let tr = state.transcript.read().await.clone();
    let etx = state.event_tx.clone();
    let gkey = state.gemini_key.clone();
    let akey = state.anthropic_key.clone();
    let grkey = state.groq_key.clone();
    let grkey2 = state.groq_key_2.clone();
    let orkey = state.openrouter_key.clone();
    let mkey = state.mistral_key.clone();
    let ckey = state.cerebras_key.clone();
    let qkey = state.qwen_key.clone();
    let ourl = state.ollama_url.clone();
    let omodels = state.ollama_models.clone();
    let rl = state.rate_limiter.clone();
    let cc = Some(state.call_counts.clone());
    tokio::spawn(async move {
        if let Err(e) = suggestion::run_single(
            &q, mode, &sp, &tr,
            &gkey, akey.as_deref(), grkey.as_deref(), grkey2.as_deref(),
            orkey.as_deref(), mkey.as_deref(), ckey.as_deref(), qkey.as_deref(),
            &ourl, &omodels, &rl, etx, &cc,
        ).await {
            tracing::error!("suggest-mode error: {}", e);
        }
    });
    Ok(StatusCode::OK)
}

#[derive(serde::Serialize)]
pub struct PredictQuestionsResponse {
    pub questions: Vec<String>,
}

pub async fn handle_predict_questions(
    State(state): State<AppState>,
) -> Json<PredictQuestionsResponse> {
    let ctx = state.prediction_context.read().await.clone();
    let cfg = AiConfig {
        gemini_key: &state.gemini_key,
        anthropic_key: state.anthropic_key.as_deref(),
        groq_key: state.groq_key.as_deref(),
        groq_key_2: state.groq_key_2.as_deref(),
        ollama_url: &state.ollama_url,
        ollama_model: &state.ollama_model,
        usage: Some(state.call_counts.clone()),
    };
    let questions = predict_questions(&ctx, &cfg).await;
    Json(PredictQuestionsResponse { questions })
}

#[derive(serde::Serialize)]
pub struct EnrichResponse {
    pub company_brief: Option<context::ai_helper::CompanyBrief>,
    pub jd_keywords: Vec<String>,
}

pub async fn handle_enrich(
    State(state): State<AppState>,
) -> Json<EnrichResponse> {
    let company_url = state.company_url.read().await.clone();
    let portfolio_url = state.portfolio_url.read().await.clone();
    let jd_text = state.jd_text.read().await.clone();

    const CRAWL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

    // Crawl company + portfolio in parallel
    let (company_info, portfolio_text) = tokio::join!(
        async {
            if company_url.is_empty() { return String::new(); }
            tokio::time::timeout(CRAWL_TIMEOUT, crawl_website(&company_url, 10))
                .await.ok().and_then(|r| r.ok()).unwrap_or_default()
        },
        async {
            let urls: Vec<&str> = portfolio_url.lines()
                .map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
            if urls.is_empty() { return String::new(); }
            let mut parts = Vec::new();
            for url in urls {
                let text = if let Some(username) = extract_github_username(url) {
                    fetch_github_portfolio(&username).await
                } else {
                    tokio::time::timeout(CRAWL_TIMEOUT, crawl_website(url, 10))
                        .await.ok().and_then(|r| r.ok()).unwrap_or_default()
                };
                if !text.is_empty() { parts.push(text); }
            }
            parts.join("\n\n---\n\n")
        },
    );

    // Append crawled context to existing system prompt
    if !company_info.is_empty() || !portfolio_text.is_empty() {
        let mut sp = state.system_prompt.write().await;
        if !company_info.is_empty() {
            sp.push_str("\n\n## Company Website\n");
            let trunc_len = company_info.char_indices().map(|(i,_)| i).take_while(|&i| i <= 4000).last().unwrap_or(0);
            sp.push_str(&company_info[..trunc_len]);
        }
        if !portfolio_text.is_empty() {
            sp.push_str("\n\n## Portfolio / Website\n");
            let trunc_len = portfolio_text.char_indices().map(|(i,_)| i).take_while(|&i| i <= 2000).last().unwrap_or(0);
            sp.push_str(&portfolio_text[..trunc_len]);
        }
    }

    let cfg = AiConfig {
        gemini_key: &state.gemini_key,
        anthropic_key: state.anthropic_key.as_deref(),
        groq_key: state.groq_key.as_deref(),
        groq_key_2: state.groq_key_2.as_deref(),
        ollama_url: &state.ollama_url,
        ollama_model: &state.ollama_model,
        usage: Some(state.call_counts.clone()),
    };

    // Run company brief + keyword extraction in parallel
    let (brief, jd_keywords) = tokio::join!(
        generate_company_brief(&company_info, &cfg),
        extract_jd_keywords(&jd_text, &cfg),
    );

    // Store keywords for review pipeline
    {
        let mut kw = state.jd_keywords.write().await;
        *kw = jd_keywords.clone();
    }

    Json(EnrichResponse {
        company_brief: if brief.name.is_empty() { None } else { Some(brief) },
        jd_keywords,
    })
}

pub async fn handle_interviewer_summaries(
    State(state): State<AppState>,
) -> Json<Vec<context::ai_helper::InterviewerSummary>> {
    let linkedin_text = state.linkedin_text.read().await.clone();
    let cfg = AiConfig {
        gemini_key: &state.gemini_key,
        anthropic_key: state.anthropic_key.as_deref(),
        groq_key: state.groq_key.as_deref(),
        groq_key_2: state.groq_key_2.as_deref(),
        ollama_url: &state.ollama_url,
        ollama_model: &state.ollama_model,
        usage: Some(state.call_counts.clone()),
    };
    Json(generate_interviewer_summary(&linkedin_text, &cfg).await)
}
