pub struct InterviewerProfile {
    pub name: String,
    pub role: String,
    pub company: String,
    pub background: String,
    pub interests: String,
}

pub fn parse_linkedin_text(text: &str) -> InterviewerProfile {
    if text.trim().is_empty() {
        return InterviewerProfile {
            name: String::new(),
            role: String::new(),
            company: String::new(),
            background: String::new(),
            interests: String::new(),
        };
    }

    let lines: Vec<&str> = text.lines().collect();

    // Heuristic: first non-empty line is usually the name
    let name = lines
        .iter()
        .find(|l| !l.trim().is_empty())
        .map(|l| l.trim().to_string())
        .unwrap_or_default();

    // Second non-empty line is usually the role/headline
    let role = lines
        .iter()
        .filter(|l| !l.trim().is_empty())
        .nth(1)
        .map(|l| l.trim().to_string())
        .unwrap_or_default();

    // Company: look for "at <Company>" or "· <Company>"
    let company = lines
        .iter()
        .find_map(|l| {
            let l = l.trim();
            if let Some(idx) = l.find(" at ") {
                Some(l[idx + 4..].trim().to_string())
            } else if l.starts_with('·') {
                Some(l.trim_start_matches('·').trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_default();

    InterviewerProfile {
        name,
        role,
        company,
        background: text.to_string(),
        interests: String::new(),
    }
}
