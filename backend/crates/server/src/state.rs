use std::sync::Arc;
use std::path::PathBuf;
use tokio::sync::{broadcast, mpsc, RwLock, watch};
use common::messages::{TranscriptSegment, WsEvent};
use common::rate_limiter::RateLimiter;
use common::providers::{SuggestionProvider, TranscriptionProvider, SentimentProvider};
use crate::review::ReviewProgress;

#[derive(Clone)]
#[allow(dead_code)]
pub struct AppState {
    pub system_prompt: Arc<RwLock<String>>,
    pub prediction_context: Arc<RwLock<String>>,
    pub company_info: Arc<RwLock<String>>,
    pub linkedin_text: Arc<RwLock<String>>,
    pub company_url: Arc<RwLock<String>>,
    pub portfolio_url: Arc<RwLock<String>>,
    pub jd_text: Arc<RwLock<String>>,
    pub jd_location: Arc<RwLock<String>>,
    pub transcript: Arc<RwLock<Vec<TranscriptSegment>>>,
    pub jd_keywords: Arc<RwLock<Vec<String>>>,
    pub audio_tx: mpsc::Sender<Vec<u8>>,
    pub mic_audio_tx: mpsc::Sender<Vec<u8>>,
    pub video_tx: mpsc::Sender<Vec<u8>>,
    pub question_tx: mpsc::Sender<String>,
    pub event_tx: broadcast::Sender<WsEvent>,
    pub gemini_key: String,
    pub anthropic_key: Option<String>,
    pub groq_key: Option<String>,
    pub groq_key_2: Option<String>,
    pub deepgram_key: Option<String>,
    pub openrouter_key: Option<String>,
    pub deepseek_key: Option<String>,
    pub mistral_key: Option<String>,
    pub cerebras_key: Option<String>,
    pub qwen_key: Option<String>,
    pub bonsai_url: Option<String>,
    pub bonsai_model: String,
    pub ollama_url: String,
    pub ollama_model: String,
    pub ollama_models: Vec<String>,
    pub whisper_url: Option<String>,
    pub whisper_model: String,
    pub diarize_url: Option<String>,
    pub rate_limiter: RateLimiter,
    pub call_counts: Arc<std::sync::Mutex<std::collections::HashMap<String, u64>>>,
    pub piper_binary: Option<String>,
    pub piper_models_dir: Option<String>,
    pub app_token: Option<String>,
    pub ffmpeg_bin: Option<String>,
    pub reviews_dir: PathBuf,
    pub review_sessions: Arc<std::sync::Mutex<std::collections::HashMap<String, watch::Receiver<ReviewProgress>>>>,
    pub suggestion_order: Arc<RwLock<Vec<SuggestionProvider>>>,
    pub transcription_order: Arc<RwLock<Vec<TranscriptionProvider>>>,
    pub sentiment_order: Arc<RwLock<Vec<SentimentProvider>>>,
    /// Keys added at runtime via POST /api/settings (override .env values).
    pub runtime_keys: Arc<RwLock<std::collections::HashMap<String, String>>>,
    /// URLs added at runtime via POST /api/settings (override .env values).
    /// Keys: "ollama", "lan_ollama", "whisper"
    pub runtime_urls: Arc<RwLock<std::collections::HashMap<String, String>>>,
    /// Models set at runtime via POST /api/settings.
    /// Keys: "ollama", "openrouter"
    pub runtime_models: Arc<RwLock<std::collections::HashMap<String, String>>>,
}
