<script lang="ts">
  import { submitSetup, type CompanyBrief, type InterviewerSummary } from '../lib/api';
  import { saveKeywords } from '../lib/keywordTracker';
  function load(key: string, fallback: string) { return localStorage.getItem(key) ?? fallback; }
  function loadArr(key: string, fallback: string[]): string[] {
    try { const v = localStorage.getItem(key); return v ? JSON.parse(v) : fallback; } catch { return fallback; }
  }

  interface Preset {
    id: string;
    name: string;
    data: {
      companyName: string;
      roleName: string;
      jobDescription: string;
      companyUrl: string;
      interviewers: string[];
      intervieweeLinkedin: string;
      portfolioUrls: string[];
      extraExperience: string;
    };
  }

  function loadPresets(): Preset[] {
    try { const v = localStorage.getItem('setup-presets'); return v ? JSON.parse(v) : []; } catch { return []; }
  }
  function savePresetsToStorage(list: Preset[]) {
    localStorage.setItem('setup-presets', JSON.stringify(list));
  }

  let companyName = $state(load('setup-company-name', ''));
  let roleName = $state(load('setup-role-name', ''));
  let jobDescription = $state(load('setup-jd', ''));
  let companyUrl = $state(load('setup-company-url', ''));
  let interviewers = $state<string[]>(loadArr('setup-interviewers', ['']));
  let intervieweeLinkedin = $state(load('setup-interviewee-linkedin', ''));
  let portfolioUrls = $state<string[]>(loadArr('setup-portfolio-urls', ['']));
  let extraExperience = $state(load('setup-extra-experience', ''));
  let cvFile: File | null = $state(null);
  let extraFile: File | null = $state(null);
  let presets = $state<Preset[]>(loadPresets());
  let selectedPresetId = $state('');
  let savedFlash = $state(false);

  $effect(() => { localStorage.setItem('setup-company-name', companyName); });
  $effect(() => { localStorage.setItem('setup-role-name', roleName); });
  $effect(() => { localStorage.setItem('setup-jd', jobDescription); });
  $effect(() => { localStorage.setItem('setup-company-url', companyUrl); });
  $effect(() => { localStorage.setItem('setup-interviewers', JSON.stringify(interviewers)); });
  $effect(() => { localStorage.setItem('setup-interviewee-linkedin', intervieweeLinkedin); });
  $effect(() => { localStorage.setItem('setup-portfolio-urls', JSON.stringify(portfolioUrls)); });
  $effect(() => { localStorage.setItem('setup-extra-experience', extraExperience); });

  let loading = $state(false);
  let loadingStep = $state('');
  let error = $state('');
  let formEl: HTMLDivElement | undefined = $state();

  let companyBrief = $state<CompanyBrief | null>(null);
  let interviewerSummaries = $state<InterviewerSummary[]>([]);
  let jdKeywords = $state<string[]>([]);
  const props = $props<{
    onSetupComplete: (data?: { companyBrief?: any; interviewerSummaries?: any[]; jdKeywords?: string[]; }) => void;
  }>();

  function currentData(): Preset['data'] {
    return { companyName, roleName, jobDescription, companyUrl, interviewers: [...interviewers], intervieweeLinkedin, portfolioUrls: [...portfolioUrls], extraExperience };
  }

  function applyPreset(p: Preset) {
    companyName = p.data.companyName;
    roleName = p.data.roleName;
    jobDescription = p.data.jobDescription;
    companyUrl = p.data.companyUrl;
    interviewers = p.data.interviewers.length ? [...p.data.interviewers] : [''];
    intervieweeLinkedin = p.data.intervieweeLinkedin;
    portfolioUrls = p.data.portfolioUrls.length ? [...p.data.portfolioUrls] : [''];
    extraExperience = p.data.extraExperience;
    selectedPresetId = p.id;
  }

  function savePreset() {
    const name = [companyName.trim(), roleName.trim()].filter(Boolean).join(' — ') || 'Untitled';
    // Replace existing preset with same name, or add new
    const existing = presets.find(p => p.name === name);
    const id = existing?.id ?? Date.now().toString();
    const updated: Preset = { id, name, data: currentData() };
    presets = [...presets.filter(p => p.id !== id), updated];
    savePresetsToStorage(presets);
    selectedPresetId = id;
    savedFlash = true;
    setTimeout(() => savedFlash = false, 1500);
  }

  function deleteSelectedPreset() {
    if (!selectedPresetId) return;
    presets = presets.filter(p => p.id !== selectedPresetId);
    savePresetsToStorage(presets);
    selectedPresetId = '';
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

  function addPortfolio() { portfolioUrls = [...portfolioUrls, '']; }
  function removePortfolio(i: number) {
    portfolioUrls = portfolioUrls.filter((_, idx) => idx !== i);
    if (portfolioUrls.length === 0) portfolioUrls = [''];
  }
  function updatePortfolio(i: number, val: string) {
    portfolioUrls = portfolioUrls.map((v, idx) => idx === i ? val : v);
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
      const portfolioText = portfolioUrls.filter(u => u.trim()).join('\n');
      if (portfolioText) formData.append('portfolio_url', portfolioText);

      formData.append('extra_experience', extraExperience);
      if (cvFile) formData.append('cv_file', cvFile);
      if (extraFile) formData.append('extra_file', extraFile);

      loadingStep = 'Generating your coaching profile…';
      const result = await submitSetup(formData);
      companyBrief = result.company_brief ?? null;
      interviewerSummaries = result.interviewer_summaries ?? [];
      jdKeywords = result.jd_keywords ?? [];
      if (jdKeywords.length > 0) saveKeywords(jdKeywords);
      props.onSetupComplete({ companyBrief, interviewerSummaries, jdKeywords });
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

  <!-- Preset bar -->
  <div class="preset-bar">
    {#if presets.length > 0}
      <select
        class="preset-select"
        value={selectedPresetId}
        onchange={(e) => { const id = (e.target as HTMLSelectElement).value; if (id) applyPreset(presets.find(p => p.id === id)!); }}
      >
        <option value="">Load a saved setup…</option>
        {#each presets as p (p.id)}
          <option value={p.id}>{p.name}</option>
        {/each}
      </select>
      {#if selectedPresetId}
        <button type="button" class="preset-delete" onclick={deleteSelectedPreset} title="Delete preset">✕</button>
      {/if}
    {/if}
    <button type="button" class="preset-save" onclick={savePreset}>
      {savedFlash ? '✓ Saved' : '⬇ Save Setup'}
    </button>
  </div>

  <!-- Company and role name (used for preset naming) -->
  <div class="name-row">
    <div class="field name-field">
      <label for="company-name">Company Name</label>
      <input id="company-name" type="text" bind:value={companyName} placeholder="Acme Corp" />
    </div>
    <div class="field name-field">
      <label for="role-name">Role / Position</label>
      <input id="role-name" type="text" bind:value={roleName} placeholder="Senior Product Manager" />
    </div>
  </div>

  {#if error}
    <div class="error">{error}</div>
  {/if}

  <div class="field">
      <label for="job-desc">Job Description</label>
      <textarea
        id="job-desc"
        bind:value={jobDescription}
        rows={4}
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
              rows={3}
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
        rows={3}
        placeholder="Paste your own LinkedIn profile text here so the AI knows your background in depth..."
      ></textarea>
      <small>Helps the AI reference your experience accurately when coaching your answers</small>
    </div>

    <div class="field">
      <div class="field-header">
        <span class="field-label">Portfolio / Personal Website(s)</span>
        <button type="button" class="btn-add" onclick={addPortfolio}>+ Add URL</button>
      </div>
      {#each portfolioUrls as url, i (i)}
        <div class="portfolio-row">
          <input
            type="url"
            value={url}
            oninput={(e) => updatePortfolio(i, (e.target as HTMLInputElement).value)}
            placeholder="https://yourportfolio.com"
            class="portfolio-input"
          />
          {#if portfolioUrls.length > 1}
            <button type="button" class="btn-remove" onclick={() => removePortfolio(i)}>✕</button>
          {/if}
        </div>
      {/each}
      <small>We'll crawl each URL to help the AI reference your projects and experience accurately</small>
    </div>

    <div class="field">
      <label for="extra">Additional Experience / Notes</label>
      <textarea
        id="extra"
        bind:value={extraExperience}
        rows={3}
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
      {loading ? (loadingStep || 'Processing…') : 'Start Interview →'}
    </button>
    {#if loading}
      <p class="loading-note">This can take 30–60 seconds if a company URL was provided.</p>
    {/if}
</div>

<style>
  .setup-form { max-width: 720px; margin: 0 auto; padding: 1rem 2rem 2rem; scroll-margin-top: 1rem; }
  .preset-bar {
    display: flex; align-items: center; gap: 0.5rem; margin-bottom: 1.25rem;
    padding: 0.6rem 0.75rem; background: #0f172a; border: 1px solid #1e293b;
    border-radius: 0.5rem; flex-wrap: wrap;
  }
  .preset-select {
    flex: 1; min-width: 10rem; padding: 0.35rem 0.5rem;
    background: #1e293b; border: 1px solid #334155; border-radius: 0.375rem;
    color: #e2e8f0; font-size: var(--fs-base); cursor: pointer;
  }
  .preset-save {
    padding: 0.35rem 0.85rem; background: transparent; border: 1px solid #334155;
    border-radius: 0.375rem; color: #60a5fa; font-size: var(--fs-base);
    cursor: pointer; white-space: nowrap; transition: all 0.15s;
  }
  .preset-save:hover { border-color: #60a5fa; background: #1e3a5f; }
  .preset-delete {
    padding: 0.35rem 0.55rem; background: transparent; border: 1px solid #334155;
    border-radius: 0.375rem; color: #64748b; font-size: var(--fs-base); cursor: pointer;
    transition: all 0.15s;
  }
  .preset-delete:hover { border-color: #ef4444; color: #ef4444; }
  .name-row { display: flex; gap: 1rem; margin-bottom: 0; }
  .name-field { flex: 1; }
  input[type='text'] {
    width: 100%; padding: 0.75rem;
    background: #1e293b; border: 1px solid #334155;
    border-radius: 0.5rem; color: #e2e8f0;
    font-size: var(--fs-base);
  }
  .setup-success {
    padding: 0.75rem 1rem; background: #052e16; border: 1px solid #166534;
    border-radius: 0.5rem; color: #4ade80; font-size: 0.875rem; font-weight: 500;
  }
  .btn-back { background: none; border: 1px solid #1e293b; color: #60a5fa; font-size: var(--fs-sm); font-weight: 600; cursor: pointer; padding: 0.5rem 1rem; border-radius: 0.5rem; white-space: nowrap; }
  .btn-back:hover { border-color: #60a5fa; }
  .loading-note { margin-top: 0.5rem; color: #64748b; font-size: var(--fs-base); }
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
    font-size: var(--fs-base);
  }
  textarea {
    resize: vertical;
    min-height: 5rem;
  }
  input[type='file'] { color: #94a3b8; }
  .file-row { display: flex; align-items: center; gap: 0.75rem; margin-top: 0.5rem; flex-wrap: wrap; }
  .file-label { font-size: var(--fs-base); color: #64748b; white-space: nowrap; }
  .file-chosen { font-size: var(--fs-base); color: #60a5fa; }
  small { display: block; margin-top: 0.25rem; color: #64748b; font-size: var(--fs-base); }
  .portfolio-row { display: flex; gap: 0.5rem; align-items: center; margin-bottom: 0.4rem; }
  .portfolio-input { flex: 1; padding: 0.5rem 0.75rem; background: #1e293b; border: 1px solid #334155; border-radius: 0.5rem; color: #e2e8f0; font-size: var(--fs-base); }
  .interviewer-entry { margin-bottom: 0.75rem; }
  .interviewer-label { font-size: var(--fs-sm); color: #60a5fa; font-weight: 600; margin-bottom: 0.25rem; }
  .interviewer-row { display: flex; gap: 0.5rem; align-items: flex-start; }
  .interviewer-row textarea { flex: 1; }
  .btn-add {
    padding: 0.3rem 0.75rem; font-size: var(--fs-base); font-weight: 600;
    background: transparent; border: 1px solid #3b82f6; color: #60a5fa;
    border-radius: 0.375rem; cursor: pointer; white-space: nowrap;
    transition: background 0.15s;
  }
  .btn-add:hover { background: #1e3a5f; }
  .btn-remove {
    flex-shrink: 0; padding: 0.4rem 0.6rem; background: transparent;
    border: 1px solid #334155; color: #64748b; border-radius: 0.375rem;
    cursor: pointer; font-size: var(--fs-base); transition: all 0.15s;
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
    border-radius: 0.5rem; white-space: pre-wrap; font-size: var(--fs-sm);
    color: #94a3b8; max-height: 50vh; overflow: auto; resize: vertical;
  }
  .btn-primary {
    padding: 0.75rem 2rem; background: #3b82f6; color: white;
    border: none; border-radius: 0.5rem; font-size: 1rem;
    cursor: pointer; font-weight: 600; transition: background 0.2s;
  }
  .btn-primary:hover:not(:disabled) { background: #2563eb; }
  .btn-primary:disabled { background: #1e3a5f; cursor: not-allowed; }
  .post-setup { display: flex; flex-direction: column; gap: 1.25rem; }
  .section-block { display: flex; flex-direction: column; gap: 0.4rem; }
  .section-block-label { font-size: var(--fs-sm); font-weight: 700; text-transform: uppercase; letter-spacing: 0.07em; color: #475569; }
  .predicted { background: #1e293b; border-radius: 0.5rem; padding: 1.25rem; }
  .predicted-toggle {
    display: flex; align-items: center; gap: 0.5rem;
    width: 100%; background: none; border: none; cursor: pointer;
    padding: 0; margin-bottom: 0; text-align: left;
  }
  .predicted-toggle:hover .predicted-label { color: #93c5fd; }
  .predicted-label { font-size: var(--fs-base); color: #60a5fa; text-transform: uppercase; letter-spacing: 0.07em; font-weight: 700; flex: 1; }
  .predicted-count { font-size: var(--fs-xs); color: #334155; background: #0f172a; border-radius: 9999px; padding: 0.05rem 0.45rem; }
  .predicted-chevron { font-size: var(--fs-xs); color: #334155; }
  .predicted-context { font-size: var(--fs-sm); color: #64748b; font-style: italic; margin: 0.75rem 0 0.75rem; }
  .questions-list { margin: 0.75rem 0 0; padding-left: 1.5rem; display: flex; flex-direction: column; gap: 0.5rem; }
  .questions-list li { color: #cbd5e1; font-size: var(--fs-base); line-height: 1.5; }
  .action-row { display: flex; align-items: center; justify-content: space-between; gap: 1rem; flex-wrap: wrap; border-top: 1px solid #1e293b; padding-top: 1.25rem; margin-top: 0.5rem; }
  .action-row-right { display: flex; align-items: center; gap: 0.75rem; margin-left: auto; }
  .btn-secondary {
    padding: 0.75rem 2rem; background: transparent; color: #60a5fa;
    border: 2px solid #3b82f6; border-radius: 0.5rem; font-size: 1rem;
    cursor: pointer; font-weight: 600; transition: all 0.2s;
  }
  .btn-secondary:hover { background: #1e3a5f; }
</style>
