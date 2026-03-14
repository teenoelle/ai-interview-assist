use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use axum::{
    routing::{get, post},
    Router,
};
use tower_http::{cors::CorsLayer, services::ServeDir};
use tracing_subscriber::EnvFilter;
use common::config::Config;
use common::messages::WsEvent;
use common::rate_limiter::RateLimiter;
use crate::state::AppState;

mod state;
mod ws_handler;
mod http_handler;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("server=debug".parse()?),
        )
        .init();

    let config = Config::from_env()?;

    let (audio_tx, audio_rx) = mpsc::channel::<Vec<u8>>(256);
    let (mic_audio_tx, mic_audio_rx) = mpsc::channel::<Vec<u8>>(256);
    let (video_tx, video_rx) = mpsc::channel::<Vec<u8>>(32);
    let (question_tx, question_rx) = mpsc::channel::<String>(64);
    let (event_tx, _event_rx) = broadcast::channel::<WsEvent>(512);

    let rate_limiter = RateLimiter::new();

    let active = |k: &Option<String>| if k.is_some() { "yes" } else { "no" };
    tracing::info!(
        "Providers — Gemini: yes | Anthropic: {} | Groq: {} | OpenRouter: {} | Cerebras: {} | Mistral: {} | Qwen: {}",
        active(&config.anthropic_api_key),
        active(&config.groq_api_key),
        active(&config.openrouter_api_key),
        active(&config.cerebras_api_key),
        active(&config.mistral_api_key),
        active(&config.qwen_api_key),
    );
    tracing::info!(
        "Suggestion order: {} OpenRouter → Qwen → Cerebras → Mistral → Groq → Gemini",
        if config.anthropic_api_key.is_some() { "Claude →" } else { "" }
    );
    tracing::info!("Transcription order: Groq Whisper → Gemini (both streams)");
    tracing::info!(
        "Sentiment: {} → Gemini Vision fallback",
        if config.anthropic_api_key.is_some() { "Claude Haiku (primary)" } else { "Gemini Vision only" }
    );
    tracing::info!(
        "Speaker diarization: {}",
        config.diarize_url.as_deref().unwrap_or("disabled (set HF_TOKEN to enable)")
    );

    let state = AppState {
        system_prompt: Arc::new(RwLock::new(String::new())),
        transcript: Arc::new(RwLock::new(Vec::new())),
        audio_tx,
        mic_audio_tx,
        video_tx,
        question_tx,
        event_tx: event_tx.clone(),
        gemini_key: config.gemini_api_key.clone(),
        anthropic_key: config.anthropic_api_key.clone(),
        groq_key: config.groq_api_key.clone(),
        openrouter_key: config.openrouter_api_key.clone(),
        mistral_key: config.mistral_api_key.clone(),
        cerebras_key: config.cerebras_api_key.clone(),
        qwen_key: config.qwen_api_key.clone(),
        rate_limiter: rate_limiter.clone(),
    };

    // Mic agent: microphone audio → always "You", never triggers suggestions
    tokio::spawn(transcription::run_mic_agent(
        mic_audio_rx,
        state.event_tx.clone(),
        state.transcript.clone(),
        config.gemini_api_key.clone(),
        config.groq_api_key.clone(),
        rate_limiter.clone(),
    ));

    // System audio agent: meeting playback → "Interviewer" (+ heuristic/diarize refinement)
    tokio::spawn(transcription::run_agent(
        audio_rx,
        state.question_tx.clone(),
        state.event_tx.clone(),
        state.transcript.clone(),
        config.gemini_api_key.clone(),
        config.groq_api_key.clone(),
        config.diarize_url.clone(),
        rate_limiter.clone(),
    ));

    tokio::spawn(sentiment::run_agent(
        video_rx,
        state.event_tx.clone(),
        config.gemini_api_key.clone(),
        config.anthropic_api_key.clone(),
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
        config.openrouter_api_key.clone(),
        config.mistral_api_key.clone(),
        config.cerebras_api_key.clone(),
        config.qwen_api_key.clone(),
        rate_limiter.clone(),
    ));

    let frontend_path = std::env::current_dir()
        .unwrap()
        .parent()
        .unwrap()
        .join("frontend")
        .join("dist");

    let app = Router::new()
        .route("/ws/audio", get(ws_handler::ws_audio))
        .route("/ws/audio/mic", get(ws_handler::ws_audio_mic))
        .route("/ws/video", get(ws_handler::ws_video))
        .route("/ws/events", get(ws_handler::ws_events))
        .route("/api/setup/finalize", post(http_handler::handle_setup_finalize))
        .route("/api/debrief", post(http_handler::handle_debrief))
        .route("/api/practice-question", post(http_handler::handle_practice_question))
        .route("/api/answer-feedback", post(http_handler::handle_answer_feedback))
        .fallback_service(ServeDir::new(&frontend_path))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", config.port);
    tracing::info!("Server listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
