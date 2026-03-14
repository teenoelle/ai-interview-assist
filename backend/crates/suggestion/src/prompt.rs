use common::messages::TranscriptSegment;

const BEHAVIORAL_TRIGGERS: &[&str] = &[
    "tell me about a time",
    "describe a situation",
    "give me an example",
    "walk me through",
    "have you ever",
    "what was a time",
    "share an experience",
    "when have you",
    "describe when",
    "describe a time",
    "can you give",
];

pub fn is_behavioral(question: &str) -> bool {
    let q = question.to_lowercase();
    BEHAVIORAL_TRIGGERS.iter().any(|&t| q.contains(t))
}

pub fn build_user_prompt(question: &str, transcript: &[TranscriptSegment]) -> String {
    let recent: Vec<&TranscriptSegment> = transcript.iter().rev().take(10).collect();
    let context = recent
        .iter()
        .rev()
        .map(|s| s.text.as_str())
        .collect::<Vec<_>>()
        .join(" ... ");

    let ctx_prefix = if context.is_empty() {
        String::new()
    } else {
        format!("Recent conversation: {}\n\n", context)
    };

    if is_behavioral(question) {
        format!(
            "{}The interviewer asked a behavioral question: '{}'\n\n\
Reply in this EXACT format — no extra text:\n\
Say: [opening sentence — hook with the core outcome or action, conversational, 15 words max]\n\
---\n\
**S:** [Situation — one short sentence, feels like you're recalling it naturally]\n\
**T:** [Task — one short sentence, what you were responsible for]\n\
**A:** [Actions — 1-2 short sentences, use **bold** on 1-2 key verbs or decisions]\n\
**R:** [Result — one short sentence with a specific outcome or number if possible]\n\
Ask: [one optional natural follow-up question, or omit this line]\n\n\
Use ONLY documented experience. Write short declarative sentences — no corporate jargon. If no clear match exists, say so honestly in the Say line.",
            ctx_prefix, question
        )
    } else {
        format!(
            "{}The interviewer asked: '{}'\n\n\
Reply in this EXACT format — no extra text:\n\
Say: [the single strongest thing to open with — direct, confident, 12 words max]\n\
---\n\
[Sentence 2: one natural elaboration — feels like thinking aloud, under 15 words]\n\
[Sentence 3: one concrete detail or example — short, spontaneous, under 15 words. Omit if not needed.]\n\
Ask: [one optional follow-up question to invite dialogue, or omit this line]\n\n\
Rules: Use ONLY documented background. Every sentence under 15 words. No bullets. No jargon. Write like you are genuinely recalling — short, confident, natural.",
            ctx_prefix, question
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(text: &str) -> TranscriptSegment {
        TranscriptSegment { text: text.to_string(), timestamp_ms: 0, speaker: String::new() }
    }

    #[test]
    fn no_transcript_gives_simple_prompt() {
        let p = build_user_prompt("Tell me about yourself", &[]);
        assert!(p.contains("Tell me about yourself"));
        assert!(!p.contains("Recent conversation"));
    }

    #[test]
    fn transcript_context_included() {
        let transcript = vec![seg("I worked at Acme"), seg("Then I moved to startup land")];
        let p = build_user_prompt("What's your background?", &transcript);
        assert!(p.contains("Recent conversation"));
        assert!(p.contains("Acme"));
    }

    #[test]
    fn only_last_10_segments_used() {
        let transcript: Vec<TranscriptSegment> = (0..15).map(|i| seg(&format!("seg{}", i))).collect();
        let p = build_user_prompt("question", &transcript);
        assert!(p.contains("seg14"));
        assert!(!p.contains("seg4"));
    }

    #[test]
    fn behavioral_question_triggers_star_format() {
        let p = build_user_prompt("Tell me about a time you led a team", &[]);
        assert!(p.contains("**S:**"));
        assert!(p.contains("**R:**"));
    }

    #[test]
    fn non_behavioral_uses_prose_format() {
        let p = build_user_prompt("What are your strengths?", &[]);
        assert!(!p.contains("**S:**"));
        assert!(p.contains("Say:"));
        assert!(!p.contains("•"));
    }
}
