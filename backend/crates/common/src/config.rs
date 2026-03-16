use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct Config {
    pub gemini_api_key: String,
    pub groq_api_key: Option<String>,
    pub openrouter_api_key: Option<String>,
    pub mistral_api_key: Option<String>,
    pub cerebras_api_key: Option<String>,
    pub anthropic_api_key: Option<String>,
    pub qwen_api_key: Option<String>,
    pub whisper_url: Option<String>,
    pub whisper_model: String,
    pub ollama_url: String,
    pub ollama_model: String,
    pub ollama_vision_model: String,
    pub diarize_url: Option<String>,
    pub port: u16,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();
        let gemini_api_key = std::env::var("GEMINI_API_KEY")
            .context("GEMINI_API_KEY must be set in .env or environment")?;
        let groq_api_key = std::env::var("GROQ_API_KEY").ok();
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
            .unwrap_or_else(|_| "llama3.2".to_string());
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
        Ok(Self { gemini_api_key, groq_api_key, openrouter_api_key, mistral_api_key, cerebras_api_key, anthropic_api_key, qwen_api_key, whisper_url, whisper_model, ollama_url, ollama_model, ollama_vision_model, diarize_url, port })
    }
}
