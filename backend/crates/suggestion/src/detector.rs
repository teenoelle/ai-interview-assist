pub fn is_question(text: &str) -> bool {
    let text = text.trim();
    // Filter preamble lines like "Here are 8 interview questions..."
    let lower_start = text.to_lowercase();
    if lower_start.starts_with("here are")
        || lower_start.starts_with("here is")
        || lower_start.starts_with("the following")
        || lower_start.starts_with("these are")
        || lower_start.starts_with("below are")
    {
        return false;
    }
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

    #[test]
    fn ends_with_question_mark() {
        assert!(is_question("So you've worked with distributed systems?"));
        assert!(is_question("Really?"));
    }

    #[test]
    fn question_starters() {
        assert!(is_question("Why did you leave your last role"));
        assert!(is_question("How do you handle conflict"));
        assert!(is_question("Where do you see yourself in five years"));
        assert!(is_question("Who was your most influential mentor"));
        assert!(is_question("Which approach would you prefer"));
        assert!(is_question("When did you first start coding"));
        assert!(is_question("Have you worked with Kubernetes before"));
        assert!(is_question("Do you have experience leading teams"));
        assert!(is_question("Did you ever manage a budget"));
        assert!(is_question("Are you comfortable with remote work"));
        assert!(is_question("Were you the tech lead on that project"));
        assert!(is_question("Walk me through your background"));
        assert!(is_question("Give me an example of a time you failed"));
        assert!(is_question("Share a challenging project you worked on"));
        assert!(is_question("Describe your ideal team culture"));
        assert!(is_question("Explain how you approach debugging"));
        assert!(is_question("Could you elaborate on that"));
        assert!(is_question("Would you be open to relocation"));
    }

    #[test]
    fn non_questions() {
        assert!(!is_question("That's a great point."));
        assert!(!is_question("I see."));
        assert!(!is_question("Interesting background."));
        assert!(!is_question("Let's move on to the next topic."));
        assert!(!is_question(""));
    }
}
