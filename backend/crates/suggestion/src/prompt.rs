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
Reply in this EXACT format. No extra text. No markdown. No hashtags.\n\
Acknowledge: [max 8 words — empathize with their specific pain point, e.g. 'That kind of instability can really slow teams down.']\n\
Affirm: [max 10 words — show personal alignment, e.g. 'Reliability at scale is exactly what I have built for.']\n\
Answer: [max 12 words — boldest outcome first, e.g. 'I cut infrastructure costs by 40%% with zero downtime.']\n\
---\n\
Context: [3-5 word cue]\n\
Action: [3-5 word cue]\n\
Result: [3-5 word cue]\n\
Ask: [full spoken question 8-14 words] | [alternative phrasing] | [casual version]\n\
Ask: [full spoken question 8-14 words] | [alternative phrasing] | [casual version]\n\n\
RULES: Acknowledge empathizes with their concern. Affirm bridges to your answer. Answer is the boldest outcome, strictly under 12 words. Cues are 3-5 word fragments only. Each Ask is a FULL SENTENCE followed by pipe-separated alternatives. Use ONLY real documented experience. Output nothing else.",
            ctx_prefix, question
        )
    } else {
        format!(
            "{}The interviewer asked: '{}'\n\n\
Reply in this EXACT format. No extra text. No markdown. No hashtags.\n\
Acknowledge: [max 8 words — empathize with their specific pain point, e.g. 'That is exactly the kind of problem worth solving.']\n\
Affirm: [max 10 words — show personal alignment, e.g. 'This is an area I have invested deeply in.']\n\
Answer: [max 12 words — strongest direct answer, e.g. 'I turn complex data into decisions people can act on.']\n\
---\n\
Point: [3-5 word cue — first supporting point]\n\
Point: [3-5 word cue — second supporting point]\n\
Ask: [full spoken question 8-14 words] | [alternative phrasing] | [casual version]\n\
Ask: [full spoken question 8-14 words] | [alternative phrasing] | [casual version]\n\n\
RULES: Acknowledge empathizes with their concern. Affirm bridges to your answer. Answer is the strongest reply, strictly under 12 words. Cues are 3-5 word fragments only. Each Ask is a FULL SENTENCE followed by pipe-separated alternatives. Use ONLY real documented background. Output nothing else.",
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
        assert!(p.contains("Context:"));
        assert!(p.contains("Result:"));
        assert!(p.contains("Acknowledge:"));
        assert!(p.contains("Affirm:"));
        assert!(p.contains("Answer:"));
    }

    #[test]
    fn non_behavioral_uses_bullet_cues() {
        let p = build_user_prompt("What are your strengths?", &[]);
        assert!(p.contains("Answer:"));
        assert!(p.contains("Affirm:"));
        assert!(p.contains("Acknowledge:"));
        assert!(p.contains("Point:"));
    }
}
