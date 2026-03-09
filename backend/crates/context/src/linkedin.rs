pub struct InterviewerProfile {
    pub name: String,
    pub role: String,
    pub company: String,
    pub background: String,
    pub interests: String,
}

/// Parse one or more LinkedIn profiles separated by `---INTERVIEWER---`.
/// Returns all profiles; caller uses `profiles[0]` as the primary one and
/// the full list for the system prompt.
pub fn parse_all_linkedin_profiles(text: &str) -> Vec<InterviewerProfile> {
    if text.trim().is_empty() {
        return vec![InterviewerProfile {
            name: String::new(), role: String::new(), company: String::new(),
            background: String::new(), interests: String::new(),
        }];
    }
    text.split("---INTERVIEWER---")
        .map(|block| parse_linkedin_text(block.trim()))
        .collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_name_and_role() {
        let text = "Jane Smith\nSenior Engineer at Acme Corp\nPassionate about Rust and distributed systems";
        let p = parse_linkedin_text(text);
        assert_eq!(p.name, "Jane Smith");
        assert_eq!(p.role, "Senior Engineer at Acme Corp");
        assert_eq!(p.company, "Acme Corp");
    }

    #[test]
    fn parses_company_with_bullet() {
        let text = "John Doe\nEngineering Manager\n· BigTech Inc";
        let p = parse_linkedin_text(text);
        assert_eq!(p.company, "BigTech Inc");
    }

    #[test]
    fn empty_input_returns_empty_profile() {
        let p = parse_linkedin_text("");
        assert!(p.name.is_empty());
        assert!(p.role.is_empty());
        assert!(p.company.is_empty());
    }

    #[test]
    fn multiple_profiles_split_correctly() {
        let text = "Alice\nCTO at Startup\n---INTERVIEWER---\nBob\nVP Engineering at Corp";
        let profiles = parse_all_linkedin_profiles(text);
        assert_eq!(profiles.len(), 2);
        assert_eq!(profiles[0].name, "Alice");
        assert_eq!(profiles[1].name, "Bob");
    }

    #[test]
    fn single_empty_text_returns_one_empty_profile() {
        let profiles = parse_all_linkedin_profiles("");
        assert_eq!(profiles.len(), 1);
        assert!(profiles[0].name.is_empty());
    }
}
