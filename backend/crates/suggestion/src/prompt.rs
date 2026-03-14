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
Tell: [one sentence STAR summary — what to say first, 15 words max]\n\
---\n\
**S:** [Situation, 1 sentence]\n\
**T:** [Task/responsibility, 1 sentence]\n\
**A:** [Actions taken, 1-2 sentences with **bold** keywords]\n\
**R:** [Measurable result, 1 sentence]\n\
Ask: [one optional clarifying question the candidate could ask, or omit this line]\n\n\
Use ONLY the candidate's documented experience. If no match, say so honestly in the Tell line.",
            ctx_prefix, question
        )
    } else {
        format!(
            "{}The interviewer asked: '{}'\n\n\
Reply in this EXACT format — no extra text:\n\
Tell: [one sentence of the strongest thing to say first, 15 words max]\n\
---\n\
• **[Keyword]** — [1 short sentence elaboration]\n\
• **[Keyword]** — [1 short sentence elaboration]\n\
• **[Keyword]** — [1 short sentence elaboration]\n\
Ask: [one optional follow-up question the candidate could ask, or omit this line]\n\n\
Use ONLY the candidate's documented background. Keep bullets under 12 words each.",
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
        assert!(p.contains("STAR"));
        assert!(p.contains("**S:**"));
    }

    #[test]
    fn non_behavioral_uses_bullet_format() {
        let p = build_user_prompt("What are your strengths?", &[]);
        assert!(!p.contains("**S:**"));
        assert!(p.contains("Tell:"));
    }
}
