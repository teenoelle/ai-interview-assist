use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct Config {
    pub gemini_api_key: String,
    pub port: u16,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();
        let gemini_api_key = std::env::var("GEMINI_API_KEY")
            .context("GEMINI_API_KEY must be set in .env or environment")?;
        let port = std::env::var("PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse::<u16>()
            .context("PORT must be a valid number")?;
        Ok(Self { gemini_api_key, port })
    }
}
