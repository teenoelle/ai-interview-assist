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
    let (video_tx, video_rx) = mpsc::channel::<Vec<u8>>(32);
    let (question_tx, question_rx) = mpsc::channel::<String>(64);
    let (event_tx, _event_rx) = broadcast::channel::<WsEvent>(512);

    let state = AppState {
        system_prompt: Arc::new(RwLock::new(String::new())),
        transcript: Arc::new(RwLock::new(Vec::new())),
        audio_tx,
        video_tx,
        question_tx,
        event_tx: event_tx.clone(),
        gemini_key: config.gemini_api_key.clone(),
    };

    // Spawn agent tasks
    tokio::spawn(transcription::run_agent(
        audio_rx,
        state.question_tx.clone(),
        state.event_tx.clone(),
        state.transcript.clone(),
        config.gemini_api_key.clone(),
    ));

    tokio::spawn(sentiment::run_agent(
        video_rx,
        state.event_tx.clone(),
        config.gemini_api_key.clone(),
    ));

    tokio::spawn(suggestion::run_agent(
        question_rx,
        state.event_tx.clone(),
        state.system_prompt.clone(),
        state.transcript.clone(),
        config.gemini_api_key.clone(),
    ));

    let frontend_path = std::env::current_dir()
        .unwrap()
        .parent()
        .unwrap()
        .join("frontend")
        .join("dist");

    let app = Router::new()
        .route("/ws/audio", get(ws_handler::ws_audio))
        .route("/ws/video", get(ws_handler::ws_video))
        .route("/ws/events", get(ws_handler::ws_events))
        .route("/api/setup/finalize", post(http_handler::handle_setup_finalize))
        .nest_service("/", ServeDir::new(&frontend_path))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", config.port);
    tracing::info!("Server listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
