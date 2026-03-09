<script lang="ts">
  import { submitSetup } from '../lib/api';

  let jobDescription = $state('');
  let companyUrl = $state('');
  let linkedinText = $state('');
  let extraExperience = $state('');
  let cvFile: File | null = $state(null);
  let loading = $state(false);
  let error = $state('');
  let systemPromptPreview = $state('');

  const { onSetupComplete } = $props<{ onSetupComplete: () => void }>();

  async function handleSubmit() {
    loading = true;
    error = '';
    try {
      const formData = new FormData();
      formData.append('job_description', jobDescription);
      formData.append('company_url', companyUrl);
      formData.append('linkedin_text', linkedinText);
      formData.append('extra_experience', extraExperience);
      if (cvFile) formData.append('cv_file', cvFile);

      const result = await submitSetup(formData);
      systemPromptPreview = result.system_prompt_preview;
      onSetupComplete();
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  function handleFileChange(e: Event) {
    const target = e.target as HTMLInputElement;
    cvFile = target.files?.[0] ?? null;
  }
</script>

<div class="setup-form">
  <h2>Interview Setup</h2>
  <p class="subtitle">Fill in your context before the interview begins</p>

  {#if error}
    <div class="error">{error}</div>
  {/if}

  <div class="field">
    <label for="job-desc">Job Description</label>
    <textarea
      id="job-desc"
      bind:value={jobDescription}
      rows={6}
      placeholder="Paste the full job description here..."
    ></textarea>
  </div>

  <div class="field">
    <label for="company-url">Company Website URL</label>
    <input
      id="company-url"
      type="url"
      bind:value={companyUrl}
      placeholder="https://company.com"
    />
    <small>We'll crawl up to 30 pages to learn about the company</small>
  </div>

  <div class="field">
    <label for="cv-file">Upload CV / Resume</label>
    <input id="cv-file" type="file" accept=".pdf,.txt" onchange={handleFileChange} />
  </div>

  <div class="field">
    <label for="linkedin">Interviewer LinkedIn Profile (paste text)</label>
    <textarea
      id="linkedin"
      bind:value={linkedinText}
      rows={4}
      placeholder="Paste the interviewer's LinkedIn profile text here..."
    ></textarea>
  </div>

  <div class="field">
    <label for="extra">Additional Experience / Notes</label>
    <textarea
      id="extra"
      bind:value={extraExperience}
      rows={4}
      placeholder="Add any extra context, achievements, or talking points..."
    ></textarea>
  </div>

  {#if systemPromptPreview}
    <details class="preview">
      <summary>System prompt preview</summary>
      <pre>{systemPromptPreview}</pre>
    </details>
  {/if}

  <button onclick={handleSubmit} disabled={loading} class="btn-primary">
    {loading ? 'Processing...' : 'Start Session'}
  </button>
</div>

<style>
  .setup-form {
    max-width: 720px;
    margin: 0 auto;
    padding: 2rem;
  }
  h2 {
    font-size: 1.75rem;
    margin-bottom: 0.5rem;
    color: #f1f5f9;
  }
  .subtitle {
    color: #94a3b8;
    margin-bottom: 2rem;
  }
  .field {
    margin-bottom: 1.5rem;
  }
  label {
    display: block;
    margin-bottom: 0.5rem;
    font-weight: 500;
    color: #cbd5e1;
  }
  textarea,
  input[type='url'],
  input[type='text'] {
    width: 100%;
    padding: 0.75rem;
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 0.5rem;
    color: #e2e8f0;
    font-size: 0.9rem;
    resize: vertical;
  }
  input[type='file'] {
    color: #94a3b8;
  }
  small {
    display: block;
    margin-top: 0.25rem;
    color: #64748b;
    font-size: 0.8rem;
  }
  .error {
    padding: 1rem;
    background: #450a0a;
    border: 1px solid #7f1d1d;
    border-radius: 0.5rem;
    color: #fca5a5;
    margin-bottom: 1rem;
  }
  .preview {
    margin-bottom: 1.5rem;
  }
  .preview summary {
    cursor: pointer;
    color: #60a5fa;
  }
  .preview pre {
    margin-top: 0.5rem;
    padding: 1rem;
    background: #1e293b;
    border-radius: 0.5rem;
    white-space: pre-wrap;
    font-size: 0.75rem;
    color: #94a3b8;
    max-height: 200px;
    overflow: auto;
  }
  .btn-primary {
    padding: 0.75rem 2rem;
    background: #3b82f6;
    color: white;
    border: none;
    border-radius: 0.5rem;
    font-size: 1rem;
    cursor: pointer;
    font-weight: 600;
    transition: background 0.2s;
  }
  .btn-primary:hover:not(:disabled) {
    background: #2563eb;
  }
  .btn-primary:disabled {
    background: #1e3a5f;
    cursor: not-allowed;
  }
</style>
