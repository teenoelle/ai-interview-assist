use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use common::messages::{TranscriptSegment, WsEvent};
use common::rate_limiter::RateLimiter;

#[derive(Clone)]
#[allow(dead_code)]
pub struct AppState {
    pub system_prompt: Arc<RwLock<String>>,
    pub transcript: Arc<RwLock<Vec<TranscriptSegment>>>,
    pub audio_tx: mpsc::Sender<Vec<u8>>,
    pub mic_audio_tx: mpsc::Sender<Vec<u8>>,
    pub video_tx: mpsc::Sender<Vec<u8>>,
    pub question_tx: mpsc::Sender<String>,
    pub event_tx: broadcast::Sender<WsEvent>,
    pub gemini_key: String,
    pub anthropic_key: Option<String>,
    pub groq_key: Option<String>,
    pub groq_key_2: Option<String>,
    pub openrouter_key: Option<String>,
    pub mistral_key: Option<String>,
    pub cerebras_key: Option<String>,
    pub qwen_key: Option<String>,
    pub ollama_url: String,
    pub ollama_model: String,
    pub ollama_models: Vec<String>,
    pub rate_limiter: RateLimiter,
    pub call_counts: Arc<std::sync::Mutex<std::collections::HashMap<String, u64>>>,
    pub piper_binary: Option<String>,
    pub piper_models_dir: Option<String>,
}
