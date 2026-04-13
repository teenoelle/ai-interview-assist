use axum::{
    extract::{Multipart, Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response, sse::{Event, Sse}},
    Json,
};
use std::convert::Infallible;
use tokio_stream::wrappers::ReceiverStream;
use crate::state::AppState;
use crate::review::{self, ReviewConfig, ReviewProgress, new_id};

fn make_cfg(state: &AppState, keywords: Vec<String>) -> ReviewConfig {
    ReviewConfig {
        gemini_key: state.gemini_key.clone(),
        anthropic_key: state.anthropic_key.clone(),
        mistral_key: state.mistral_key.clone(),
        bonsai_url: state.bonsai_url.clone(),
        bonsai_model: state.bonsai_model.clone(),
        groq_key: state.groq_key.clone(),
        groq_key_2: state.groq_key_2.clone(),
        ollama_url: state.ollama_url.clone(),
        ollama_model: state.ollama_model.clone(),
        whisper_url: state.whisper_url.clone(),
        whisper_model: state.whisper_model.clone(),
        diarize_url: state.diarize_url.clone(),
        keywords,
        reviews_dir: state.reviews_dir.clone(),
        ffmpeg_bin: state.ffmpeg_bin.clone(),
    }
}

// POST /api/review/upload
pub async fn handle_upload(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let id = new_id();
    let work_dir = state.reviews_dir.join(&id);
    tokio::fs::create_dir_all(&work_dir).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut source_path = None;
    let mut source_filename = "recording".to_string();

    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("file") {
            source_filename = field.file_name().unwrap_or("recording").to_string();
            let ext = source_filename.rsplit('.').next().unwrap_or("mp4");
            let dst = work_dir.join(format!("source.{}", ext));
            let data = field.bytes().await.map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
            tokio::fs::write(&dst, &data).await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            source_path = Some(dst);
        }
    }

    let src = source_path.ok_or((StatusCode::BAD_REQUEST, "No file field in upload".to_string()))?;

    let (watch_tx, watch_rx) = tokio::sync::watch::channel(ReviewProgress::default());
    {
        let mut sessions = state.review_sessions.lock().unwrap();
        sessions.insert(id.clone(), watch_rx);
    }

    let keywords = state.jd_keywords.read().await.clone();
    let cfg = make_cfg(&state, keywords);
    let id2 = id.clone();
    let fn2 = source_filename.clone();

    tokio::spawn(async move {
        if let Err(e) = review::process_review(id2, src, fn2, cfg, watch_tx.clone()).await {
            let _ = watch_tx.send(ReviewProgress {
                pct: 0, step: String::new(), done: true,
                error: Some(e.to_string()),
            });
        }
    });

    Ok(Json(serde_json::json!({ "id": id })))
}

// GET /api/review/:id/events  (SSE progress stream)
pub async fn handle_events(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Response {
    let (mpsc_tx, mpsc_rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(128);
    let watch_rx = state.review_sessions.lock().unwrap().get(&id).cloned();
    let reviews_dir = state.reviews_dir.clone();

    tokio::spawn(async move {
        let emit = |p: ReviewProgress| {
            let data = serde_json::to_string(&p).unwrap_or_default();
            Event::default().data(data)
        };

        if let Some(mut rx) = watch_rx {
            // Emit current state immediately
            let current = rx.borrow().clone();
            let done = current.done;
            let _ = mpsc_tx.send(Ok(emit(current))).await;
            if done { return; }
            // Stream subsequent changes
            while rx.changed().await.is_ok() {
                let p = rx.borrow().clone();
                let done = p.done;
                let _ = mpsc_tx.send(Ok(emit(p))).await;
                if done { return; }
            }
        } else {
            // Session not in memory — check if report is on disk (server restarted)
            let p = if review::load_report(&reviews_dir, &id).await.is_ok() {
                ReviewProgress { pct: 100, step: "Done".to_string(), done: true, error: None }
            } else {
                ReviewProgress { pct: 0, step: String::new(), done: true, error: Some("Not found".to_string()) }
            };
            let _ = mpsc_tx.send(Ok(emit(p))).await;
        }
    });

    Sse::new(ReceiverStream::new(mpsc_rx)).into_response()
}

// GET /api/review/:id
pub async fn handle_get_report(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<review::ReviewReport>, (StatusCode, String)> {
    review::load_report(&state.reviews_dir, &id).await
        .map(Json)
        .map_err(|_| (StatusCode::NOT_FOUND, "Report not found".to_string()))
}

// GET /api/review/:id/source  (stream the original recording for replay)
pub async fn handle_get_source(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Response, (StatusCode, String)> {
    let dir = state.reviews_dir.join(&id);
    let mut rd = tokio::fs::read_dir(&dir).await
        .map_err(|_| (StatusCode::NOT_FOUND, "Review not found".to_string()))?;
    while let Ok(Some(entry)) = rd.next_entry().await {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("source.") {
            let data = tokio::fs::read(entry.path()).await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            let mime = mime_for_ext(name.rsplit('.').next().unwrap_or("mp4"));
            return Ok(([(header::CONTENT_TYPE, mime)], data).into_response());
        }
    }
    Err((StatusCode::NOT_FOUND, "Source file not found".to_string()))
}

fn mime_for_ext(ext: &str) -> &'static str {
    match ext {
        "mp4" => "video/mp4", "webm" => "video/webm", "mov" => "video/quicktime",
        "mp3" => "audio/mpeg", "m4a" => "audio/mp4", "wav" => "audio/wav",
        "ogg" => "audio/ogg", _ => "application/octet-stream",
    }
}

// GET /api/reviews
pub async fn handle_list_reports(
    State(state): State<AppState>,
) -> Json<Vec<review::ReviewSummary>> {
    Json(review::list_summaries(&state.reviews_dir).await)
}

// DELETE /api/review/:id
pub async fn handle_delete_report(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    review::delete_review(&state.reviews_dir, &id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    state.review_sessions.lock().unwrap().remove(&id);
    Ok(Json(serde_json::json!({ "ok": true })))
}

// DELETE /api/reviews  (sweep all)
pub async fn handle_delete_all(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    review::delete_all_reviews(&state.reviews_dir).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    state.review_sessions.lock().unwrap().clear();
    Ok(Json(serde_json::json!({ "ok": true })))
}

// POST /api/review/from-live
pub async fn handle_from_live(
    State(state): State<AppState>,
) -> Result<Json<review::ReviewReport>, (StatusCode, String)> {
    let transcript = state.transcript.read().await.clone();
    if transcript.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "No transcript yet".to_string()));
    }
    let keywords = state.jd_keywords.read().await.clone();
    let cfg = make_cfg(&state, keywords.clone());
    review::generate_live_report(new_id(), transcript, keywords, cfg).await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

// GET /api/review/:id/download
pub async fn handle_download(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Response, (StatusCode, String)> {
    let report = review::load_report(&state.reviews_dir, &id).await
        .map_err(|_| (StatusCode::NOT_FOUND, "Report not found".to_string()))?;
    let md = review::format_markdown(&report);
    let filename = format!("interview-report-{}.md", &id[..8]);
    Ok((
        [
            (header::CONTENT_TYPE, "text/markdown; charset=utf-8"),
            (header::CONTENT_DISPOSITION, &format!("attachment; filename=\"{}\"", filename) as &str),
        ],
        md,
    ).into_response())
}
