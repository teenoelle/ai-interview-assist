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
            "The interviewer just asked: '{}'\nGive 3 concise talking points (max 2 sentences each) to answer this well based on my background.",
            question
        )
    } else {
        format!(
            "Recent conversation: {}\n\nThe interviewer just asked: '{}'\nGive 3 concise talking points (max 2 sentences each) to answer this well based on my background.",
            context, question
        )
    }
}
