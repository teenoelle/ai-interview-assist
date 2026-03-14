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
Affirm: [3-5 word cue acknowledging what they care about — e.g. 'ownership under pressure', 'team resilience']\n\
Say: [3-6 word hook — the outcome or boldest action, e.g. 'led migration, cut costs 40%%']\n\
---\n\
**S:** [situation cue + business impact — e.g. 'legacy system → $2M risk']\n\
**T:** [3-5 word task/ownership cue]\n\
**A:** [3-6 word action cue with one **bold** keyword]\n\
**R:** [3-5 word result cue, include number if possible]\n\
Ask: [3-6 word optional follow-up question cue, or omit]\n\n\
CRITICAL: Every line is a memory cue, NOT a full sentence. 3-6 words max per bullet. Candidate will expand verbally. Use ONLY documented experience.",
            ctx_prefix, question
        )
    } else {
        format!(
            "{}The interviewer asked: '{}'\n\n\
Reply in this EXACT format — no extra text:\n\
Affirm: [3-5 word cue — the pain point or concern behind this question, e.g. 'clarity under ambiguity', 'culture alignment']\n\
Say: [3-6 word opening cue — the strongest hook]\n\
---\n\
• [3-5 word cue — first supporting point]\n\
• [3-5 word cue — second supporting point or example]\n\
• [3-5 word cue — optional third point, omit if weak]\n\
Ask: [3-6 word optional follow-up question cue, or omit]\n\n\
CRITICAL: Every line is a memory cue, NOT a full sentence. 3-6 words max. No verbs that make it a full sentence. Candidate reads the cue and speaks naturally. Use ONLY documented background.",
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
        assert!(p.contains("Affirm:"));
    }

    #[test]
    fn non_behavioral_uses_bullet_cues() {
        let p = build_user_prompt("What are your strengths?", &[]);
        assert!(!p.contains("**S:**"));
        assert!(p.contains("Say:"));
        assert!(p.contains("Affirm:"));
        assert!(p.contains("•"));
    }
}
