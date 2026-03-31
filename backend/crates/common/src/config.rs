use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct Config {
    pub gemini_api_key: String,
    pub groq_api_key: Option<String>,
    pub groq_api_key_2: Option<String>,
    pub openrouter_api_key: Option<String>,
    pub mistral_api_key: Option<String>,
    pub cerebras_api_key: Option<String>,
    pub anthropic_api_key: Option<String>,
    pub qwen_api_key: Option<String>,
    pub whisper_url: Option<String>,
    pub whisper_model: String,
    pub ollama_url: String,
    pub ollama_model: String,
    pub ollama_models: Vec<String>,  // suggestion fallback chain (may include multiple)
    pub ollama_vision_model: String,
    pub diarize_url: Option<String>,
    pub port: u16,
    pub piper_binary: Option<String>,
    pub piper_models_dir: Option<String>,
    pub app_token: Option<String>,
    pub ffmpeg_bin: Option<String>,
    /// Optional command to spawn the local Whisper / Ollama server (e.g. "ollama")
    pub whisper_spawn_cmd: Option<String>,
    /// Arguments for whisper_spawn_cmd (e.g. ["serve"])
    pub whisper_spawn_args: Vec<String>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();
        let gemini_api_key = std::env::var("GEMINI_API_KEY")
            .context("GEMINI_API_KEY must be set in .env or environment")?;
        let groq_api_key = std::env::var("GROQ_API_KEY").ok();
        let groq_api_key_2 = std::env::var("GROQ_API_KEY_2").ok();
        let openrouter_api_key = std::env::var("OPENROUTER_API_KEY").ok();
        let mistral_api_key = std::env::var("MISTRAL_API_KEY").ok();
        let cerebras_api_key = std::env::var("CEREBRAS_API_KEY").ok();
        let anthropic_api_key = std::env::var("ANTHROPIC_API_KEY").ok();
        let qwen_api_key = std::env::var("QWEN_API_KEY").ok();
        // Local Whisper: set WHISPER_URL to e.g. http://localhost:8000
        // Works with faster-whisper-server, whisper.cpp HTTP server, or any OpenAI ASR-compat server
        let whisper_url = std::env::var("WHISPER_URL").ok();
        let whisper_model = std::env::var("WHISPER_MODEL")
            .unwrap_or_else(|_| "Systran/faster-whisper-large-v3".to_string());
        let ollama_url = std::env::var("OLLAMA_URL")
            .unwrap_or_else(|_| "http://localhost:11434".to_string());
        let ollama_model = std::env::var("OLLAMA_MODEL")
            .unwrap_or_else(|_| "llama3.2:latest".to_string());
        let ollama_models = std::env::var("OLLAMA_MODELS")
            .map(|s| s.split(',').map(|m| m.trim().to_string()).filter(|m| !m.is_empty()).collect::<Vec<_>>())
            .unwrap_or_else(|_| vec![ollama_model.clone()]);
        let ollama_vision_model = std::env::var("OLLAMA_VISION_MODEL")
            .unwrap_or_else(|_| "llava".to_string());
        // Diarization sidecar — optional; defaults to localhost:8001 if HF_TOKEN is set
        let diarize_url = if std::env::var("HF_TOKEN").is_ok() {
            Some(
                std::env::var("DIARIZE_URL")
                    .unwrap_or_else(|_| "http://127.0.0.1:8001".to_string()),
            )
        } else {
            std::env::var("DIARIZE_URL").ok()
        };
        let port = std::env::var("PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse::<u16>()
            .context("PORT must be a valid number")?;
        let piper_binary = std::env::var("PIPER_BINARY").ok().filter(|s| !s.is_empty());
        let piper_models_dir = std::env::var("PIPER_MODELS_DIR").ok().filter(|s| !s.is_empty());
        let app_token = std::env::var("APP_TOKEN").ok().filter(|s| !s.is_empty());
        let ffmpeg_bin = std::env::var("FFMPEG_BIN").ok().filter(|s| !s.is_empty());
        let whisper_spawn_cmd = std::env::var("WHISPER_SPAWN_CMD").ok().filter(|s| !s.is_empty());
        let whisper_spawn_args = std::env::var("WHISPER_SPAWN_ARGS")
            .unwrap_or_default()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
        Ok(Self { gemini_api_key, groq_api_key, groq_api_key_2, openrouter_api_key, mistral_api_key, cerebras_api_key, anthropic_api_key, qwen_api_key, whisper_url, whisper_model, ollama_url, ollama_model, ollama_models, ollama_vision_model, diarize_url, port, piper_binary, piper_models_dir, app_token, ffmpeg_bin, whisper_spawn_cmd, whisper_spawn_args })
    }
}
