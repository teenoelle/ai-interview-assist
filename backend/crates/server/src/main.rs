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
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};
use tracing_appender::non_blocking::WorkerGuard;
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

/// Spawn a detached process (fire-and-forget — the child outlives the server process).
fn spawn_detached(cmd: &str, args: &[String]) {
    match std::process::Command::new(cmd).args(args).spawn() {
        Ok(_) => tracing::info!("Spawned Whisper process: {} {:?}", cmd, args),
        Err(e) => tracing::warn!("Failed to spawn Whisper process '{}': {}", cmd, e),
    }
}

/// Poll the Whisper URL root until it responds, or give up after ~30 s.
/// Once reachable, fire a silent dummy transcription to pre-load the model
/// so the first real segment doesn't incur a 60 s model-loading delay.
async fn wait_for_whisper(url: &str, whisper_model: &str) {
    let client = reqwest::Client::new();
    let check = format!("{}/", url.trim_end_matches('/'));
    tracing::info!("Waiting for local Whisper at {} …", url);
    for attempt in 1u32..=15 {
        let result = client
            .get(&check)
            .timeout(std::time::Duration::from_secs(2))
            .send()
            .await;
        match result {
            // Any HTTP response (even 404) means the server is up
            Ok(_) => {
                tracing::info!("Local Whisper ready (attempt {})", attempt);
                warmup_whisper(url, whisper_model).await;
                return;
            }
            Err(_) => {
                if attempt < 15 {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            }
        }
    }
    tracing::warn!(
        "Local Whisper at {} did not respond within 30 s — will fall back to Groq/Gemini",
        url
    );
}

/// Send a tiny silent WAV to Whisper to trigger model loading before any real audio arrives.
async fn warmup_whisper(url: &str, model: &str) {
    // 1 second of silence: 16000 samples × 2 bytes = 32000 bytes of zeros
    let silent_pcm = vec![0u8; 16000 * 2];
    let endpoint = format!("{}/v1/audio/transcriptions", url.trim_end_matches('/'));
    tracing::info!("Warming up local Whisper model (pre-loading) …");
    match transcription::groq::transcribe_openai_asr(&endpoint, "", model, &silent_pcm, 120).await {
        Ok(_) => tracing::info!("Local Whisper model warm — first real segment will be fast"),
        Err(e) => tracing::warn!("Whisper warmup failed (model may still load on first use): {}", e),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Write logs to both stdout and backend/logs/server.log (created relative to the exe's dir).
    let log_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("logs")))
        .unwrap_or_else(|| std::path::PathBuf::from("logs"));
    std::fs::create_dir_all(&log_dir).ok();
    let file_appender = tracing_appender::rolling::never(&log_dir, "server.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    // Keep _guard alive for the duration of main so the background writer flushes on exit.
    let _log_guard: WorkerGuard = _guard;

    let timer = tracing_subscriber::fmt::time::LocalTime::new(
        time::macros::format_description!("[year]-[month]-[day] [hour]:[minute]:[second]"),
    );
    let env_filter = EnvFilter::from_default_env()
        .add_directive("server=debug".parse()?)
        .add_directive("suggestion=info".parse()?)
        .add_directive("transcription=debug".parse()?);
    let file_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_timer(timer.clone())
        .compact()
        .with_writer(non_blocking);
    let stdout_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_timer(timer)
        .compact();
    tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer)
        .with(stdout_layer)
        .init();

    let config = Config::from_env()?;

    // Optionally start the local Whisper / Ollama process (non-blocking — server binds immediately)
    if let Some(ref cmd) = config.whisper_spawn_cmd {
        spawn_detached(cmd, &config.whisper_spawn_args);
    }
    if let Some(url) = config.whisper_url.clone() {
        let model = config.whisper_model.clone();
        tokio::spawn(async move { wait_for_whisper(&url, &model).await });
    }

    // Pre-warm Claude TLS connections so the first suggestion/test-question isn't slow
    if let Some(ref key) = config.anthropic_api_key {
        let key1 = key.clone();
        let key2 = key.clone();
        tokio::spawn(async move {
            suggestion::claude_llm::prewarm(&key1).await;
            tracing::info!("Claude connection pre-warmed (suggestion path)");
        });
        tokio::spawn(async move {
            context::ai_helper::prewarm(&key2).await;
            tracing::info!("Claude connection pre-warmed (ai_helper path)");
        });
    }

    let (audio_tx, audio_rx) = mpsc::channel::<Vec<u8>>(256);
    let (mic_audio_tx, mic_audio_rx) = mpsc::channel::<Vec<u8>>(256);
    let (video_tx, video_rx) = mpsc::channel::<Vec<u8>>(32);
    let (question_tx, question_rx) = mpsc::channel::<String>(64);
    let (event_tx, _event_rx) = broadcast::channel::<WsEvent>(512);

    let mic_rl        = RateLimiter::new(); // Gemini fallback for mic transcription
    let system_rl     = RateLimiter::new(); // Gemini fallback for system-audio transcription
    let sentiment_rl  = RateLimiter::new(); // Gemini/vision sentiment — isolated so it never competes with transcription
    let suggestion_rl = RateLimiter::new(); // suggestion chain — already separate
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
        jd_location: Arc::new(RwLock::new(String::new())),
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
        deepgram_key: config.deepgram_api_key.clone(),
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
        rate_limiter: suggestion_rl.clone(),
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
        config.deepgram_api_key.clone(),
        config.whisper_url.clone(),
        config.whisper_model.clone(),
        mic_rl,
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
        config.deepgram_api_key.clone(),
        config.whisper_url.clone(),
        config.whisper_model.clone(),
        config.diarize_url.clone(),
        system_rl,
        Some(call_counts.clone()),
    ));

    tokio::spawn(sentiment::run_agent(
        video_rx,
        state.event_tx.clone(),
        config.gemini_api_key.clone(),
        config.anthropic_api_key.clone(),
        config.ollama_url.clone(),
        config.ollama_vision_model.clone(),
        sentiment_rl,
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
        suggestion_rl,
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
        .route("/api/extract-file", post(http_handler::handle_extract_file))
        .route("/api/debrief", post(http_handler::handle_debrief))
        .route("/api/followup-email", post(http_handler::handle_followup_email))
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
        .route("/api/suggest-mode", post(http_handler::handle_suggest_mode))
        .route("/api/predict-questions", post(http_handler::handle_predict_questions))
        .route("/api/enrich", post(http_handler::handle_enrich))
        .route("/api/draft-followup", post(http_handler::handle_draft_followup))
        .route("/api/interviewer-summaries", post(http_handler::handle_interviewer_summaries))
        .route("/api/interviewer-summary", post(http_handler::handle_interviewer_summary_single))
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
