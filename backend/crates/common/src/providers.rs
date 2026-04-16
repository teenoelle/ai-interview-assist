/// Provider chain: tries each provider in order, falling back when one
/// returns a quota/rate-limit error (429 or RESOURCE_EXHAUSTED).

#[derive(Debug, Clone, PartialEq)]
pub enum Provider {
    Gemini,
    Groq,
    OpenRouter,
}

impl Provider {
    pub fn name(&self) -> &'static str {
        match self {
            Provider::Gemini => "Gemini",
            Provider::Groq => "Groq",
            Provider::OpenRouter => "OpenRouter",
        }
    }
}

/// All suggestion providers, in the order they are tried.
/// Serialised as snake_case strings for the frontend settings API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SuggestionProvider {
    Groq,
    Groq2,
    Mistral,
    ClaudeCli,
    ClaudeApi,
    Ollama,
    OpenRouter,
    Qwen,
    Cerebras,
    DeepSeek,
    LanOllama,
    Gemma,
    Gemini,
}

impl SuggestionProvider {
    pub fn name(self) -> &'static str {
        match self {
            SuggestionProvider::Groq      => "Groq",
            SuggestionProvider::Groq2     => "Groq #2",
            SuggestionProvider::Mistral   => "Mistral",
            SuggestionProvider::ClaudeCli => "Claude CLI",
            SuggestionProvider::ClaudeApi => "Claude API",
            SuggestionProvider::Ollama    => "Ollama",
            SuggestionProvider::OpenRouter => "OpenRouter",
            SuggestionProvider::Qwen      => "Qwen",
            SuggestionProvider::Cerebras  => "Cerebras",
            SuggestionProvider::DeepSeek  => "DeepSeek",
            SuggestionProvider::LanOllama => "LAN Ollama",
            SuggestionProvider::Gemma     => "Gemma",
            SuggestionProvider::Gemini    => "Gemini",
        }
    }

    /// True when the provider runs on the local machine (affects the `local` field in ProviderUsed events).
    pub fn is_local(self) -> bool {
        matches!(self, SuggestionProvider::Ollama | SuggestionProvider::ClaudeCli)
    }

    /// Default fallback order — tries fastest/cheapest first.
    pub fn default_order() -> Vec<Self> {
        vec![
            SuggestionProvider::Groq,
            SuggestionProvider::Mistral,
            SuggestionProvider::Groq2,
            SuggestionProvider::ClaudeCli,
            SuggestionProvider::ClaudeApi,
            SuggestionProvider::Ollama,
            SuggestionProvider::OpenRouter,
            SuggestionProvider::Qwen,
            SuggestionProvider::Cerebras,
            SuggestionProvider::DeepSeek,
            SuggestionProvider::LanOllama,
            SuggestionProvider::Gemma,
            SuggestionProvider::Gemini,
        ]
    }
}

/// Transcription provider fallback chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptionProvider {
    WhisperLocal,
    Deepgram,
    GroqWhisper2,
    GroqWhisper,
    Gemini,
}

impl TranscriptionProvider {
    pub fn name(self) -> &'static str {
        match self {
            TranscriptionProvider::WhisperLocal  => "Whisper (local)",
            TranscriptionProvider::Deepgram      => "Deepgram",
            TranscriptionProvider::GroqWhisper2  => "Groq Whisper #2",
            TranscriptionProvider::GroqWhisper   => "Groq Whisper",
            TranscriptionProvider::Gemini        => "Gemini",
        }
    }

    pub fn is_local(self) -> bool {
        matches!(self, TranscriptionProvider::WhisperLocal)
    }

    pub fn default_order() -> Vec<Self> {
        vec![
            TranscriptionProvider::WhisperLocal,
            TranscriptionProvider::Deepgram,
            TranscriptionProvider::GroqWhisper2,
            TranscriptionProvider::GroqWhisper,
            TranscriptionProvider::Gemini,
        ]
    }
}

/// Sentiment / facial-expression analysis provider chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SentimentProvider {
    OllamaVision,
    GeminiVision,
    ClaudeVision,
}

