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
use common::providers::{SuggestionProvider, TranscriptionProvider, SentimentProvider};
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

/// Extract the port number from a URL like "http://localhost:8000".
fn extract_port(url: &str) -> u16 {
    url.split(':')
        .last()
        .and_then(|s| s.split('/').next())
        .and_then(|s| s.parse().ok())
        .unwrap_or(8000)
}

/// Return the first port in [start, start+4] that is not already in use.
fn find_free_port(start: u16) -> u16 {
    for port in start..start + 5 {
        if std::net::TcpListener::bind(("127.0.0.1", port)).is_ok() {
            return port;
        }
    }
    start
}

/// Kill whatever process is currently listening on `port` (Windows only).
#[cfg(windows)]
fn kill_port_owner(port: u16) {
    let Ok(out) = std::process::Command::new("netstat").args(["-ano"]).output() else { return };
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines() {
        if !line.contains("LISTENING") { continue; }
        let parts: Vec<&str> = line.split_whitespace().collect();
        // netstat -ano: Proto  Local  Foreign  State  PID
        let addr_match = parts.get(1).map(|a| a.ends_with(&format!(":{}", port))).unwrap_or(false);
        if addr_match {
            if let Some(pid_str) = parts.last() {
                if let Ok(pid) = pid_str.parse::<u32>() {
                    if pid > 0 {
                        let _ = std::process::Command::new("taskkill")
                            .args(["/F", "/PID", &pid.to_string()])
                            .output();
                        tracing::info!("Killed PID {} that was holding whisper port {}", pid, port);
                    }
                }
            }
        }
    }
}
#[cfg(not(windows))]
fn kill_port_owner(_port: u16) {}

/// Poll the Whisper URL until it responds, or give up after ~60 s.
/// Waits 8 s before first attempt so the model has time to start loading,
/// then polls every 5 s (fewer hung connections vs rapid 2-s polling).
/// Once reachable, fires a silent warmup transcription to pre-load the model.
async fn wait_for_whisper(url: &str, whisper_model: &str) {
    tracing::info!("Waiting for local Whisper at {} …", url);
    // Initial delay — CPU model loading typically takes 15-30 s
    tokio::time::sleep(std::time::Duration::from_secs(8)).await;

    let client = reqwest::Client::new();
    let check = format!("{}/v1/models", url.trim_end_matches('/'));
    for attempt in 1u32..=12 {
        match client
            .get(&check)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
        {
            Ok(_) => {
                tracing::info!("Local Whisper ready (attempt {})", attempt);
                warmup_whisper(url, whisper_model).await;
                return;
            }
            Err(_) => {
                tracing::debug!("Whisper not ready yet (attempt {}/12), retrying in 5 s…", attempt);
                if attempt < 12 {
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }
            }
        }
    }
    tracing::warn!(
        "Local Whisper at {} did not respond — will fall back to cloud providers",
        url
    );
}

