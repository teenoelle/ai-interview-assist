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
}

pub fn is_rate_limit(err: &anyhow::Error) -> bool {
    let msg = err.to_string();
    msg.contains("429") || msg.contains("rate_limit") || msg.contains("Rate limit")
}
