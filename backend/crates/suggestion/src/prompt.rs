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
            "{}The interviewer asked a behavioral question: '{}'\n\nProvide a STAR-format answer using ONLY the candidate's documented experience:\n\n**S — Situation:** [set the scene in 1 sentence]\n**T — Task:** [your specific responsibility in 1 sentence]\n**A — Action:** [exactly what you did — 1-2 sentences]\n**R — Result:** [measurable outcome in 1 sentence]\n\nOnly include facts from the candidate's actual documented background. If no matching experience exists, say so clearly.",
            ctx_prefix, question
        )
    } else {
        format!(
            "{}The interviewer just asked: '{}'\n\nProvide 3 talking points using ONLY the candidate's documented background above. Use the scannable format from the instructions. If no relevant experience exists, say so honestly.",
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
        assert!(p.contains("**S —"));
    }

    #[test]
    fn non_behavioral_uses_bullet_format() {
        let p = build_user_prompt("What are your strengths?", &[]);
        assert!(!p.contains("**S —"));
        assert!(p.contains("talking points"));
    }
}
