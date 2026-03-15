<script lang="ts">
  import { submitSetup, type CompanyBrief, type InterviewerSummary } from '../lib/api';
  import { saveKeywords } from '../lib/keywordTracker';
  import CompanyBriefPanel from './CompanyBriefPanel.svelte';
  import InterviewerProfilePanel from './InterviewerProfilePanel.svelte';
  import StoryBankPanel from './StoryBankPanel.svelte';

  let jobDescription = $state('');
  let companyUrl = $state('');
  let interviewers = $state<string[]>(['']);   // start with one empty slot
  let intervieweeLinkedin = $state('');
  let extraExperience = $state('');
  let cvFile: File | null = $state(null);
  let extraFile: File | null = $state(null);
  let loading = $state(false);
  let loadingStep = $state('');
  let error = $state('');
  let formEl: HTMLDivElement | undefined = $state();
  let systemPromptPreview = $state('');
  let predictedQuestions = $state<string[]>([]);
  let companyBrief = $state<CompanyBrief | null>(null);
  let interviewerSummaries = $state<InterviewerSummary[]>([]);
  let jdKeywords = $state<string[]>([]);
  let setupDone = $state(false);
  let activeTab = $state<'overview' | 'stories'>('overview');

  const props = $props<{
    onSetupComplete: (data?: { companyBrief?: any; interviewerSummaries?: any[]; jdKeywords?: string[] }) => void;
    onPractice: (questions: string[]) => void;
  }>();

  function startInterview() {
    try {
      props.onSetupComplete({ companyBrief, interviewerSummaries, jdKeywords });
    } catch (e) {
      error = 'Error starting interview: ' + String(e);
    }
  }

  function startPractice() {
    try {
      props.onPractice(predictedQuestions);
    } catch (e) {
      error = 'Error starting practice: ' + String(e);
    }
  }

  function addInterviewer() {
    interviewers = [...interviewers, ''];
  }

  function removeInterviewer(i: number) {
    interviewers = interviewers.filter((_, idx) => idx !== i);
    if (interviewers.length === 0) interviewers = [''];
  }

  function updateInterviewer(i: number, val: string) {
    interviewers = interviewers.map((v, idx) => idx === i ? val : v);
  }

  async function handleSubmit() {
    loading = true;
    error = '';
    loadingStep = companyUrl.trim() ? 'Crawling company website…' : 'Analysing your background…';
    try {
      const formData = new FormData();
      formData.append('job_description', jobDescription);
      formData.append('company_url', companyUrl);

      // Join multiple interviewer profiles with a clear separator
      const linkedinText = interviewers
        .filter(t => t.trim().length > 0)
        .join('\n\n---INTERVIEWER---\n\n');
      formData.append('linkedin_text', linkedinText);
      formData.append('interviewee_linkedin', intervieweeLinkedin);

      formData.append('extra_experience', extraExperience);
      if (cvFile) formData.append('cv_file', cvFile);
      if (extraFile) formData.append('extra_file', extraFile);

      loadingStep = 'Generating your coaching profile…';
      const result = await submitSetup(formData);
      systemPromptPreview = result.system_prompt_preview;
      predictedQuestions = result.predicted_questions ?? [];
      companyBrief = result.company_brief ?? null;
      interviewerSummaries = result.interviewer_summaries ?? [];
      jdKeywords = result.jd_keywords ?? [];
      if (jdKeywords.length > 0) saveKeywords(jdKeywords);
      setupDone = true;
    } catch (e) {
      error = String(e);
      formEl?.scrollIntoView({ behavior: 'smooth', block: 'start' });
    } finally {
      loading = false;
      loadingStep = '';
    }
  }

  function handleFileChange(e: Event) {
    const target = e.target as HTMLInputElement;
    cvFile = target.files?.[0] ?? null;
  }

  function handleExtraFileChange(e: Event) {
    const target = e.target as HTMLInputElement;
    extraFile = target.files?.[0] ?? null;
  }
</script>

