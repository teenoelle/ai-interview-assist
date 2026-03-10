use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    Json,
};
use common::messages::SetupPayload;
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

    Ok(Json(SetupResponse {
        success: true,
        system_prompt_preview: preview,
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
