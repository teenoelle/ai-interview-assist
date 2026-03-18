export interface CompanyBrief {
  name: string;
  what_they_do: string;
  products: string[];
  culture: string;
  recent_news: string;
  why_join: string;
}

export interface InterviewerSummary {
  name: string;
  role: string;
  background: string;
  tenure: string;
  rapport_tips: string[];
}

export interface SetupResponse {
  success: boolean;
  system_prompt_preview: string;
  message: string;
  predicted_questions: string[];
  company_brief?: CompanyBrief;
  interviewer_summaries?: InterviewerSummary[];
  jd_keywords?: string[];
}

export async function submitSetup(formData: FormData): Promise<SetupResponse> {
  const resp = await fetch('/api/setup/finalize', {
    method: 'POST',
    body: formData,
  });
  if (!resp.ok) {
    const text = await resp.text();
    throw new Error(`Setup failed: ${resp.status} ${text}`);
  }
  return resp.json();
}

export async function fetchUsage(): Promise<Record<string, number>> {
  try {
    const resp = await fetch('/api/usage');
    if (resp.ok) return resp.json();
  } catch {}
  return {};
}
