export interface SetupResponse {
  success: boolean;
  system_prompt_preview: string;
  message: string;
  predicted_questions: string[];
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