<div class="setup-form" bind:this={formEl}>
  <h2>Interview Setup</h2>
  <p class="subtitle">Fill in your context before the interview begins</p>

  {#if error}
    <div class="error">{error}</div>
  {/if}

  {#if setupDone}
    <div class="post-setup">
      <div class="setup-success">
        ✓ Setup complete — review your brief below, then start the interview
      </div>

      <!-- Tab bar -->
      <div class="tab-bar">
        <button class="tab" class:tab-active={activeTab === 'overview'} onclick={() => activeTab = 'overview'}>Overview</button>
        <button class="tab" class:tab-active={activeTab === 'stories'} onclick={() => activeTab = 'stories'}>Story Bank</button>
      </div>

      {#if activeTab === 'overview'}
        {#if companyBrief}
          <CompanyBriefPanel brief={companyBrief} />
        {/if}

        {#if interviewerSummaries.length > 0}
          <div class="section-block">
            <div class="section-block-label">Interviewer Profiles</div>
            <InterviewerProfilePanel interviewers={interviewerSummaries} />
          </div>
        {/if}

        {#if jdKeywords.length > 0}
          <div class="section-block">
            <div class="section-block-label">Keywords to mention</div>
            <div class="keyword-chips">
              {#each jdKeywords as kw}
                <span class="kw-chip">{kw}</span>
              {/each}
            </div>
          </div>
        {/if}

        {#if predictedQuestions.length > 0}
          <div class="predicted">
            <h3>Predicted Interview Questions</h3>
            <ol class="questions-list">
              {#each predictedQuestions as q}
                <li>{q}</li>
              {/each}
            </ol>
          </div>
        {/if}

        {#if systemPromptPreview}
          <details class="preview">
            <summary>System prompt preview</summary>
            <pre>{systemPromptPreview}</pre>
          </details>
        {/if}
      {:else}
        <StoryBankPanel mode="setup" />
      {/if}

      <div class="action-row">
        <button onclick={startInterview} class="btn-primary">Start Interview →</button>
        {#if predictedQuestions.length > 0}
          <button onclick={startPractice} class="btn-secondary">Practice First</button>
        {/if}
      </div>
    </div>
  {:else}
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
      <small>We'll crawl up to 50 pages, prioritising mission, products, team, careers, blog, news, case studies, and investor pages</small>
    </div>

    <div class="field">
      <div class="field-header">
        <span class="field-label">Interviewer LinkedIn Profile(s)</span>
        <button type="button" class="btn-add" onclick={addInterviewer}>+ Add interviewer</button>
      </div>
      {#each interviewers as text, i (i)}
        <div class="interviewer-entry">
          {#if interviewers.length > 1}
            <div class="interviewer-label">Interviewer {i + 1}</div>
          {/if}
          <div class="interviewer-row">
            <textarea
              rows={4}
              value={text}
              oninput={(e) => updateInterviewer(i, (e.target as HTMLTextAreaElement).value)}
              placeholder="Paste the interviewer's LinkedIn profile text here..."
            ></textarea>
            {#if interviewers.length > 1}
              <button type="button" class="btn-remove" onclick={() => removeInterviewer(i)}>✕</button>
            {/if}
          </div>
        </div>
      {/each}
      <small>Paste text copied from their LinkedIn page. Add one entry per interviewer.</small>
    </div>

    <div class="field">
      <label for="cv-file">Upload CV / Resume</label>
      <input id="cv-file" type="file" accept=".pdf,.docx,.txt,.md,.pptx,.xlsx,.csv,.png,.jpg,.jpeg,.gif,.webp" onchange={handleFileChange} />
      <small>Supported: PDF, Word, PowerPoint, Excel, CSV, plain text, images</small>
    </div>

    <div class="field">
      <label for="interviewee-linkedin">Your LinkedIn Profile</label>
      <textarea
        id="interviewee-linkedin"
        bind:value={intervieweeLinkedin}
        rows={4}
        placeholder="Paste your own LinkedIn profile text here so the AI knows your background in depth..."
      ></textarea>
      <small>Helps the AI reference your experience accurately when coaching your answers</small>
    </div>

    <div class="field">
      <label for="extra">Additional Experience / Notes</label>
      <textarea
        id="extra"
        bind:value={extraExperience}
        rows={4}
        placeholder="Add any extra context, achievements, or talking points..."
      ></textarea>
      <div class="file-row">
        <label class="file-label" for="extra-file">Or upload a file</label>
        <input id="extra-file" type="file" accept=".pdf,.docx,.txt,.md,.pptx,.xlsx,.csv,.png,.jpg,.jpeg,.gif,.webp" onchange={handleExtraFileChange} />
        {#if extraFile}<span class="file-chosen">{extraFile.name}</span>{/if}
      </div>
      <small>Supported: PDF, Word, PowerPoint (.pptx), Excel (.xlsx), CSV, images — text is extracted automatically</small>
    </div>

    <button onclick={handleSubmit} disabled={loading} class="btn-primary">
      {loading ? (loadingStep || 'Processing…') : 'Start Session'}
    </button>
    {#if loading}
      <p class="loading-note">This can take 30–60 seconds if a company URL was provided.</p>
    {/if}
  {/if}
</div>

<style>
  .setup-form { max-width: 720px; margin: 0 auto; padding: 2rem; scroll-margin-top: 1rem; }
  .setup-success {
    padding: 0.75rem 1rem; background: #052e16; border: 1px solid #166534;
    border-radius: 0.5rem; color: #4ade80; font-size: 0.875rem; font-weight: 500;
  }
  .loading-note { margin-top: 0.5rem; color: #64748b; font-size: 0.8rem; }
  h2 { font-size: 1.75rem; margin-bottom: 0.5rem; color: #f1f5f9; }
  .subtitle { color: #94a3b8; margin-bottom: 2rem; }
  .field { margin-bottom: 1.5rem; }
  .field-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.5rem;
  }
  .field-header .field-label { margin-bottom: 0; }
  .field-label { font-weight: 500; color: #cbd5e1; }
  label { display: block; margin-bottom: 0.5rem; font-weight: 500; color: #cbd5e1; }
  textarea, input[type='url'] {
    width: 100%; padding: 0.75rem;
    background: #1e293b; border: 1px solid #334155;
    border-radius: 0.5rem; color: #e2e8f0;
    font-size: 0.9rem; resize: vertical;
  }
  input[type='file'] { color: #94a3b8; }
  .file-row { display: flex; align-items: center; gap: 0.75rem; margin-top: 0.5rem; flex-wrap: wrap; }
  .file-label { font-size: 0.8rem; color: #64748b; white-space: nowrap; }
  .file-chosen { font-size: 0.8rem; color: #60a5fa; }
  small { display: block; margin-top: 0.25rem; color: #64748b; font-size: 0.8rem; }
  .interviewer-entry { margin-bottom: 0.75rem; }
  .interviewer-label { font-size: 0.75rem; color: #60a5fa; font-weight: 600; margin-bottom: 0.25rem; }
  .interviewer-row { display: flex; gap: 0.5rem; align-items: flex-start; }
  .interviewer-row textarea { flex: 1; }
  .btn-add {
    padding: 0.3rem 0.75rem; font-size: 0.8rem; font-weight: 600;
    background: transparent; border: 1px solid #3b82f6; color: #60a5fa;
    border-radius: 0.375rem; cursor: pointer; white-space: nowrap;
    transition: background 0.15s;
  }
  .btn-add:hover { background: #1e3a5f; }
  .btn-remove {
    flex-shrink: 0; padding: 0.4rem 0.6rem; background: transparent;
    border: 1px solid #334155; color: #64748b; border-radius: 0.375rem;
    cursor: pointer; font-size: 0.85rem; transition: all 0.15s;
    margin-top: 0.25rem;
  }
  .btn-remove:hover { border-color: #ef4444; color: #ef4444; }
  .error {
    padding: 1rem; background: #450a0a; border: 1px solid #7f1d1d;
    border-radius: 0.5rem; color: #fca5a5; margin-bottom: 1rem;
  }
  .preview { margin-bottom: 1.5rem; }
  .preview summary { cursor: pointer; color: #60a5fa; }
  .preview pre {
    margin-top: 0.5rem; padding: 1rem; background: #1e293b;
    border-radius: 0.5rem; white-space: pre-wrap; font-size: 0.75rem;
    color: #94a3b8; max-height: 200px; overflow: auto;
  }
  .btn-primary {
    padding: 0.75rem 2rem; background: #3b82f6; color: white;
    border: none; border-radius: 0.5rem; font-size: 1rem;
    cursor: pointer; font-weight: 600; transition: background 0.2s;
  }
  .btn-primary:hover:not(:disabled) { background: #2563eb; }
  .btn-primary:disabled { background: #1e3a5f; cursor: not-allowed; }
  .post-setup { display: flex; flex-direction: column; gap: 1.25rem; }
  .tab-bar { display: flex; gap: 0.25rem; border-bottom: 1px solid #1e293b; padding-bottom: 0.5rem; }
  .tab { padding: 0.3rem 0.9rem; background: transparent; border: 1px solid #1e293b; border-radius: 0.375rem; color: #475569; font-size: 0.8rem; cursor: pointer; transition: all 0.15s; }
  .tab:hover { border-color: #334155; color: #94a3b8; }
  .tab.tab-active { background: #1e293b; border-color: #334155; color: #e2e8f0; }
  .section-block { display: flex; flex-direction: column; gap: 0.4rem; }
  .section-block-label { font-size: 0.68rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.07em; color: #475569; }
  .keyword-chips { display: flex; flex-wrap: wrap; gap: 0.3rem 0.4rem; }
  .kw-chip { font-size: 0.72rem; padding: 0.15rem 0.5rem; background: #0f172a; border: 1px solid #1e293b; border-radius: 9999px; color: #60a5fa; }
  .predicted { background: #1e293b; border-radius: 0.5rem; padding: 1.25rem; }
  .predicted h3 { font-size: 0.85rem; color: #60a5fa; text-transform: uppercase; letter-spacing: 0.07em; margin-bottom: 0.75rem; }
  .questions-list { margin: 0; padding-left: 1.5rem; display: flex; flex-direction: column; gap: 0.5rem; }
  .questions-list li { color: #cbd5e1; font-size: 0.9rem; line-height: 1.5; }
  .action-row { display: flex; gap: 1rem; flex-wrap: wrap; }
  .btn-secondary {
    padding: 0.75rem 2rem; background: transparent; color: #60a5fa;
    border: 2px solid #3b82f6; border-radius: 0.5rem; font-size: 1rem;
    cursor: pointer; font-weight: 600; transition: all 0.2s;
  }
  .btn-secondary:hover { background: #1e3a5f; }
</style>
