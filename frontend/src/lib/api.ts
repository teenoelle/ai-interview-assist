function authHeaders(): Record<string, string> {
  try {
    const token = localStorage.getItem('app-token');
    return token ? { 'Authorization': `Bearer ${token}` } : {};
  } catch { return {}; }
}

export function authFetch(input: RequestInfo, init?: RequestInit): Promise<Response> {
  const headers = { ...authHeaders(), ...(init?.headers as Record<string, string> ?? {}) };
  return fetch(input, { ...init, headers });
}

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
  const resp = await authFetch('/api/setup/finalize', {
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
    const resp = await authFetch('/api/usage');
    if (resp.ok) return resp.json();
  } catch {}
  return {};
}
