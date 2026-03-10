use common::messages::TranscriptSegment;

pub fn build_user_prompt(question: &str, transcript: &[TranscriptSegment]) -> String {
    let recent: Vec<&TranscriptSegment> = transcript.iter().rev().take(10).collect();
    let context = recent
        .iter()
        .rev()
        .map(|s| s.text.as_str())
        .collect::<Vec<_>>()
        .join(" ... ");

    if context.is_empty() {
        format!(
            "The interviewer just asked: '{}'\n\nProvide 3 talking points using ONLY the candidate's documented background above. Use the scannable format from the instructions. If no relevant experience exists, say so honestly.",
            question
        )
    } else {
        format!(
            "Recent conversation: {}\n\nThe interviewer just asked: '{}'\n\nProvide 3 talking points using ONLY the candidate's documented background above. Use the scannable format from the instructions. If no relevant experience exists, say so honestly.",
            context, question
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
        assert!(p.contains("seg14")); // most recent included
        assert!(!p.contains("seg4")); // oldest 5 excluded
    }
}
