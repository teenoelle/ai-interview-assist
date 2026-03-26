<script lang="ts">
  import type { TranscriptEntry, SuggestionEntry } from '../lib/types';

  const { transcript, suggestions, onClose, onSave, recordingUrl } = $props<{
    transcript: TranscriptEntry[];
    suggestions: SuggestionEntry[];
    onClose: () => void;
    onSave?: (result: DebriefResult) => void;
    recordingUrl?: string;
  }>();

  type Tab = 'review' | 'qa' | 'recording';
  let activeTab = $state<Tab>('review');

  interface DebriefResult {
    summary: string;
    strong_points: string[];
    improvement_areas: string[];
    followup_email: string[];
    followup_email_draft?: string;
  }

  let loading = $state(true);
  let result = $state<DebriefResult | null>(null);
  let error = $state('');
  let copied = $state(false);
  let saved = $state(false);
  let emailTo = $state(localStorage.getItem('debrief-email') ?? '');
  let emailSent = $state(false);
  let nextSteps = $state<string[]>([]);
  let loadingNextSteps = $state(false);

  const qaEntries = $derived(suggestions.filter(s => s.question && s.suggestion));

  async function fetchNextSteps() {
    if (transcript.length === 0) return;
    loadingNextSteps = true;
    try {
      const resp = await fetch('/api/next-steps', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ transcript: transcript.map(e => ({ speaker: e.speaker, text: e.text })) }),
      });
      if (resp.ok) { const d = await resp.json(); nextSteps = d.steps ?? []; }
    } catch { /* ignore */ }
    loadingNextSteps = false;
  }

  async function fetchDebrief() {
    fetchNextSteps();
    try {
      const resp = await fetch('/api/debrief', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          transcript: transcript.map(e => ({ speaker: e.speaker, text: e.text })),
          suggestions: suggestions.filter(s => s.suggestion).map(s => ({ question: s.question, suggestion: s.suggestion })),
        }),
      });
      if (!resp.ok) throw new Error(`Debrief failed: ${resp.status}`);
      result = await resp.json();
      onSave?.(result);
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  fetchDebrief();

  async function copyEmail() {
    if (!result) return;
    const text = result.followup_email_draft
      ?? result.followup_email.map(p => `• ${p}`).join('\n');
    await navigator.clipboard.writeText(text);
    copied = true;
    setTimeout(() => { copied = false; }, 2000);
  }

  function composeDebriefText(): string {
    if (!result) return '';
    const lines: string[] = [];
    lines.push('INTERVIEW DEBRIEF');
    lines.push('=================', '');
    lines.push('SUMMARY', result.summary, '');
    if (result.strong_points.length) {
      lines.push('STRONG MOMENTS');
      result.strong_points.forEach(p => lines.push(`• ${p}`));
      lines.push('');
    }
    if (result.improvement_areas.length) {
      lines.push('AREAS TO IMPROVE');
      result.improvement_areas.forEach(p => lines.push(`• ${p}`));
      lines.push('');
    }
    if (result.followup_email_draft) {
      lines.push('---', 'FOLLOW-UP EMAIL DRAFT', '---', result.followup_email_draft);
    } else if (result.followup_email.length) {
      lines.push('FOLLOW-UP EMAIL POINTS');
      result.followup_email.forEach(p => lines.push(`• ${p}`));
    }
    return lines.join('\n');
  }

  function sendDebriefEmail() {
    if (!result || !emailTo.trim()) return;
    localStorage.setItem('debrief-email', emailTo.trim());
    const subject = encodeURIComponent('Interview Debrief');
    const body = encodeURIComponent(composeDebriefText());
    // mailto: body has ~2000 char limit in some clients; truncate gracefully
    const href = `mailto:${encodeURIComponent(emailTo.trim())}?subject=${subject}&body=${body}`;
    window.open(href.slice(0, 2000), '_self');
    emailSent = true;
    setTimeout(() => { emailSent = false; }, 3000);
  }
</script>

