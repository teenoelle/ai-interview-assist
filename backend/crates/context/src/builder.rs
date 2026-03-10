use common::messages::SetupPayload;
use crate::linkedin::InterviewerProfile;

pub fn build_system_prompt(
    payload: &SetupPayload,
    company_info: &str,
    interviewers: &[InterviewerProfile],
) -> String {
    let mut prompt = String::new();

    prompt.push_str("You are an AI interview coach helping a candidate during a real-time job interview. ");
    prompt.push_str("Based on the candidate's background and the job requirements, provide concise, relevant talking points.\n\n");
    prompt.push_str("CRITICAL RULES — follow these exactly:\n");
    prompt.push_str("1. ONLY use information explicitly present in the candidate's CV, extra experience, or the job description provided below. Do NOT invent experiences, projects, metrics, or technologies that are not documented.\n");
    prompt.push_str("2. If the candidate's background does not contain relevant information for the question, say: 'No specific experience documented — suggest asking the interviewer to clarify scope, then pivot to [closest related skill].'\n");
    prompt.push_str("3. Never fabricate numbers, company names, project names, or outcomes. Only cite what is in the provided context.\n\n");

    if !payload.job_description.is_empty() {
        prompt.push_str("## Job Description\n");
        prompt.push_str(&payload.job_description);
        prompt.push_str("\n\n");
    }

    if !payload.cv_text.is_empty() {
        prompt.push_str("## Candidate CV / Resume\n");
        let cv_preview = if payload.cv_text.len() > 20000 {
            &payload.cv_text[..20000]
        } else {
            &payload.cv_text
        };
        prompt.push_str(cv_preview);
        prompt.push_str("\n\n");
    }

    if !payload.extra_experience.is_empty() {
        prompt.push_str("## Additional Experience / Notes\n");
        prompt.push_str(&payload.extra_experience);
        prompt.push_str("\n\n");
    }

    if !company_info.is_empty() {
        prompt.push_str("## Company Information\n");
        let company_preview = if company_info.len() > 15000 {
            &company_info[..15000]
        } else {
            company_info
        };
        prompt.push_str(company_preview);
        prompt.push_str("\n\n");
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
                let bg = if p.background.len() > 1500 { &p.background[..1500] } else { &p.background };
                prompt.push_str("Background:\n");
                prompt.push_str(bg);
                prompt.push('\n');
            }
            prompt.push('\n');
        }
    }

    prompt.push_str("## Instructions\n");
    prompt.push_str("When the interviewer asks a question, provide exactly 3 talking points in this scannable format:\n\n");
    prompt.push_str("**1. [2-4 WORD KEYWORD]** — one sentence grounded in the candidate's actual documented experience.\n");
    prompt.push_str("**2. [2-4 WORD KEYWORD]** — one sentence grounded in the candidate's actual documented experience.\n");
    prompt.push_str("**3. [2-4 WORD KEYWORD]** — one sentence grounded in the candidate's actual documented experience.\n\n");
    prompt.push_str("The KEYWORD is a memory trigger the candidate glances at — make it a vivid 2-4 word phrase (e.g. 'Led 3 migrations', 'Reduced costs 40%', 'Python + Kubernetes'). ");
    prompt.push_str("The supporting sentence must reference specific facts from their CV. If no direct experience exists, say so honestly rather than inventing details.");

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
    fn prompt_contains_instructions() {
        let p = build_system_prompt(&empty_payload(), "", &[]);
        assert!(p.contains("## Instructions"));
        assert!(p.contains("3 talking points"));
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
