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
        || msg.contains("context_length_exceeded") // prompt too long for this provider
        || msg.contains("Please reduce the length") // Cerebras context limit
}

pub fn is_rate_limit(err: &anyhow::Error) -> bool {
    let msg = err.to_string();
    msg.contains("429")
        || msg.contains("rate_limit")
        || msg.contains("Rate limit")
        || msg.contains("rate-limited")
        || msg.contains("Too Many Requests")
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
