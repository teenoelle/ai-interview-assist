use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsEvent {
    Transcript { text: String, timestamp_ms: u64, speaker: String },
    Sentiment { emotion: String },
    QuestionDetected { question: String },
    SuggestionToken { token: String },
    SuggestionComplete { full_text: String },
    Error { message: String },
    Status { message: String },
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
    pub company_url: String,
    pub linkedin_text: String,
    pub extra_experience: String,
    pub cv_text: String,
}
