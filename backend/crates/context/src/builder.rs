use common::messages::SetupPayload;
use crate::linkedin::InterviewerProfile;

/// Detect the employer's company type from the job description and company info.
/// Returns a static string used to inject a Company Type block into the system prompt.
fn detect_company_type(jd: &str, company_info: &str) -> Option<&'static str> {
    let combined = format!("{} {}", jd, company_info).to_lowercase();
    // Strong agency/consultancy signals
    let agency_keywords: &[&str] = &[
        "agency", "agencies", "client accounts", "client work", "client base",
        "our clients", "for clients", "client relationships", "client brands",
        "managed services", "consultancy", "consulting firm", "media buying",
        "account management", "account manager", "client services",
        "advertising agency", "digital agency", "marketing agency",
        "media agency", "pr agency", "creative agency",
    ];
    let agency_score = agency_keywords.iter().filter(|&&kw| combined.contains(kw)).count();
    // A single strong keyword (e.g. "agency") or two weaker signals is enough
    if agency_score >= 1 {
        return Some("agency");
    }
    None
}

fn trunc(s: &str, chars: usize) -> &str {
    match s.char_indices().nth(chars) {
        Some((i, _)) => &s[..i],
        None => s,
    }
}

pub fn build_system_prompt(
    payload: &SetupPayload,
    company_info: &str,
    interviewers: &[InterviewerProfile],
) -> String {
    let mut prompt = String::new();

    prompt.push_str("You are an AI interview coach helping a candidate during a real-time job interview. ");
    prompt.push_str("Based on the candidate's background and the job requirements, provide concise, relevant talking points.\n\n");
    prompt.push_str("CRITICAL RULES — follow these exactly:\n");
    prompt.push_str("1. ONLY use information explicitly present in the candidate's CV, LinkedIn profile, extra experience notes, or the job description provided below. Do NOT invent experiences, projects, metrics, or technologies that are not documented.\n");
    prompt.push_str("2. If the candidate's background does not contain relevant information for the question, say: 'No specific experience documented — suggest asking the interviewer to clarify scope, then pivot to [closest related skill].'\n");
    prompt.push_str("3. Never fabricate numbers, company names, project names, or outcomes. Only cite what is in the provided context.\n\n");

    if !payload.job_description.is_empty() {
        prompt.push_str("## Job Description\n");
        prompt.push_str(&payload.job_description);
        prompt.push_str("\n\n");
    }

    if !payload.cv_text.is_empty() {
        prompt.push_str("## Candidate CV / Resume\n");
        let cv_preview = trunc(&payload.cv_text, 20000);
        prompt.push_str(cv_preview);
        prompt.push_str("\n\n");
    }

    if !payload.interviewee_linkedin.is_empty() {
        prompt.push_str("## Candidate LinkedIn Profile\n");
        let li_preview = trunc(&payload.interviewee_linkedin, 10000);
        prompt.push_str(li_preview);
        prompt.push_str("\n\n");
    }

    if !payload.portfolio_text.is_empty() {
        prompt.push_str("## Candidate Portfolio / Personal Website\n");
        let portfolio_preview = trunc(&payload.portfolio_text, 5000);
        prompt.push_str(portfolio_preview);
        prompt.push_str("\n\n");
    }

    if !payload.extra_experience.is_empty() {
        prompt.push_str("## Early Career & Additional Context\n");
        prompt.push_str("This section contains early career history, pre-CV roles, volunteering, education context, or other background the candidate wants to draw on. IMPORTANT: this section also contains the candidate's explicitly stated preferences — what they look for in a company, manager, team, or role (e.g. culture, growth, management style, values). Use these stated preferences directly when answering values/preferences questions.\n");
        prompt.push_str(&payload.extra_experience);
        prompt.push_str("\n\n");
    }

    if !company_info.is_empty() {
        prompt.push_str("## Company Information\n");
        let company_preview = trunc(company_info, 15000);
        prompt.push_str(company_preview);
        prompt.push_str("\n\n");
    }

    // Inject company type based on JD + company info so the LLM never makes wrong assumptions
    match detect_company_type(&payload.job_description, company_info) {
        Some("agency") => {
            prompt.push_str("## Company Type\n");
            prompt.push_str("This employer is an AGENCY or client-services firm that manages work ACROSS MULTIPLE CLIENTS. ");
            prompt.push_str("Frame ALL answers in terms of client account work — delivering strategy, results, and relationships across a portfolio of clients. ");
            prompt.push_str("NEVER suggest the candidate is moving from agency to in-house. NEVER say they want to own one company's strategy long-term. ");
            prompt.push_str("The candidate is applying to continue and grow in client-facing agency work.\n\n");
        }
        _ => {}
    }

    let non_empty: Vec<&InterviewerProfile> = interviewers
        .iter()
        .filter(|p| !p.name.is_empty() || !p.role.is_empty() || !p.background.is_empty())
        .collect();

    if !non_empty.is_empty() {
        if non_empty.len() > 1 {
            prompt.push_str(&format!("## Interviewers ({} people)\n", non_empty.len()));
        } else {
            prompt.push_str("## Interviewer Profile\n");
        }
        for (i, p) in non_empty.iter().enumerate() {
            if non_empty.len() > 1 {
                prompt.push_str(&format!("### Interviewer {}\n", i + 1));
            }
            if !p.name.is_empty() { prompt.push_str(&format!("Name: {}\n", p.name)); }
            if !p.role.is_empty() { prompt.push_str(&format!("Role: {}\n", p.role)); }
            if !p.company.is_empty() { prompt.push_str(&format!("Company: {}\n", p.company)); }
            if !p.background.is_empty() {
                let bg = trunc(&p.background, 1500);
                prompt.push_str("Background:\n");
                prompt.push_str(bg);
                prompt.push('\n');
            }
            prompt.push('\n');
        }
    }

    prompt.push_str("## Context Rules\n");
    prompt.push_str("Draw ONLY from the candidate background, CV, LinkedIn, portfolio, and job description provided above. ");
    prompt.push_str("Never invent experiences, metrics, company names, or outcomes not present in the provided context. ");
    prompt.push_str("Follow the output format specified in each user message exactly — do not add extra sections or change the structure.");

    prompt
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::messages::SetupPayload;

    fn empty_payload() -> SetupPayload {
        SetupPayload::default()
    }

    #[test]
    fn prompt_contains_context_rules() {
        let p = build_system_prompt(&empty_payload(), "", &[]);
        assert!(p.contains("## Context Rules"));
        assert!(p.contains("Draw ONLY from the candidate background"));
    }

    #[test]
    fn prompt_includes_job_description() {
        let mut payload = empty_payload();
        payload.job_description = "Rust backend engineer".to_string();
        let p = build_system_prompt(&payload, "", &[]);
        assert!(p.contains("## Job Description"));
        assert!(p.contains("Rust backend engineer"));
    }

    #[test]
    fn prompt_includes_company_info() {
        let p = build_system_prompt(&empty_payload(), "Acme builds widgets", &[]);
        assert!(p.contains("## Company Information"));
        assert!(p.contains("Acme builds widgets"));
    }

    #[test]
    fn prompt_includes_single_interviewer() {
        let interviewer = InterviewerProfile {
            name: "Jane Smith".to_string(),
            role: "CTO".to_string(),
            company: "Acme".to_string(),
            background: "20 years in systems".to_string(),
            interests: String::new(),
        };
        let p = build_system_prompt(&empty_payload(), "", &[interviewer]);
        assert!(p.contains("## Interviewer Profile"));
        assert!(p.contains("Jane Smith"));
    }

    #[test]
    fn prompt_numbers_multiple_interviewers() {
        let make = |name: &str| InterviewerProfile {
            name: name.to_string(),
            role: String::new(),
            company: String::new(),
            background: "x".to_string(),
            interests: String::new(),
        };
        let p = build_system_prompt(&empty_payload(), "", &[make("Alice"), make("Bob")]);
        assert!(p.contains("## Interviewers (2 people)"));
        assert!(p.contains("### Interviewer 1"));
        assert!(p.contains("### Interviewer 2"));
    }

    #[test]
    fn cv_truncated_at_limit() {
        let mut payload = empty_payload();
        payload.cv_text = "z".repeat(25000);
        let p = build_system_prompt(&payload, "", &[]);
        // The prompt should contain 20000 z's but not 25000
        let count = p.matches('z').count();
        assert_eq!(count, 20000);
    }
}
