<script lang="ts">
  import type { TranscriptEntry, SuggestionEntry } from '../lib/types';

  const { transcript, suggestions, onClose, onSave } = $props<{
    transcript: TranscriptEntry[];
    suggestions: SuggestionEntry[];
    onClose: () => void;
    onSave?: (result: DebriefResult) => void;
  }>();

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
  let emailTo = $state(localStorage.getItem('debrief-email') ?? '');
  let emailSent = $state(false);

  async function fetchDebrief() {
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

    <div class="modal-body">
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

        <section class="email-section">
          <div class="followup-header">
            <h3>Follow-up Email</h3>
            <button class="copy-btn" class:copied onclick={copyEmail}>
              {copied ? '✓ Copied!' : 'Copy email'}
            </button>
          </div>
          {#if result.followup_email_draft}
            <div class="email-draft">
              {#each result.followup_email_draft.split('\n') as line, i}
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
    </div>

    <!-- Email footer — always visible when result is ready -->
    {#if result}
      <div class="email-footer">
        <span class="email-footer-label">Email debrief to myself</span>
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
          disabled={!emailTo.trim() || !result}
        >{emailSent ? '✓ Opening…' : 'Send'}</button>
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
    background: none; border: none; color: #64748b; font-size: 1.1rem;
    cursor: pointer; padding: 0.2rem 0.4rem;
  }
  .close-btn:hover { color: #e2e8f0; }
  .modal-body { overflow-y: auto; padding: 1.5rem; display: flex; flex-direction: column; gap: 1.5rem; }
  section { display: flex; flex-direction: column; gap: 0.5rem; }
  h3 { font-size: 0.8rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.07em; color: #94a3b8; margin: 0; }
  h3.green { color: #4ade80; }
  h3.yellow { color: #f59e0b; }
  .summary { color: #cbd5e1; line-height: 1.6; font-size: 0.9rem; margin: 0; }
  ul { margin: 0; padding-left: 1.25rem; display: flex; flex-direction: column; gap: 0.3rem; }
  li { color: #94a3b8; font-size: 0.875rem; line-height: 1.5; }
  .two-col { display: grid; grid-template-columns: 1fr 1fr; gap: 1.5rem; }
  .followup-header { display: flex; align-items: center; justify-content: space-between; }
  .copy-btn {
    padding: 0.2rem 0.7rem; background: transparent;
    border: 1px solid #334155; border-radius: 0.25rem;
    color: #64748b; font-size: 0.72rem; cursor: pointer;
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
    font-size: 0.82rem;
    font-weight: 700;
    color: #93c5fd;
    margin-bottom: 0.5rem;
    padding-bottom: 0.5rem;
    border-bottom: 1px solid #1a2d4a;
  }
  .email-line {
    font-size: 0.85rem;
    color: #cbd5e1;
    line-height: 1.7;
  }
  .email-blank { height: 0.7rem; }
  .loading { color: #60a5fa; font-style: italic; text-align: center; padding: 2rem; }
  .error { color: #fca5a5; padding: 1rem; background: #450a0a; border-radius: 0.5rem; }

  .email-footer {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.75rem 1.5rem;
    border-top: 1px solid #1e293b;
    flex-shrink: 0;
    background: #080f1c;
    border-radius: 0 0 0.75rem 0.75rem;
  }
  .email-footer-label {
    font-size: 0.72rem;
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
    font-size: 0.8rem;
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
    font-size: 0.8rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s;
    white-space: nowrap;
  }
  .send-btn:hover:not(:disabled) { background: #2563eb; }
  .send-btn:disabled { opacity: 0.4; cursor: default; }
  .send-btn.sent { background: #166534; }
</style>
