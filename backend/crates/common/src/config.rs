use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct Config {
    pub gemini_api_key: String,
    pub groq_api_key: Option<String>,
    pub openrouter_api_key: Option<String>,
    pub mistral_api_key: Option<String>,
    pub cerebras_api_key: Option<String>,
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
        let port = std::env::var("PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse::<u16>()
            .context("PORT must be a valid number")?;
        Ok(Self { gemini_api_key, groq_api_key, openrouter_api_key, mistral_api_key, cerebras_api_key, port })
    }
}
