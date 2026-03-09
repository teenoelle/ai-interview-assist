use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use common::messages::{TranscriptSegment, WsEvent};

#[derive(Clone)]
pub struct AppState {
    pub system_prompt: Arc<RwLock<String>>,
    pub transcript: Arc<RwLock<Vec<TranscriptSegment>>>,
    pub audio_tx: mpsc::Sender<Vec<u8>>,
    pub video_tx: mpsc::Sender<Vec<u8>>,
    pub question_tx: mpsc::Sender<String>,
    pub event_tx: broadcast::Sender<WsEvent>,
    pub gemini_key: String,
}
