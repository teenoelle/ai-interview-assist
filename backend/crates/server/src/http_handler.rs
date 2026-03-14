use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    Json,
};
use common::messages::SetupPayload;
use context::ai_helper::{generate_debrief, predict_questions, call_ai_simple, generate_company_brief, generate_interviewer_summary, extract_jd_keywords};
use context::builder::build_system_prompt;
use context::crawler::crawl_website;
use context::linkedin::parse_all_linkedin_profiles;
use context::pdf::{describe_image_with_gemini, extract_docx_text, extract_pdf_text, extract_pptx_text, extract_xlsx_text};
use crate::state::AppState;

#[derive(serde::Serialize)]
pub struct SetupResponse {
    pub success: bool,
    pub system_prompt_preview: String,
    pub message: String,
    pub predicted_questions: Vec<String>,
    pub company_brief: Option<context::ai_helper::CompanyBrief>,
    pub interviewer_summaries: Vec<context::ai_helper::InterviewerSummary>,
    pub jd_keywords: Vec<String>,
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
            "linkedin_text" => {
                payload.linkedin_text = field.text().await.unwrap_or_default();
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

    // Extract CV text
    if let Some(bytes) = cv_bytes {
        let name_lower = cv_filename.to_lowercase();
        payload.cv_text = extract_file_text(&bytes, &name_lower, &state.gemini_key).await;
    }

    // Extract extra experience file and append to text
    if let Some(bytes) = extra_file_bytes {
        let name_lower = extra_filename.to_lowercase();
        let file_text = extract_file_text(&bytes, &name_lower, &state.gemini_key).await;
        if !file_text.is_empty() {
            if !payload.extra_experience.is_empty() {
                payload.extra_experience.push_str("\n\n");
            }
            payload.extra_experience.push_str(&file_text);
        }
    }

    // Crawl company website
    let company_info = if !payload.company_url.is_empty() {
        crawl_website(&payload.company_url, 50)
            .await
            .unwrap_or_default()
    } else {
        String::new()
    };

    // Parse LinkedIn (supports multiple profiles separated by ---INTERVIEWER---)
    let interviewer_profiles = parse_all_linkedin_profiles(&payload.linkedin_text);

    // Build system prompt
    let system_prompt = build_system_prompt(&payload, &company_info, &interviewer_profiles);

    // Store
    {
        let mut sp = state.system_prompt.write().await;
        *sp = system_prompt.clone();
    }

    let preview = if system_prompt.len() > 500 {
        format!("{}...", &system_prompt[..500])
    } else {
        system_prompt.clone()
    };

    let _ = state.event_tx.send(common::messages::WsEvent::Status {
        message: "Setup complete. Ready for interview.".to_string(),
    });

    let (predicted_questions, company_brief, interviewer_summaries, jd_keywords) = tokio::join!(
        predict_questions(&system_prompt, &state.gemini_key, state.anthropic_key.as_deref()),
        generate_company_brief(&company_info, &state.gemini_key, state.anthropic_key.as_deref()),
        generate_interviewer_summary(&payload.linkedin_text, &state.gemini_key, state.anthropic_key.as_deref()),
        extract_jd_keywords(&payload.job_description, &state.gemini_key, state.anthropic_key.as_deref()),
    );

    let company_brief_opt = if company_brief.name.is_empty() { None } else { Some(company_brief) };

    Ok(Json(SetupResponse {
        success: true,
        system_prompt_preview: preview,
        message: "Setup complete".to_string(),
        predicted_questions,
        company_brief: company_brief_opt,
        interviewer_summaries,
        jd_keywords,
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

    generate_debrief(
        &transcript_text,
        &suggestions_text,
        &state.gemini_key,
        state.anthropic_key.as_deref(),
    )
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

    let suggestion = call_ai_simple(
        &sp,
        &user_prompt,
        &state.gemini_key,
        state.anthropic_key.as_deref(),
    )
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
    context::ai_helper::generate_answer_feedback(
        &req.question,
        &req.answer,
        &req.suggestion,
        &state.gemini_key,
        state.anthropic_key.as_deref(),
    )
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
    let questions = context::ai_helper::predict_next_questions(
        &transcript_text,
        &sp,
        &state.gemini_key,
        state.anthropic_key.as_deref(),
    ).await;
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
    let tactics = context::ai_helper::generate_salary_tactics(
        &req.role_context,
        &state.gemini_key,
        state.anthropic_key.as_deref(),
    ).await;
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
    let score = context::ai_helper::score_practice_answer(
        &req.question,
        &req.answer,
        &sp,
        &state.gemini_key,
        state.anthropic_key.as_deref(),
    ).await;
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
    let steps = context::ai_helper::extract_next_steps(
        &transcript_text,
        &state.gemini_key,
        state.anthropic_key.as_deref(),
    ).await;
    Ok(Json(NextStepsResponse { steps }))
}
