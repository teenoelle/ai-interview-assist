pub fn is_question(text: &str) -> bool {
    let text = text.trim();
    if text.ends_with('?') {
        return true;
    }
    let lower = text.to_lowercase();
    let question_starters = [
        "what", "why", "how", "when", "where", "who", "which",
        "can you", "could you", "would you", "tell me", "describe", "explain",
        "have you", "do you", "did you", "are you", "were you",
        "walk me", "give me", "share",
    ];
    let words: Vec<&str> = lower.split_whitespace().collect();
    if words.is_empty() {
        return false;
    }
    for q in &question_starters {
        let q_words: Vec<&str> = q.split_whitespace().collect();
        if words.len() >= q_words.len() {
            let matches = q_words.iter().zip(words.iter()).all(|(a, b)| a == b);
            if matches {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_question_detection() {
        assert!(is_question("What is your experience with Rust?"));
        assert!(is_question("Tell me about yourself"));
        assert!(is_question("Can you describe a challenge you faced?"));
        assert!(!is_question("I see, that's interesting."));
        assert!(!is_question("Thank you for your answer."));
    }
}