impl SentimentProvider {
    pub fn name(self) -> &'static str {
        match self {
            SentimentProvider::OllamaVision => "Ollama Vision",
            SentimentProvider::GeminiVision => "Gemini Vision",
            SentimentProvider::ClaudeVision => "Claude API",
        }
    }

    pub fn is_local(self) -> bool {
        matches!(self, SentimentProvider::OllamaVision)
    }

    pub fn default_order() -> Vec<Self> {
        vec![
            SentimentProvider::OllamaVision,
            SentimentProvider::GeminiVision,
            SentimentProvider::ClaudeVision,
        ]
    }
}

/// Returns true if the error is a quota / rate-limit error that warrants
/// falling back to the next provider (rather than retrying the same one).
pub fn is_quota_exhausted(err: &anyhow::Error) -> bool {
    let msg = err.to_string();
    // 429 rate limit (temporary — retry same provider after delay)
    // vs daily quota exhausted (permanent for today — skip to next provider)
    msg.contains("GenerateRequestsPerDayPerProjectPerModel")
        || msg.contains("GenerateContentInputTokensPerModelPerDay")
        || msg.contains("daily")
        || (msg.contains("429") && msg.contains("limit: 0"))
        || msg.contains("RESOURCE_EXHAUSTED")
        || msg.contains("exceeded your current quota")
        || msg.contains("quota_exceeded")
        || msg.contains("rate_limit_exceeded") // Groq
        || msg.contains("No credits")          // OpenRouter
        || msg.contains("insufficient_quota")  // OpenAI-compat
        || msg.contains("credit balance is too low") // Anthropic billing
        || msg.contains("invalid_api_key")          // Qwen / OpenAI-compat invalid key
        || msg.contains("Incorrect API key")        // Qwen / OpenAI-compat invalid key
        || msg.contains("context_length_exceeded") // prompt too long for this provider
        || msg.contains("Please reduce the length") // Cerebras context limit
        || msg.contains("model_permission_blocked") // Groq project-level model block
        || msg.contains("permission_denied")        // generic permission/access denied
}

pub fn is_rate_limit(err: &anyhow::Error) -> bool {
    let msg = err.to_string();
    msg.contains("429")
        || msg.contains("rate_limit")
        || msg.contains("Rate limit")
        || msg.contains("rate-limited")
        || msg.contains("Too Many Requests")
}

/// Returns true for transient server-side errors (5xx) that warrant
/// falling through to the next provider rather than hard-failing.
pub fn is_server_error(err: &anyhow::Error) -> bool {
    let msg = err.to_string();
    msg.contains("503")
        || msg.contains("502")
        || msg.contains("500")
        || msg.contains("Service Unavailable")
        || msg.contains("Bad Gateway")
        || msg.contains("upstream connect error")
        || msg.contains("overflow")
        || msg.contains("Internal Server Error")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn err(msg: &str) -> anyhow::Error {
        anyhow::anyhow!("{}", msg)
    }

    #[test]
    fn quota_exhausted_detects_daily_limit() {
        assert!(is_quota_exhausted(&err("GenerateRequestsPerDayPerProjectPerModel exceeded")));
        assert!(is_quota_exhausted(&err("RESOURCE_EXHAUSTED: quota limit")));
        assert!(is_quota_exhausted(&err("exceeded your current quota")));
        assert!(is_quota_exhausted(&err("No credits remaining")));
        assert!(is_quota_exhausted(&err("rate_limit_exceeded for daily")));
        assert!(is_quota_exhausted(&err("insufficient_quota")));
    }

    #[test]
    fn quota_exhausted_does_not_match_transient() {
        assert!(!is_quota_exhausted(&err("500 Internal Server Error")));
        assert!(!is_quota_exhausted(&err("connection refused")));
        assert!(!is_quota_exhausted(&err("404 Not Found")));
    }

    #[test]
    fn rate_limit_detects_429() {
        assert!(is_rate_limit(&err("429 Too Many Requests")));
        assert!(is_rate_limit(&err("temporarily rate-limited upstream")));
        assert!(is_rate_limit(&err("Rate limit reached, retry later")));
        assert!(is_rate_limit(&err("rate_limit exceeded")));
    }

    #[test]
    fn rate_limit_does_not_match_other_errors() {
        assert!(!is_rate_limit(&err("404 Not Found")));
        assert!(!is_rate_limit(&err("500 Internal Server Error")));
    }
}
