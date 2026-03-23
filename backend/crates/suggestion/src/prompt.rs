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
IMPORTANT: Output ONLY the exact labeled lines below. No preamble, no intro, nothing extra.\n\n\
Acknowledge: <One complete grammatical sentence ending with a period. Name the high-level business goal or bottom-line pain point behind this question — do NOT restate the question. Open with one of: 'Growth here depends on', 'The core challenge is', 'Profitability here depends on', 'Scaling here requires'. Then complete the sentence with a noun phrase naming the business outcome at stake. Max 18 words. Never say 'our' or 'I'.>\n\
Answer: <The spoken answer on this same line. Short sentences starting with 'I'. Max 10 words per sentence. Use as many sentences as needed — do not sacrifice specificity for brevity. Directly answer the question with strategy or approach — NO specific stories or examples (those go in Example cues). No adjectives. No adverbs. No 'utilize' — use 'use'. Facts and actions only.>\n\
---\n\
General: [General Answer] <keyword phrase: 2-3 word approach. Max 6 words total. No metrics. e.g. 'keyword research: intent-first'>\n\
Example: [Example] <keyword phrase: outcome. Include a metric ONLY if it appears explicitly in the candidate background — never invent one. Max 6 words total. e.g. 'difficult client: retained account'>\n\
Ask: <keyword phrase — no question words> | <Strategic follow-up. 1-2 sentences max 10 words each. Ends with '?'.>\n\
Ask: <different keyword phrase — no question words> | <Strategic follow-up. 1-2 sentences max 10 words each. Ends with '?'.>\n\n\
Rules:\n\
- Output ONLY these lines. No extra text.\n\
- Acknowledge: one complete grammatical sentence ending with a period. Names the business pain point — never starts with 'I'.\n\
- Answer text must be on the same line as 'Answer:' — not on a new line.\n\
- Answer: strategy and approach only. No specific stories, anecdotes, or named examples — those belong in Example cues.\n\
- Answer: no adjectives or adverbs. No 'utilize'. Facts and actions only.\n\
- NEVER invent metrics, percentages, or numbers. Only use figures explicitly stated in the candidate background. If an outcome improved but the magnitude is unknown, describe the direction only (e.g. 'improved conversion') — never fabricate a number.\n\
- General and Example hints: max 6 words total (keyword phrase + 2-3 word result/approach).\n\
- Only add a second General or Example if it addresses a genuinely DIFFERENT part of the question or a different story. No repeating the same point in different words.\n\
- Keywords are multi-word phrases from the question (e.g. 'difficult conversation', 'conflicting priorities', 'client relationships').\n\
- Ask topic: keyword phrase — NO question words (no 'how', 'what', 'when', 'why').\n\
- Ask sentence: short, specific, grammatical, ends with '?'. No adjectives or adverbs.\n\
- Ask lines come AFTER the --- separator only.\n\
- NEVER name specific clients, employers, or companies. Refer to them by industry only (e.g. 'retail brand', 'tech startup', 'financial services firm').\n\
- Read the system prompt carefully to understand the employer's business model. If the employer is an agency, consultancy, or services firm that works with multiple clients, frame all answers in terms of client work across accounts — NEVER describe it as owning one company's strategy long-term.\n\
- Use only background provided. No invented details.",
            ctx_prefix, question
        )
    } else {
        format!(
            "{}The interviewer asked: '{}'\n\n\
IMPORTANT: Output ONLY the exact labeled lines below. No preamble, no intro, nothing extra.\n\n\
Acknowledge: <One complete grammatical sentence ending with a period. Name the high-level business goal or bottom-line pain point behind this question — do NOT restate the question. Open with one of: 'Growth here depends on', 'The core challenge is', 'Profitability here depends on', 'Scaling here requires'. Then complete the sentence with a noun phrase naming the business outcome at stake. Max 18 words. Never say 'our' or 'I'.>\n\
Answer: <The spoken answer on this same line. Short sentences starting with 'I'. Max 10 words per sentence. Use as many sentences as needed — do not sacrifice specificity for brevity. Directly answer the question with strategy or approach — NO specific stories or examples (those go in Example cues). No adjectives. No adverbs. No 'utilize' — use 'use'. Facts and actions only.>\n\
---\n\
General: [General Answer] <keyword phrase: 2-3 word approach. Max 6 words total. No metrics. e.g. 'keyword research: intent-first'>\n\
General: [General Answer] <different keyword phrase: 2-3 word approach. Max 6 words total. No metrics.>\n\
Example: [Example] <keyword phrase: outcome. Include a metric ONLY if it appears explicitly in the candidate background — never invent one. Max 6 words total. e.g. 'ad copywriting: lower CPA'>\n\
Ask: <keyword phrase — no question words> | <Strategic follow-up. 1-2 sentences max 10 words each. Ends with '?'.>\n\
Ask: <different keyword phrase — no question words> | <Strategic follow-up. 1-2 sentences max 10 words each. Ends with '?'.>\n\n\
Rules:\n\
- Output ONLY these lines. No extra text.\n\
- Acknowledge: one complete grammatical sentence ending with a period. Names the business pain point — never starts with 'I'.\n\
- Answer text must be on the same line as 'Answer:' — not on a new line.\n\
- Answer: strategy and approach only. No specific stories, anecdotes, or named examples — those belong in Example cues.\n\
- Answer: no adjectives or adverbs. No 'utilize'. Facts and actions only.\n\
- NEVER invent metrics, percentages, or numbers. Only use figures explicitly stated in the candidate background. If an outcome improved but the magnitude is unknown, describe the direction only (e.g. 'improved conversion') — never fabricate a number.\n\
- General and Example hints: max 6 words total (keyword phrase + 2-3 word result/approach).\n\
- Only add a second General or Example if it addresses a genuinely DIFFERENT part of the question or a different story. No repeating the same point in different words.\n\
- Keywords are multi-word phrases from the question (e.g. 'keyword research', 'client relationships', 'ad copywriting').\n\
- Ask topic: keyword phrase — NO question words (no 'how', 'what', 'when', 'why').\n\
- Ask sentence: short, specific, grammatical, ends with '?'. No adjectives or adverbs.\n\
- Ask lines come AFTER the --- separator only.\n\
- NEVER name specific clients, employers, or companies. Refer to them by industry only (e.g. 'retail brand', 'tech startup', 'financial services firm').\n\
- Use only background provided. No invented details.",
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
    fn behavioral_question_uses_example_format() {
        let p = build_user_prompt("Tell me about a time you led a team", &[]);
        assert!(p.contains("Example:"));
        assert!(p.contains("General:"));
        assert!(p.contains("Acknowledge:"));
        assert!(p.contains("Answer:"));
        assert!(!p.contains("Affirm:"));
        assert!(!p.contains("Story:"));
    }

    #[test]
    fn non_behavioral_uses_general_example_format() {
        let p = build_user_prompt("What are your strengths?", &[]);
        assert!(p.contains("Answer:"));
        assert!(p.contains("Acknowledge:"));
        assert!(p.contains("General:"));
        assert!(p.contains("Example:"));
        assert!(!p.contains("Affirm:"));
    }
}
