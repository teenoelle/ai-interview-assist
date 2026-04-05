use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuggestionMode {
    Compound,
    Primary,
    Secondary,
    ClosingHr,
    ClosingHm,
    ClosingCeo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsEvent {
    Transcript { text: String, timestamp_ms: u64, speaker: String },
    Sentiment { emotion: String, reason: Option<String>, coaching: Option<String>, coaching_why: Option<String> },
    QuestionDetected { question: String, #[serde(skip_serializing_if = "Option::is_none")] secondary_tag: Option<String> },
    SuggestionToken { token: String, mode: SuggestionMode },
    SuggestionComplete { full_text: String, mode: SuggestionMode },
    Error { message: String },
    Status { message: String },
    RateLimit { provider: String, requests_remaining: u32, requests_limit: u32 },
    ProviderUsed { service: String, provider: String, local: bool },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptSegment {
    pub text: String,
    pub timestamp_ms: u64,
    pub speaker: String,   // "Interviewer" or "You"
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SetupPayload {
    pub job_description: String,
    pub job_location: String,
    pub company_url: String,
    pub portfolio_url: String,
    pub linkedin_text: String,
    pub interviewee_linkedin: String,
    pub extra_experience: String,
    pub cv_text: String,
    pub portfolio_text: String,
}
