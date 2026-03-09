use common::messages::SetupPayload;
use crate::linkedin::InterviewerProfile;

pub fn build_system_prompt(
    payload: &SetupPayload,
    company_info: &str,
    interviewer: &InterviewerProfile,
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
        let company_preview = if company_info.len() > 8000 {
            &company_info[..8000]
        } else {
            company_info
        };
        prompt.push_str(company_preview);
        prompt.push_str("\n\n");
    }

    if !interviewer.name.is_empty() || !interviewer.role.is_empty() {
        prompt.push_str("## Interviewer Profile\n");
        if !interviewer.name.is_empty() {
            prompt.push_str(&format!("Name: {}\n", interviewer.name));
        }
        if !interviewer.role.is_empty() {
            prompt.push_str(&format!("Role: {}\n", interviewer.role));
        }
        if !interviewer.company.is_empty() {
            prompt.push_str(&format!("Company: {}\n", interviewer.company));
        }
        if !interviewer.background.is_empty() && interviewer.background.len() < 2000 {
            prompt.push_str("Background:\n");
            prompt.push_str(&interviewer.background);
        }
        prompt.push_str("\n\n");
    }

    prompt.push_str("## Instructions\n");
    prompt.push_str("When the interviewer asks a question, provide exactly 3 numbered talking points. ");
    prompt.push_str("Each point should be 1-2 sentences, specific to the candidate's experience, and relevant to the job. ");
    prompt.push_str("Reference the company, the interviewer's background, or specific job requirements when relevant.");

    prompt
}
