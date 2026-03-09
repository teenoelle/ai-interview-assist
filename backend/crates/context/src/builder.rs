use common::messages::SetupPayload;
use crate::linkedin::InterviewerProfile;

pub fn build_system_prompt(
    payload: &SetupPayload,
    company_info: &str,
    interviewers: &[InterviewerProfile],
) -> String {
    let mut prompt = String::new();

    prompt.push_str("You are an AI interview coach helping a candidate during a real-time job interview. ");
    prompt.push_str("Based on the candidate's background and the job requirements, provide concise, relevant talking points. ");
    prompt.push_str("Keep suggestions brief and practical — 2-3 sentences per point maximum.\n\n");

    if !payload.job_description.is_empty() {
        prompt.push_str("## Job Description\n");
        prompt.push_str(&payload.job_description);
        prompt.push_str("\n\n");
    }

    if !payload.cv_text.is_empty() {
        prompt.push_str("## Candidate CV / Resume\n");
        let cv_preview = if payload.cv_text.len() > 5000 {
            &payload.cv_text[..5000]
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
    prompt.push_str("When the interviewer asks a question, provide exactly 3 numbered talking points. ");
    prompt.push_str("Each point should be 1-2 sentences, specific to the candidate's experience, and relevant to the job. ");
    prompt.push_str("Reference the company, the interviewer's background, or specific job requirements when relevant.");

    prompt
}
