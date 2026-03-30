<script lang="ts">
  import type { InterviewRecord, SavedQaEntry } from '../lib/interviewHistory';
  import { authFetch } from '../lib/api';

  const { record, onDelete, onRehearsal } = $props<{
    record: InterviewRecord;
    onDelete: (id: string) => void;
    onRehearsal: (questions: string[]) => void;
  }>();

  let showDraft = $state(false);
  let collapsedQa = $state<number[]>([]);
  function toggleQa(i: number) {
    collapsedQa = collapsedQa.includes(i) ? collapsedQa.filter(x => x !== i) : [...collapsedQa, i];
  }
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

  let copyReportState = $state<'idle' | 'copied'>('idle');
  let emailTo = $state(localStorage.getItem('session-report-email') ?? '');
  let emailSent = $state(false);

  function buildReportText(): string {
    const title = [record.company, record.role].filter(Boolean).join(' · ') || 'Interview Session';
    const lines = [
      `SESSION REPORT — ${title}`,
      record.date,
      '─'.repeat(52),
      '',
      'SUMMARY',
      record.summary,
      '',
    ];
    if (record.strong_points.length) {
      lines.push('STRONG MOMENTS');
      record.strong_points.forEach(p => lines.push(`  • ${p}`));
      lines.push('');
    }
    if (record.improvement_areas.length) {
      lines.push('AREAS TO IMPROVE');
      record.improvement_areas.forEach(p => lines.push(`  • ${p}`));
      lines.push('');
    }
    if (record.qa_entries?.length) {
      lines.push('Q&A');
      record.qa_entries.forEach((e, i) => {
        lines.push(`${i + 1}. ${e.question}`);
        if (e.candidateAnswer) lines.push(`   You said: ${e.candidateAnswer}`);
        if (e.vocalScore != null) lines.push(`   Voice score: ${e.vocalScore}%`);
        lines.push(`   Suggested: ${e.suggestion}`);
        if (e.coaching) lines.push(`   Coaching: ${e.coaching}`);
        lines.push('');
      });
    }
    return lines.join('\n');
  }

  function buildReportMarkdown(): string {
    const title = [record.company, record.role].filter(Boolean).join(' · ') || 'Interview Session';
    const lines = [
      `# Session Report: ${title}`,
      `*${record.date}*`,
      '',
      '## Summary',
      record.summary,
      '',
    ];
    if (record.strong_points.length) {
      lines.push('## Strong Moments');
      record.strong_points.forEach(p => lines.push(`- ${p}`));
      lines.push('');
    }
    if (record.improvement_areas.length) {
      lines.push('## Areas to Improve');
      record.improvement_areas.forEach(p => lines.push(`- ${p}`));
      lines.push('');
    }
    if (record.qa_entries?.length) {
      lines.push('## Q&A');
      record.qa_entries.forEach((e, i) => {
        lines.push(`### ${i + 1}. ${e.question}`);
        if (e.candidateAnswer) lines.push(`**You said:** ${e.candidateAnswer}\n`);
        lines.push(`**Suggested:** ${e.suggestion}`);
        if (e.coaching) lines.push(`\n*Coaching: ${e.coaching}*`);
        lines.push('');
      });
    }
    return lines.join('\n');
  }

  function downloadReport() {
    const text = buildReportText();
    const blob = new Blob([text], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `session-report-${record.date.replace(/[\/,: ]+/g, '-')}.txt`;
    a.click();
    URL.revokeObjectURL(url);
  }

  async function copyMarkdown() {
    await navigator.clipboard.writeText(buildReportMarkdown());
    copyReportState = 'copied';
    setTimeout(() => { copyReportState = 'idle'; }, 2000);
  }

  function sendEmail() {
    if (!emailTo.trim()) return;
    localStorage.setItem('session-report-email', emailTo.trim());
    const text = buildReportText();
    const MAX = 1600;
    const body = text.length > MAX ? text.slice(0, MAX) + '\n\n[Truncated]' : text;
    const title = [record.company, record.role].filter(Boolean).join(' · ') || 'Interview Session';
    const sub = encodeURIComponent(`Session Report — ${title}`);
    window.open(`mailto:${encodeURIComponent(emailTo.trim())}?subject=${sub}&body=${encodeURIComponent(body)}`.slice(0, 2000), '_self');
    emailSent = true;
    setTimeout(() => { emailSent = false; }, 3000);
  }
</script>

<div class="detail-view">
  <div class="detail-body">

    <section>
      <h3>Overall</h3>
      <p class="summary">{record.summary}</p>
    </section>

    <div class="two-col">
      {#if record.strong_points.length}
        <section>
          <h3 class="green">Strong Moments</h3>
          <ul>{#each record.strong_points as p}<li>{p}</li>{/each}</ul>
        </section>
      {/if}
      {#if record.improvement_areas.length}
        <section>
          <h3 class="yellow">Areas to Improve</h3>
          <ul>{#each record.improvement_areas as p}<li>{p}</li>{/each}</ul>
        </section>
      {/if}
    </div>

    {#if record.qa_entries && record.qa_entries.length > 0}
      <section>
        <h3 class="blue">Q&amp;A ({record.qa_entries.length})</h3>
        <div class="qa-list">
          {#each record.qa_entries as entry, i}
            {@const collapsed = collapsedQa.includes(i)}
            <div class="qa-item">
              <button class="qa-header" onclick={() => toggleQa(i)}>
                <span class="qa-num">{i + 1}</span>
                <span class="qa-q">{entry.question}</span>
                {#if entry.confidenceScore != null}
                  <span class="qa-score" class:score-high={entry.confidenceScore >= 70} class:score-mid={entry.confidenceScore >= 40 && entry.confidenceScore < 70} class:score-low={entry.confidenceScore < 40}>
                    {entry.confidenceScore}%
                  </span>
                {/if}
                {#if entry.tag}
                  <span class="qa-tag">{entry.tag}</span>
                {/if}
                <span class="qa-chevron">{collapsed ? '▸' : '▾'}</span>
              </button>
              {#if !collapsed}
                <div class="qa-body">
                  <div class="qa-col">
                    <div class="qa-col-label green">Your answer</div>
                    {#if entry.candidateAnswer}
                      <div class="qa-answer">{entry.candidateAnswer}</div>
                    {:else}
                      <div class="qa-no-answer">No answer captured</div>
                    {/if}
                    {#if entry.vocalScore != null || entry.vocalTone || entry.vocalCoaching}
                      <div class="qa-vocal-row">
                        {#if entry.vocalScore != null}
                          <span class="qa-vocal-score" class:score-high={entry.vocalScore >= 70} class:score-mid={entry.vocalScore >= 40 && entry.vocalScore < 70} class:score-low={entry.vocalScore < 40}>{entry.vocalScore}% voice</span>
                        {/if}
                        {#if entry.vocalTone}<span class="qa-vocal-chip">{entry.vocalTone}</span>{/if}
                        {#if entry.vocalPace}<span class="qa-vocal-chip">{entry.vocalPace} pace</span>{/if}
                        {#if entry.vocalFillers}<span class="qa-vocal-fillers">Fillers: {entry.vocalFillers}</span>{/if}
                      </div>
                      {#if entry.vocalCoaching}
                        <div class="qa-coaching">{entry.vocalCoaching}</div>
                      {/if}
                    {/if}
                  </div>
                  <div class="qa-col">
                    <div class="qa-col-label blue">Suggested answer</div>
                    <div class="qa-suggestion">{entry.suggestion}</div>
                    {#if entry.coaching && entry.coaching !== entry.suggestion}
                      <div class="qa-coaching">{entry.coaching}</div>
                    {/if}
                  </div>
                </div>
              {/if}
            </div>
          {/each}
        </div>
      </section>
    {/if}

    {#if record.debrief_result?.followup_email_draft && !showDraft}
      <section>
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
      <section>
        <h3 class="blue">Follow-Up Email Points</h3>
        <ul>{#each record.debrief_result.followup_email as p}<li>{p}</li>{/each}</ul>
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

  <div class="detail-footer">
    <div class="footer-row">
      <button class="rehearse-btn" onclick={() => onRehearsal(record.rehearsal_questions?.length ? record.rehearsal_questions : record.improvement_areas.map(a => `Practice: ${a}`))}>Practice weak answers</button>
      <button class="followup-btn" onclick={openDraft}>Draft Follow-Up</button>
      <button class="delete-btn" onclick={() => onDelete(record.id)}>Delete</button>
    </div>
    <div class="footer-row share-row">
      <button class="share-btn" onclick={downloadReport}>↓ Report</button>
      <button class="share-btn" class:copied={copyReportState === 'copied'} onclick={copyMarkdown}>
        {copyReportState === 'copied' ? '✓ Copied!' : 'Copy Markdown'}
      </button>
      {#if record.transcript}
        <button class="share-btn" onclick={downloadTranscript}>↓ Transcript</button>
      {/if}
      <div class="email-row">
        <input class="email-input" type="email" placeholder="Email myself…" bind:value={emailTo} onkeydown={(e) => { if (e.key === 'Enter') sendEmail(); }} />
        <button class="send-btn" class:sent={emailSent} onclick={sendEmail} disabled={!emailTo.trim()}>{emailSent ? '✓' : '✉'}</button>
      </div>
    </div>
  </div>
</div>

<style>
  .detail-view { display: flex; flex-direction: column; flex: 1; min-height: 0; }
  .detail-body { flex: 1; overflow-y: auto; padding: 1.25rem 1.5rem; display: flex; flex-direction: column; gap: 1.5rem; min-height: 0; }
  .detail-footer {
    flex-shrink: 0; border-top: 1px solid #1e293b;
    padding: 0.6rem 1.5rem 0.75rem; display: flex; flex-direction: column; gap: 0.45rem;
    background: #080f1c;
  }

  section { display: flex; flex-direction: column; gap: 0.5rem; }
  h3 { font-size: var(--fs-sm); font-weight: 700; text-transform: uppercase; letter-spacing: 0.07em; color: #64748b; margin: 0; }
  h3.green { color: #4ade80; }
  h3.yellow { color: #f59e0b; }
  h3.blue { color: #93c5fd; }
  .summary { color: #cbd5e1; line-height: 1.6; font-size: var(--fs-base); margin: 0; }
  ul { margin: 0; padding-left: 1.25rem; display: flex; flex-direction: column; gap: 0.3rem; }
  li { color: #94a3b8; font-size: 0.875rem; line-height: 1.5; }
  .two-col { display: grid; grid-template-columns: 1fr 1fr; gap: 1.5rem; }

  .qa-list { display: flex; flex-direction: column; gap: 0.5rem; }
  .qa-item { background: #060e1a; border: 1px solid #1a2d4a; border-radius: 0.4rem; padding: 0.65rem 0.85rem; display: flex; flex-direction: column; gap: 0.35rem; }
  .qa-header { display: flex; align-items: baseline; gap: 0.45rem; flex-wrap: wrap; width: 100%; background: none; border: none; padding: 0; cursor: pointer; text-align: left; }
  .qa-header:hover .qa-q { color: #bfdbfe; }
  .qa-chevron { font-size: var(--fs-xs); color: #334155; margin-left: auto; flex-shrink: 0; }
  .qa-num { font-size: var(--fs-xs); color: #334155; font-weight: 700; background: #0d1f35; border: 1px solid #1e3a5f; border-radius: 0.2rem; padding: 0 0.3em; line-height: 1.6; flex-shrink: 0; }
  .qa-q { font-size: var(--fs-sm); font-weight: 600; color: #93c5fd; line-height: 1.4; flex: 1; }
  .qa-score { font-size: var(--fs-xs); font-weight: 700; border-radius: 0.25rem; padding: 0.05rem 0.4rem; flex-shrink: 0; }
  .qa-score.score-high { color: #4ade80; background: #052e16; }
  .qa-score.score-mid { color: #fbbf24; background: #1c1000; }
  .qa-score.score-low { color: #f87171; background: #1c0a0a; }
  .qa-tag { font-size: var(--fs-xs); color: #475569; background: #0f172a; border: 1px solid #1e293b; border-radius: 0.2rem; padding: 0.05rem 0.4rem; flex-shrink: 0; text-transform: capitalize; }
  .qa-body { display: grid; grid-template-columns: 1fr 1fr; gap: 0; border-top: 1px solid #0f1f35; margin-top: 0.25rem; }
  .qa-col { padding: 0.55rem 0.65rem; display: flex; flex-direction: column; gap: 0.3rem; }
  .qa-col:first-child { border-right: 1px solid #0f1f35; }
  .qa-col-label { font-size: var(--fs-xs); font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em; }
  .qa-col-label.green { color: #4ade80; }
  .qa-col-label.blue { color: #60a5fa; }
  .qa-answer { font-size: var(--fs-sm); color: #cbd5e1; line-height: 1.55; white-space: pre-wrap; }
  .qa-no-answer { font-size: var(--fs-xs); color: #334155; font-style: italic; }
  .qa-vocal-row { display: flex; gap: 0.35rem; align-items: center; flex-wrap: wrap; }
  .qa-vocal-score { font-size: var(--fs-xs); font-weight: 700; border-radius: 0.25rem; padding: 0.05rem 0.4rem; }
  .qa-vocal-score.score-high { color: #4ade80; background: #052e16; }
  .qa-vocal-score.score-mid { color: #fbbf24; background: #1c1000; }
  .qa-vocal-score.score-low { color: #f87171; background: #1c0a0a; }
  .qa-vocal-chip { font-size: var(--fs-xs); color: #7dd3fc; background: #0c1f35; border: 1px solid #1e3a5f; border-radius: 0.2rem; padding: 0.05rem 0.35rem; }
  .qa-vocal-fillers { font-size: var(--fs-xs); color: #f59e0b; font-style: italic; }
  .qa-suggestion { font-size: var(--fs-sm); color: #94a3b8; line-height: 1.55; white-space: pre-wrap; }
  .qa-coaching { font-size: var(--fs-xs); color: #64748b; line-height: 1.5; border-left: 2px solid #1e3a5f; padding-left: 0.5rem; font-style: italic; }

  .email-draft { background: #060e1a; border: 1px solid #1a2d4a; border-radius: 0.5rem; padding: 1rem 1.25rem; font-family: 'Georgia', serif; }
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

  .footer-row { display: flex; gap: 0.5rem; align-items: center; flex-wrap: wrap; }
  .share-row { padding-top: 0.45rem; border-top: 1px solid #1a2d4a; }
  .rehearse-btn { padding: 0.3rem 0.75rem; background: #14532d; border: none; border-radius: 0.3rem; color: #4ade80; font-size: var(--fs-sm); cursor: pointer; }
  .rehearse-btn:hover { background: #166534; }
  .followup-btn { padding: 0.3rem 0.65rem; background: transparent; border: 1px solid #1e3a5f; border-radius: 0.3rem; color: #60a5fa; font-size: var(--fs-sm); cursor: pointer; transition: all 0.12s; }
  .followup-btn:hover { border-color: #3b82f6; color: #93c5fd; background: rgba(59,130,246,0.07); }
  .delete-btn { padding: 0.25rem 0.6rem; background: transparent; border: 1px solid #1e293b; border-radius: 0.3rem; color: #334155; font-size: var(--fs-sm); cursor: pointer; margin-left: auto; }
  .delete-btn:hover { border-color: #7f1d1d; color: #fca5a5; }
  .share-btn { padding: 0.25rem 0.6rem; background: transparent; border: 1px solid #334155; border-radius: 0.3rem; color: #64748b; font-size: var(--fs-xs); cursor: pointer; text-decoration: none; transition: all 0.15s; white-space: nowrap; }
  .share-btn:hover { border-color: #60a5fa; color: #60a5fa; }
  .share-btn.copied { border-color: #4ade80; color: #4ade80; }
  .email-row { display: flex; align-items: center; gap: 0.3rem; flex: 1; min-width: 140px; }
  .email-input { flex: 1; padding: 0.25rem 0.5rem; background: #0f172a; border: 1px solid #1e293b; border-radius: 0.3rem; color: #e2e8f0; font-size: var(--fs-xs); outline: none; transition: border-color 0.15s; }
  .email-input:focus { border-color: #3b82f6; }
  .send-btn { padding: 0.25rem 0.5rem; background: #1d4ed8; border: none; border-radius: 0.3rem; color: white; font-size: 0.8rem; cursor: pointer; transition: background 0.15s; }
  .send-btn:hover:not(:disabled) { background: #2563eb; }
  .send-btn:disabled { opacity: 0.4; cursor: default; }
  .send-btn.sent { background: #166534; }
</style>