<div class="modal-backdrop" onclick={onClose} role="none">
  <div class="modal" onclick={(e) => e.stopPropagation()} role="dialog">
    <div class="modal-header">
      <h2>Interview Debrief</h2>
      <button class="close-btn" onclick={onClose}>✕</button>
    </div>

    <!-- Tab bar -->
    <div class="tab-bar">
      <button class="tab" class:active={activeTab === 'review'} onclick={() => activeTab = 'review'}>
        AI Review
      </button>
      <button class="tab" class:active={activeTab === 'qa'} onclick={() => activeTab = 'qa'}>
        Q&amp;A {qaEntries.length > 0 ? `(${qaEntries.length})` : ''}
      </button>
      <button class="tab" class:active={activeTab === 'recording'} onclick={() => activeTab = 'recording'}>
        Recording
      </button>
    </div>

    <div class="modal-body">

      <!-- AI Review tab -->
      {#if activeTab === 'review'}
        {#if loading}
          <div class="loading">Analyzing your interview...</div>
        {:else if error}
          <div class="error">{error}</div>
        {:else if result}
          <section>
            <h3>Overall</h3>
            <p class="summary">{result.summary}</p>
          </section>

          <div class="two-col">
            <section>
              <h3 class="green">Strong Moments</h3>
              <ul>
                {#each result.strong_points as point}
                  <li>{point}</li>
                {/each}
              </ul>
            </section>
            <section>
              <h3 class="yellow">Areas to Improve</h3>
              <ul>
                {#each result.improvement_areas as area}
                  <li>{area}</li>
                {/each}
              </ul>
            </section>
          </div>

          <section class="next-steps-section">
            <h3 class="amber">Next Steps</h3>
            {#if loadingNextSteps}
              <p class="steps-loading">Extracting next steps...</p>
            {:else if nextSteps.length > 0}
              <ul>
                {#each nextSteps as step}
                  <li>{step}</li>
                {/each}
              </ul>
            {:else}
              <p class="steps-empty">No specific next steps mentioned in the interview.</p>
            {/if}
          </section>

          <section class="email-section">
            <div class="followup-header">
              <h3>Follow-up Email</h3>
              <button class="copy-btn" class:copied onclick={copyEmail}>
                {copied ? '✓ Copied!' : 'Copy email'}
              </button>
            </div>
            {#if result.followup_email_draft}
              <div class="email-draft">
                {#each result.followup_email_draft.split('\n') as line}
                  {#if line.trim().startsWith('Subject:')}
                    <div class="email-subject">{line}</div>
                  {:else if line.trim() === ''}
                    <div class="email-blank"></div>
                  {:else}
                    <div class="email-line">{line}</div>
                  {/if}
                {/each}
              </div>
            {:else}
              <ul>
                {#each result.followup_email as point}
                  <li>{point}</li>
                {/each}
              </ul>
            {/if}
          </section>
        {/if}
      {/if}

      <!-- Recording tab -->
      {#if activeTab === 'recording'}
        {#if recordingUrl}
          <section class="recording-section">
            <h3>App Screen Recording</h3>
            <video class="recording-player" src={recordingUrl} controls></video>
            <a class="recording-download" href={recordingUrl} download="interview-recording.webm">
              ⬇ Download recording
            </a>
          </section>
        {:else}
          <div class="recording-empty">
            <p class="steps-empty">No recording for this session.</p>
            <p class="steps-empty">To record the app screen during a future interview, click <strong>Record App Screen</strong> when starting the meeting capture.</p>
          </div>
        {/if}
      {/if}

      <!-- Q&A tab -->
      {#if activeTab === 'qa'}
        <section class="qa-section">
          {#if qaEntries.length === 0}
            <p class="steps-empty">No questions detected this session.</p>
          {:else}
            {#each qaEntries as entry, i}
              <div class="qa-item">
                <div class="qa-q"><span class="qa-num">{i + 1}</span>{entry.question}</div>
                <div class="qa-coaching">{entry.suggestion}</div>
              </div>
            {/each}
          {/if}
        </section>
      {/if}

    </div>

    <!-- Footer -->
    {#if result}
      <div class="modal-footer">
        <div class="footer-left">
          <button
            class="save-btn"
            class:saved
            onclick={() => {
              const text = composeDebriefText();
              const blob = new Blob([text], { type: 'text/plain' });
              const url = URL.createObjectURL(blob);
              const a = document.createElement('a');
              a.href = url;
              a.download = `debrief-${new Date().toISOString().slice(0,10)}.txt`;
              a.click();
              URL.revokeObjectURL(url);
              saved = true;
              setTimeout(() => { saved = false; }, 2500);
            }}
          >{saved ? '✓ Saved!' : 'Save Report'}</button>
        </div>
        {#if activeTab === 'review'}
          <div class="footer-email">
            <span class="email-footer-label">Email to myself</span>
            <input
              class="email-input"
              type="email"
              placeholder="you@email.com"
              bind:value={emailTo}
              onkeydown={(e) => { if (e.key === 'Enter') sendDebriefEmail(); }}
            />
            <button
              class="send-btn"
              class:sent={emailSent}
              onclick={sendDebriefEmail}
              disabled={!emailTo.trim()}
            >{emailSent ? '✓ Opening…' : 'Send'}</button>
          </div>
        {/if}
      </div>
    {/if}
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed; inset: 0; background: rgba(0,0,0,0.7);
    display: flex; align-items: center; justify-content: center;
    z-index: 100;
  }
  .modal {
    background: #0f172a; border: 1px solid #1e293b; border-radius: 0.75rem;
    width: min(700px, 95vw); max-height: 85vh; display: flex; flex-direction: column;
  }
  .modal-header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 1rem 1.5rem; border-bottom: 1px solid #1e293b; flex-shrink: 0;
  }
  h2 { font-size: 1.2rem; font-weight: 700; color: #f1f5f9; margin: 0; }
  .close-btn {
    background: none; border: none; color: #64748b; font-size: var(--fs-lg);
    cursor: pointer; padding: 0.2rem 0.4rem;
  }
  .close-btn:hover { color: #e2e8f0; }
  .tab-bar {
    display: flex; border-bottom: 1px solid #1e293b; flex-shrink: 0;
    padding: 0 1rem;
  }
  .tab {
    padding: 0.6rem 1rem; background: none; border: none; border-bottom: 2px solid transparent;
    color: #475569; font-size: var(--fs-base); font-weight: 600; cursor: pointer;
    margin-bottom: -1px; transition: all 0.15s; white-space: nowrap;
  }
  .tab:hover { color: #94a3b8; }
  .tab.active { color: #60a5fa; border-bottom-color: #3b82f6; }
  .modal-body { overflow-y: auto; padding: 1.5rem; display: flex; flex-direction: column; gap: 1.5rem; }
  section { display: flex; flex-direction: column; gap: 0.5rem; }
  h3 { font-size: var(--fs-base); font-weight: 700; text-transform: uppercase; letter-spacing: 0.07em; color: #94a3b8; margin: 0; }
  h3.green { color: #4ade80; }
  h3.yellow { color: #f59e0b; }
  h3.amber { color: #fb923c; }
  .next-steps-section { gap: 0.5rem; display: flex; flex-direction: column; }
  .steps-loading, .steps-empty { color: #475569; font-size: var(--fs-base); font-style: italic; margin: 0; }
  .summary { color: #cbd5e1; line-height: 1.6; font-size: var(--fs-base); margin: 0; }
  ul { margin: 0; padding-left: 1.25rem; display: flex; flex-direction: column; gap: 0.3rem; }
  li { color: #94a3b8; font-size: 0.875rem; line-height: 1.5; }
  .two-col { display: grid; grid-template-columns: 1fr 1fr; gap: 1.5rem; }
  .followup-header { display: flex; align-items: center; justify-content: space-between; }
  .copy-btn {
    padding: 0.2rem 0.7rem; background: transparent;
    border: 1px solid #334155; border-radius: 0.25rem;
    color: #64748b; font-size: var(--fs-sm); cursor: pointer;
    transition: all 0.15s;
  }
  .copy-btn:hover { border-color: #60a5fa; color: #60a5fa; }
  .copy-btn.copied { border-color: #4ade80; color: #4ade80; }

  .email-section { gap: 0.75rem; }
  .email-draft {
    background: #060e1a;
    border: 1px solid #1a2d4a;
    border-radius: 0.5rem;
    padding: 1rem 1.25rem;
    font-family: 'Georgia', serif;
    user-select: text;
    cursor: text;
  }
  .email-subject {
    font-size: var(--fs-base);
    font-weight: 700;
    color: #93c5fd;
    margin-bottom: 0.5rem;
    padding-bottom: 0.5rem;
    border-bottom: 1px solid #1a2d4a;
  }
  .email-line {
    font-size: var(--fs-base);
    color: #cbd5e1;
    line-height: 1.7;
  }
  .email-blank { height: 0.7rem; }
  .loading { color: #60a5fa; font-style: italic; text-align: center; padding: 2rem; }
  .error { color: #fca5a5; padding: 1rem; background: #450a0a; border-radius: 0.5rem; }

  .modal-footer {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.75rem 1.5rem;
    border-top: 1px solid #1e293b;
    flex-shrink: 0;
    background: #080f1c;
    border-radius: 0 0 0.75rem 0.75rem;
  }
  .footer-left { display: flex; align-items: center; flex-shrink: 0; }
  .footer-email { display: flex; align-items: center; gap: 0.6rem; flex: 1; justify-content: flex-end; }
  .save-btn {
    padding: 0.3rem 1rem;
    background: #1d4ed8;
    border: none;
    border-radius: 0.3rem;
    color: white;
    font-size: var(--fs-base);
    font-weight: 700;
    cursor: pointer;
    transition: all 0.15s;
    white-space: nowrap;
  }
  .save-btn:hover { background: #2563eb; }
  .save-btn.saved { background: #166534; }
  .email-footer-label {
    font-size: var(--fs-sm);
    color: #475569;
    white-space: nowrap;
  }
  .email-input {
    flex: 1;
    padding: 0.3rem 0.65rem;
    background: #0f172a;
    border: 1px solid #1e293b;
    border-radius: 0.3rem;
    color: #e2e8f0;
    font-size: var(--fs-base);
    outline: none;
    transition: border-color 0.15s;
  }
  .email-input:focus { border-color: #3b82f6; }
  .send-btn {
    padding: 0.3rem 0.9rem;
    background: #1d4ed8;
    border: none;
    border-radius: 0.3rem;
    color: white;
    font-size: var(--fs-base);
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s;
    white-space: nowrap;
  }
  .send-btn:hover:not(:disabled) { background: #2563eb; }
  .send-btn:disabled { opacity: 0.4; cursor: default; }
  .send-btn.sent { background: #166534; }

  .recording-section { display: flex; flex-direction: column; gap: 0.75rem; }
  .recording-player { width: 100%; border-radius: 0.5rem; background: #000; max-height: 50vh; }
  .recording-download {
    align-self: flex-start; font-size: var(--fs-sm); color: #60a5fa;
    text-decoration: none; padding: 0.25rem 0.75rem;
    border: 1px solid #334155; border-radius: 0.375rem; transition: all 0.15s;
  }
  .recording-download:hover { border-color: #60a5fa; background: #1e3a5f; }
  .recording-empty { display: flex; flex-direction: column; gap: 0.75rem; padding: 1rem 0; }
  .qa-section { display: flex; flex-direction: column; gap: 0.75rem; }
  .qa-item {
    background: #060e1a; border: 1px solid #1a2d4a; border-radius: 0.4rem;
    padding: 0.65rem 0.8rem; display: flex; flex-direction: column; gap: 0.4rem;
  }
  .qa-q {
    font-size: var(--fs-sm); font-weight: 700; color: #93c5fd;
    display: flex; gap: 0.5rem; align-items: baseline; line-height: 1.4;
  }
  .qa-num {
    font-size: var(--fs-xs); color: #334155; font-weight: 700;
    background: #0d1f35; border: 1px solid #1e3a5f; border-radius: 0.25rem;
    padding: 0 0.3em; line-height: 1.6; flex-shrink: 0;
  }
  .qa-coaching {
    font-size: var(--fs-sm); color: #94a3b8; line-height: 1.55;
    white-space: pre-wrap;
  }
</style>