/// Send a minimal chat request to Ollama to load a model into RAM before the interview starts.
async fn warmup_ollama(base_url: &str, model: &str) {
    let url = format!("{}/v1/chat/completions", base_url.trim_end_matches('/'));
    let body = serde_json::json!({
        "model": model,
        "messages": [{ "role": "user", "content": "hi" }],
        "max_tokens": 1,
        "stream": false,
        "keep_alive": "60m",
    });
    tracing::info!("Warming up Ollama model {}…", model);
    match reqwest::Client::new()
        .post(&url)
        .json(&body)
        .timeout(std::time::Duration::from_secs(120))
        .send()
        .await
    {
        Ok(_) => tracing::info!("Ollama {} warm", model),
        Err(e) => tracing::warn!("Ollama {} warmup failed: {}", model, e),
    }
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
        .add_directive("transcription=debug".parse()?)
        .add_directive("context=info".parse()?);
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

    let mut config = Config::from_env()?;

    // Resolve whisper port: kill any existing owner, fall back to next free port if needed.
    if config.whisper_spawn_cmd.is_some() {
        if let Some(ref url) = config.whisper_url.clone() {
            let preferred = extract_port(url);
            kill_port_owner(preferred);
            // Brief pause so the OS releases the port before we check availability
            std::thread::sleep(std::time::Duration::from_millis(400));
            let actual = find_free_port(preferred);
            if actual != preferred {
                tracing::info!("Port {} still in use — whisper will use backup port {}", preferred, actual);
                config.whisper_url = Some(
                    url.replacen(&format!(":{}", preferred), &format!(":{}", actual), 1)
                );
                config.whisper_spawn_args = config.whisper_spawn_args.iter().map(|a| {
                    if a == &preferred.to_string() { actual.to_string() } else { a.clone() }
                }).collect();
            }
        }
    }

    // Optionally start the local Whisper process (non-blocking — server binds immediately)
    if let Some(ref cmd) = config.whisper_spawn_cmd {
        spawn_detached(cmd, &config.whisper_spawn_args);
    }
    if let Some(url) = config.whisper_url.clone() {
        let model = config.whisper_model.clone();
        tokio::spawn(async move { wait_for_whisper(&url, &model).await });
    }

    // Pre-warm Ollama models so they're loaded into RAM before the first interview question
    for model in &config.ollama_models {
        let url = config.ollama_url.clone();
        let model = model.clone();
        tokio::spawn(async move { warmup_ollama(&url, &model).await });
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
    {
        let default_names: Vec<&str> = SuggestionProvider::default_order()
            .iter().map(|p| p.name()).collect();
        tracing::info!("Default suggestion order: {}", default_names.join(" → "));
    }
    tracing::info!(
        "Transcription order: {} Groq Whisper → Gemini (both streams)",
        config.whisper_url.as_deref().map(|u| format!("Local Whisper ({u}) →")).unwrap_or_default()
    );
    tracing::info!(
        "Sentiment: Ollama Vision ({}) → Gemini Vision → Claude API",
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
        deepseek_key: config.deepseek_api_key.clone(),
        openrouter_key: config.openrouter_api_key.clone(),
        mistral_key: config.mistral_api_key.clone(),
        cerebras_key: config.cerebras_api_key.clone(),
        qwen_key: config.qwen_api_key.clone(),
        bonsai_url: config.bonsai_url.clone(),
        bonsai_model: config.bonsai_model.clone(),
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
        suggestion_order: Arc::new(RwLock::new(SuggestionProvider::default_order())),
        transcription_order: Arc::new(RwLock::new(TranscriptionProvider::default_order())),
        sentiment_order: Arc::new(RwLock::new(SentimentProvider::default_order())),
        runtime_keys: Arc::new(RwLock::new(std::collections::HashMap::new())),
        runtime_urls: Arc::new(RwLock::new(std::collections::HashMap::new())),
        runtime_models: Arc::new(RwLock::new(std::collections::HashMap::new())),
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
        state.transcription_order.clone(),
        state.runtime_keys.clone(),
        state.runtime_urls.clone(),
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
        state.transcription_order.clone(),
        state.runtime_keys.clone(),
        state.runtime_urls.clone(),
    ));

    tokio::spawn(sentiment::run_agent(
        video_rx,
        state.event_tx.clone(),
        config.gemini_api_key.clone(),
        config.anthropic_api_key.clone(),
        config.ollama_url.clone(),
        config.ollama_vision_model.clone(),
        sentiment_rl,
        state.sentiment_order.clone(),
        state.runtime_keys.clone(),
        state.runtime_urls.clone(),
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
        config.deepseek_api_key.clone(),
        config.mistral_api_key.clone(),
        config.cerebras_api_key.clone(),
        config.qwen_api_key.clone(),
        config.bonsai_url.clone(),
        config.bonsai_model.clone(),
        config.ollama_url.clone(),
        config.ollama_models.clone(),
        suggestion_rl,
        Some(call_counts.clone()),
        state.suggestion_order.clone(),
        state.runtime_keys.clone(),
        state.runtime_urls.clone(),
        state.runtime_models.clone(),
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
        .route("/api/settings", get(http_handler::handle_settings_get).post(http_handler::handle_settings))
        .route("/api/probe", get(http_handler::handle_probe))
        .route("/api/ollama/models", get(http_handler::handle_ollama_models))
        .route("/api/ollama/pull", post(http_handler::handle_ollama_pull))
        .route("/api/claude-cli/install", post(http_handler::handle_claude_cli_install))
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
