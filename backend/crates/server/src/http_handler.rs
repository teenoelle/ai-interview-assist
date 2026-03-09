use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    Json,
};
use common::messages::SetupPayload;
use context::builder::build_system_prompt;
use context::crawler::crawl_website;
use context::linkedin::parse_linkedin_text;
use context::pdf::extract_pdf_text;
use crate::state::AppState;

#[derive(serde::Serialize)]
pub struct SetupResponse {
    pub success: bool,
    pub system_prompt_preview: String,
    pub message: String,
}

pub async fn handle_setup_finalize(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<SetupResponse>, (StatusCode, String)> {
    let mut payload = SetupPayload::default();
    let mut cv_bytes: Option<Vec<u8>> = None;
    let mut cv_filename = String::new();

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
            _ => {}
        }
    }

    // Extract CV text
    if let Some(bytes) = cv_bytes {
        if cv_filename.ends_with(".pdf") {
            payload.cv_text = extract_pdf_text(&bytes).unwrap_or_else(|_| String::new());
        } else {
            payload.cv_text = String::from_utf8_lossy(&bytes).to_string();
        }
    }

    // Crawl company website
    let company_info = if !payload.company_url.is_empty() {
        crawl_website(&payload.company_url, 30)
            .await
            .unwrap_or_default()
    } else {
        String::new()
    };

    // Parse LinkedIn
    let interviewer_info = parse_linkedin_text(&payload.linkedin_text);

    // Build system prompt
    let system_prompt = build_system_prompt(&payload, &company_info, &interviewer_info);

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

    Ok(Json(SetupResponse {
        success: true,
        system_prompt_preview: preview,
        message: "Setup complete".to_string(),
    }))
}
