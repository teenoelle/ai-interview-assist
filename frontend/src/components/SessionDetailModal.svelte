<script lang="ts">
  import type { InterviewRecord } from '../lib/interviewHistory';
  import { authFetch } from '../lib/api';

  const { record, onClose, onDelete, onRehearsal } = $props<{
    record: InterviewRecord;
    onClose: () => void;
    onDelete: (id: string) => void;
    onRehearsal: (questions: string[]) => void;
  }>();

  // ── Follow-up email draft ──────────────────────────────────────────────────
  let showDraft = $state(false);
  let draftText = $state('');
  let draftCopied = $state(false);
  let aiDrafting = $state(false);
  let aiDraftError = $state('');

  function loadInterviewerNames(): string[] {
    try {
      const s = JSON.parse(localStorage.getItem('setup-interviewer-summaries') ?? '[]');
      return (s as Array<{ name: string }>).map(iv => iv.name).filter(Boolean);
    } catch { return []; }
  }

  function buildLocalTemplate(): string {
    const names = loadInterviewerNames();
    const salutation = names.length > 0
      ? names.map(n => n.trim().split(/\s+/)[0]).join(' and ')
      : 'there';
    const strongLine = record.strong_points[0]
      ? `\nI especially enjoyed our discussion around ${record.strong_points[0].replace(/^I\s+/i, '').toLowerCase().slice(0, 80)}.`
      : '';
    return `Dear ${salutation},\n\nThank you for taking the time to speak with me today. I enjoyed our conversation and learning more about the role and the team.${strongLine}\n\nI remain very interested in this opportunity and look forward to hearing about next steps.\n\nBest regards,\n[Your name]`;
  }

  function openDraft() {
    draftText = record.debrief_result?.followup_email_draft ?? buildLocalTemplate();
    draftCopied = false;
    aiDraftError = '';
    showDraft = true;
  }

  function copyDraft() {
    navigator.clipboard.writeText(draftText);
    draftCopied = true;
    setTimeout(() => { draftCopied = false; }, 2000);
  }

  async function aiPersonalize() {
    aiDrafting = true;
    aiDraftError = '';
    try {
      const resp = await authFetch('/api/draft-followup', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          summary: record.summary,
          strong_points: record.strong_points,
          improvement_areas: record.improvement_areas,
          interviewer_names: loadInterviewerNames(),
        }),
      });
      if (resp.ok) {
        const data = await resp.json();
        draftText = data.email ?? draftText;
      } else {
        aiDraftError = 'Could not reach AI — edit the draft manually.';
      }
    } catch {
      aiDraftError = 'Connection error — edit the draft manually.';
    }
    aiDrafting = false;
  }

  function downloadTranscript() {
    const blob = new Blob([record.transcript!], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `transcript-${record.date.replace(/[\/,: ]+/g, '-')}.txt`;
    a.click();
    URL.revokeObjectURL(url);
  }

  function handleDelete() {
    onDelete(record.id);
    onClose();
  }

  function handleRehearsal() {
    const questions = record.rehearsal_questions?.length
      ? record.rehearsal_questions
      : record.improvement_areas.map(a => `Practice: ${a}`);
    onRehearsal(questions);
    onClose();
  }
</script>

<div class="modal-backdrop" onclick={onClose} role="none">
  <div class="modal" onclick={(e) => e.stopPropagation()} role="dialog">

    <div class="modal-header">
      <div class="header-meta">
        <h2>Session Report</h2>
        <span class="header-date">{record.date}</span>
      </div>
      <button class="close-btn" onclick={onClose}>✕</button>
    </div>

    <div class="modal-body">

      <section>
        <h3>Overall</h3>
        <p class="summary">{record.summary}</p>
      </section>

      <div class="two-col">
        {#if record.strong_points.length}
          <section>
            <h3 class="green">Strong Moments</h3>
            <ul>
              {#each record.strong_points as p}<li>{p}</li>{/each}
            </ul>
          </section>
        {/if}
        {#if record.improvement_areas.length}
          <section>
            <h3 class="yellow">Areas to Improve</h3>
            <ul>
              {#each record.improvement_areas as p}<li>{p}</li>{/each}
            </ul>
          </section>
        {/if}
      </div>

      {#if record.debrief_result?.followup_email_draft && !showDraft}
        <section class="email-section">
          <h3 class="blue">Follow-Up Email</h3>
          <div class="email-draft">
            {#each record.debrief_result.followup_email_draft.split('\n') as line}
              {#if line.trim() === ''}
                <div class="email-blank"></div>
              {:else}
                <div class="email-line">{line}</div>
              {/if}
            {/each}
          </div>
        </section>
      {:else if record.debrief_result?.followup_email?.length && !showDraft}
        <section class="email-section">
          <h3 class="blue">Follow-Up Email Points</h3>
          <ul>
            {#each record.debrief_result.followup_email as p}<li>{p}</li>{/each}
          </ul>
        </section>
      {/if}

      {#if showDraft}
        <section class="draft-section">
          <div class="draft-header">
            <h3 class="blue">Follow-Up Email Draft</h3>
            <button class="draft-close" onclick={() => showDraft = false}>✕</button>
          </div>
          <textarea class="draft-textarea" bind:value={draftText} rows="9"></textarea>
          <div class="draft-actions">
            <button class="copy-btn" class:copied={draftCopied} onclick={copyDraft}>
              {draftCopied ? '✓ Copied!' : 'Copy'}
            </button>
            <button class="ai-btn" onclick={aiPersonalize} disabled={aiDrafting}>
              {aiDrafting ? 'Generating…' : 'Personalize with AI ▸'}
            </button>
          </div>
          {#if aiDraftError}
            <div class="draft-error">{aiDraftError}</div>
          {/if}
        </section>
      {/if}

    </div>

    <div class="modal-footer">
      <button class="rehearse-btn" onclick={handleRehearsal}>Practice weak answers</button>
      <button class="followup-btn" onclick={openDraft}>Draft Follow-Up</button>
      {#if record.transcript}
        <button class="transcript-btn" onclick={downloadTranscript}>↓ Transcript</button>
      {/if}
      <button class="delete-btn" onclick={handleDelete}>Delete</button>
    </div>

  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed; inset: 0; background: rgba(0,0,0,0.75);
    display: flex; align-items: center; justify-content: center;
    z-index: 200;
  }
  .modal {
    background: #0f172a; border: 1px solid #1e293b; border-radius: 0.75rem;
    width: min(680px, 95vw); max-height: 88vh; display: flex; flex-direction: column;
  }
  .modal-header {
    display: flex; align-items: flex-start; justify-content: space-between;
    padding: 1rem 1.5rem; border-bottom: 1px solid #1e293b; flex-shrink: 0;
  }
  .header-meta { display: flex; flex-direction: column; gap: 0.15rem; }
  h2 { font-size: 1.1rem; font-weight: 700; color: #f1f5f9; margin: 0; }
  .header-date { font-size: var(--fs-xs); color: #475569; }
  .close-btn {
    background: none; border: none; color: #64748b; font-size: 1rem;
    cursor: pointer; padding: 0.2rem 0.4rem; flex-shrink: 0;
  }
  .close-btn:hover { color: #e2e8f0; }

  .modal-body { overflow-y: auto; padding: 1.5rem; display: flex; flex-direction: column; gap: 1.5rem; }

  section { display: flex; flex-direction: column; gap: 0.5rem; }
  h3 { font-size: var(--fs-sm); font-weight: 700; text-transform: uppercase; letter-spacing: 0.07em; color: #64748b; margin: 0; }
  h3.green { color: #4ade80; }
  h3.yellow { color: #f59e0b; }
  h3.blue { color: #93c5fd; }
  .summary { color: #cbd5e1; line-height: 1.6; font-size: var(--fs-base); margin: 0; }
  ul { margin: 0; padding-left: 1.25rem; display: flex; flex-direction: column; gap: 0.3rem; }
  li { color: #94a3b8; font-size: 0.875rem; line-height: 1.5; }
  .two-col { display: grid; grid-template-columns: 1fr 1fr; gap: 1.5rem; }

  .email-section { gap: 0.5rem; }
  .email-draft {
    background: #060e1a; border: 1px solid #1a2d4a; border-radius: 0.5rem;
    padding: 1rem 1.25rem; font-family: 'Georgia', serif;
  }
  .email-line { font-size: var(--fs-base); color: #cbd5e1; line-height: 1.7; }
  .email-blank { height: 0.7rem; }

  .draft-section { display: flex; flex-direction: column; gap: 0.5rem; background: #060e1a; border: 1px solid #1a2d4a; border-radius: 0.5rem; padding: 0.85rem 1rem; }
  .draft-header { display: flex; align-items: center; justify-content: space-between; }
  .draft-close { background: none; border: none; color: #475569; font-size: var(--fs-sm); cursor: pointer; padding: 0; }
  .draft-close:hover { color: #94a3b8; }
  .draft-textarea { background: #0a1628; border: 1px solid #1e293b; border-radius: 0.3rem; color: #94a3b8; font-size: var(--fs-sm); line-height: 1.55; padding: 0.5rem 0.65rem; resize: vertical; font-family: inherit; width: 100%; box-sizing: border-box; }
  .draft-textarea:focus { outline: none; border-color: #334155; }
  .draft-actions { display: flex; gap: 0.4rem; align-items: center; }
  .copy-btn { padding: 0.25rem 0.7rem; background: transparent; border: 1px solid #334155; border-radius: 0.25rem; color: #64748b; font-size: var(--fs-sm); cursor: pointer; transition: all 0.15s; }
  .copy-btn:hover { border-color: #60a5fa; color: #60a5fa; }
  .copy-btn.copied { border-color: #4ade80; color: #4ade80; }
  .ai-btn { padding: 0.25rem 0.75rem; background: #1d3461; border: none; border-radius: 0.25rem; color: #93c5fd; font-size: var(--fs-sm); cursor: pointer; transition: all 0.15s; }
  .ai-btn:hover:not(:disabled) { background: #1e40af; color: #bfdbfe; }
  .ai-btn:disabled { opacity: 0.5; cursor: default; }
  .draft-error { font-size: var(--fs-xs); color: #f87171; font-style: italic; }

  .modal-footer {
    display: flex; gap: 0.5rem; align-items: center; flex-wrap: wrap;
    padding: 0.75rem 1.5rem; border-top: 1px solid #1e293b;
    flex-shrink: 0; background: #080f1c; border-radius: 0 0 0.75rem 0.75rem;
  }
  .rehearse-btn { padding: 0.3rem 0.75rem; background: #14532d; border: none; border-radius: 0.3rem; color: #4ade80; font-size: var(--fs-sm); cursor: pointer; }
  .rehearse-btn:hover { background: #166534; }
  .followup-btn { padding: 0.3rem 0.65rem; background: transparent; border: 1px solid #1e3a5f; border-radius: 0.3rem; color: #60a5fa; font-size: var(--fs-sm); cursor: pointer; transition: all 0.12s; }
  .followup-btn:hover { border-color: #3b82f6; color: #93c5fd; background: rgba(59,130,246,0.07); }
  .transcript-btn { padding: 0.3rem 0.65rem; background: transparent; border: 1px solid #1e3a5f; border-radius: 0.3rem; color: #7dd3fc; font-size: var(--fs-sm); cursor: pointer; transition: all 0.12s; }
  .transcript-btn:hover { border-color: #38bdf8; color: #e0f2fe; background: rgba(14,165,233,0.07); }
  .delete-btn { padding: 0.25rem 0.6rem; background: transparent; border: 1px solid #1e293b; border-radius: 0.3rem; color: #334155; font-size: var(--fs-sm); cursor: pointer; margin-left: auto; }
  .delete-btn:hover { border-color: #7f1d1d; color: #fca5a5; }
</style>
