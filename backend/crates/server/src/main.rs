use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::IntoResponse,
    routing::{delete, get, post},
    Router,
};
use tower_http::{cors::CorsLayer, services::ServeDir};
use axum::extract::DefaultBodyLimit;
use tracing_subscriber::EnvFilter;
use common::config::Config;
use common::messages::WsEvent;
use common::rate_limiter::RateLimiter;
use crate::state::AppState;

mod state;
mod ws_handler;
mod http_handler;
mod tts_handler;
mod review;
mod review_handler;


async fn auth_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> impl IntoResponse {
    if let Some(expected) = &state.app_token {
        // Check Authorization: Bearer <token> header
        let bearer = req.headers()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .map(|s| s.to_string());
        // Check ?token=<token> query param
        let query_token = req.uri().query().and_then(|q| {
            q.split('&').find_map(|pair| {
                let (k, v) = pair.split_once('=')?;
                if k == "token" { Some(v.to_string()) } else { None }
            })
        });
        let provided = bearer.or(query_token);
        if provided.as_deref() != Some(expected.as_str()) {
            return (StatusCode::UNAUTHORIZED, "Invalid or missing token").into_response();
        }
    }
    next.run(req).await.into_response()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("server=debug".parse()?)
                .add_directive("suggestion=info".parse()?),
        )
        .init();

    let config = Config::from_env()?;

    let (audio_tx, audio_rx) = mpsc::channel::<Vec<u8>>(256);
    let (mic_audio_tx, mic_audio_rx) = mpsc::channel::<Vec<u8>>(256);
    let (video_tx, video_rx) = mpsc::channel::<Vec<u8>>(32);
    let (question_tx, question_rx) = mpsc::channel::<String>(64);
    let (event_tx, _event_rx) = broadcast::channel::<WsEvent>(512);

    let rate_limiter = RateLimiter::new();
    let call_counts = Arc::new(std::sync::Mutex::new(std::collections::HashMap::<String, u64>::new()));

    let active = |k: &Option<String>| if k.is_some() { "yes" } else { "no" };
    tracing::info!(
        "Providers — Gemini: yes | Anthropic: {} | Groq: {}{} | OpenRouter: {} | Cerebras: {} | Mistral: {} | Qwen: {} | Ollama: {} ({})",
        active(&config.anthropic_api_key),
        active(&config.groq_api_key),
        if config.groq_api_key_2.is_some() { "+key2" } else { "" },
        active(&config.openrouter_api_key),
        active(&config.cerebras_api_key),
        active(&config.mistral_api_key),
        active(&config.qwen_api_key),
        config.ollama_url,
        config.ollama_model,
    );
    tracing::info!(
        "Suggestion order: {} Groq (8b-instant) → Cerebras → OpenRouter → Qwen → Mistral → Ollama ({}) → Gemini",
        if config.anthropic_api_key.is_some() { "Claude Haiku →" } else { "" },
        config.ollama_models.join(", "),
    );
    tracing::info!(
        "Transcription order: {} Groq Whisper → Gemini (both streams)",
        config.whisper_url.as_deref().map(|u| format!("Local Whisper ({u}) →")).unwrap_or_default()
    );
    tracing::info!(
        "Sentiment: {} → Ollama Vision ({}) → Gemini Vision",
        if config.anthropic_api_key.is_some() { "Claude Haiku" } else { "Ollama Vision" },
        config.ollama_vision_model,
    );
    tracing::info!(
        "Speaker diarization: {}",
        config.diarize_url.as_deref().unwrap_or("disabled (set HF_TOKEN to enable)")
    );

    let reviews_dir = std::env::current_dir()
        .unwrap()
        .parent()
        .unwrap()
        .join("reviews");
    tokio::fs::create_dir_all(&reviews_dir).await?;

    let state = AppState {
        system_prompt: Arc::new(RwLock::new(String::new())),
        prediction_context: Arc::new(RwLock::new(String::new())),
        company_info: Arc::new(RwLock::new(String::new())),
        linkedin_text: Arc::new(RwLock::new(String::new())),
        company_url: Arc::new(RwLock::new(String::new())),
        portfolio_url: Arc::new(RwLock::new(String::new())),
        jd_text: Arc::new(RwLock::new(String::new())),
        transcript: Arc::new(RwLock::new(Vec::new())),
        jd_keywords: Arc::new(RwLock::new(Vec::new())),
        audio_tx,
        mic_audio_tx,
        video_tx,
        question_tx,
        event_tx: event_tx.clone(),
        gemini_key: config.gemini_api_key.clone(),
        anthropic_key: config.anthropic_api_key.clone(),
        groq_key: config.groq_api_key.clone(),
        groq_key_2: config.groq_api_key_2.clone(),
        openrouter_key: config.openrouter_api_key.clone(),
        mistral_key: config.mistral_api_key.clone(),
        cerebras_key: config.cerebras_api_key.clone(),
        qwen_key: config.qwen_api_key.clone(),
        ollama_url: config.ollama_url.clone(),
        ollama_model: config.ollama_model.clone(),
        ollama_models: config.ollama_models.clone(),
        whisper_url: config.whisper_url.clone(),
        whisper_model: config.whisper_model.clone(),
        diarize_url: config.diarize_url.clone(),
        rate_limiter: rate_limiter.clone(),
        call_counts: call_counts.clone(),
        piper_binary: config.piper_binary.clone(),
        piper_models_dir: config.piper_models_dir.clone(),
        app_token: config.app_token.clone(),
        ffmpeg_bin: config.ffmpeg_bin.clone(),
        reviews_dir,
        review_sessions: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
    };

    // Mic agent: microphone audio → always "You", never triggers suggestions
    tokio::spawn(transcription::run_mic_agent(
        mic_audio_rx,
        state.event_tx.clone(),
        state.transcript.clone(),
        config.gemini_api_key.clone(),
        config.groq_api_key.clone(),
        config.groq_api_key_2.clone(),
        config.whisper_url.clone(),
        config.whisper_model.clone(),
        rate_limiter.clone(),
        Some(call_counts.clone()),
    ));

    // System audio agent: meeting playback → "Interviewer" (+ heuristic/diarize refinement)
    tokio::spawn(transcription::run_agent(
        audio_rx,
        state.question_tx.clone(),
        state.event_tx.clone(),
        state.transcript.clone(),
        config.gemini_api_key.clone(),
        config.groq_api_key.clone(),
        config.groq_api_key_2.clone(),
        config.whisper_url.clone(),
        config.whisper_model.clone(),
        config.diarize_url.clone(),
        rate_limiter.clone(),
        Some(call_counts.clone()),
    ));

    tokio::spawn(sentiment::run_agent(
        video_rx,
        state.event_tx.clone(),
        config.gemini_api_key.clone(),
        config.anthropic_api_key.clone(),
        config.ollama_url.clone(),
        config.ollama_vision_model.clone(),
        rate_limiter.clone(),
    ));

    tokio::spawn(suggestion::run_agent(
        question_rx,
        state.event_tx.clone(),
        state.system_prompt.clone(),
        state.transcript.clone(),
        config.gemini_api_key.clone(),
        config.anthropic_api_key.clone(),
        config.groq_api_key.clone(),
        config.groq_api_key_2.clone(),
        config.openrouter_api_key.clone(),
        config.mistral_api_key.clone(),
        config.cerebras_api_key.clone(),
        config.qwen_api_key.clone(),
        config.ollama_url.clone(),
        config.ollama_models.clone(),
        rate_limiter.clone(),
        Some(call_counts.clone()),
    ));

    let frontend_path = std::env::current_dir()
        .unwrap()
        .parent()
        .unwrap()
        .join("frontend")
        .join("dist");

    let api_routes = Router::new()
        .route("/api/setup/finalize", post(http_handler::handle_setup_finalize))
        .route("/api/debrief", post(http_handler::handle_debrief))
        .route("/api/practice-question", post(http_handler::handle_practice_question))
        .route("/api/answer-feedback", post(http_handler::handle_answer_feedback))
        .route("/api/next-question", post(http_handler::handle_next_question))
        .route("/api/salary-coach", post(http_handler::handle_salary_coach))
        .route("/api/score-practice", post(http_handler::handle_score_practice))
        .route("/api/vocal-sentiment", post(http_handler::handle_vocal_sentiment))
        .route("/api/keyword-definition", post(http_handler::handle_keyword_definition))
        .route("/api/expand-cue", post(http_handler::handle_expand_cue))
        .route("/api/next-steps", post(http_handler::handle_next_steps))
        .route("/api/presence-check", post(http_handler::handle_presence_check))
        .route("/api/simulate-question", post(http_handler::handle_simulate_question))
        .route("/api/predict-questions", post(http_handler::handle_predict_questions))
        .route("/api/enrich", post(http_handler::handle_enrich))
        .route("/api/interviewer-summaries", post(http_handler::handle_interviewer_summaries))
        .route("/api/usage", get(http_handler::handle_usage))
        .route("/api/tts/voices", get(tts_handler::handle_tts_voices))
        .route("/api/tts/speak", post(tts_handler::handle_speak))
        .route("/api/review/upload", post(review_handler::handle_upload))
        .route("/api/review/from-live", post(review_handler::handle_from_live))
        .route("/api/reviews", get(review_handler::handle_list_reports))
        .route("/api/reviews", delete(review_handler::handle_delete_all))
        .route("/api/review/{id}", get(review_handler::handle_get_report))
        .route("/api/review/{id}", delete(review_handler::handle_delete_report))
        .route("/api/review/{id}/events", get(review_handler::handle_events))
        .route("/api/review/{id}/source", get(review_handler::handle_get_source))
        .route("/api/review/{id}/download", get(review_handler::handle_download))
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware));

    let app = Router::new()
        .route("/ws/audio", get(ws_handler::ws_audio))
        .route("/ws/audio/mic", get(ws_handler::ws_audio_mic))
        .route("/ws/video", get(ws_handler::ws_video))
        .route("/ws/events", get(ws_handler::ws_events))
        .merge(api_routes)
        .fallback_service(ServeDir::new(&frontend_path))
        .layer(CorsLayer::permissive())
        .layer(DefaultBodyLimit::max(500 * 1024 * 1024)) // 500 MB — review video uploads
        .with_state(state);

    let addr = format!("0.0.0.0:{}", config.port);
    tracing::info!("Server listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
