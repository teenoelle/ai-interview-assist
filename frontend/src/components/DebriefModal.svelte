<script lang="ts">
  import type { TranscriptEntry, SuggestionEntry, DebriefResult, PracticeAnswer } from '../lib/types';
  import { TAG_CONFIG } from '../lib/questionTagger';

  const { transcript = [], suggestions = [], onClose, onSave, recordingUrl, prefilled, practiceAnswers = [] } = $props<{
    transcript?: TranscriptEntry[];
    suggestions?: SuggestionEntry[];
    onClose: () => void;
    onSave?: (result: DebriefResult) => void;
    recordingUrl?: string;
    prefilled?: DebriefResult;
    practiceAnswers?: PracticeAnswer[];
  }>();

  type Tab = 'review' | 'timeline' | 'practice' | 'recording';
  let activeTab = $state<Tab>('review');

  let loading = $state(!prefilled);
  let result = $state<DebriefResult | null>(prefilled ?? null);
  let error = $state('');
  let copied = $state(false);
  let saved = $state(false);
  let emailTo = $state(localStorage.getItem('debrief-email') ?? '');
  let emailSent = $state(false);
  let nextSteps = $state<string[]>([]);
  let loadingNextSteps = $state(false);

  // ── Timeline Q&A ─────────────────────────────────────────────────────────────

  interface DebriefQaEntry {
    question: string;
    tag?: string;
    suggestion: string;
    answerText: string;
    coaching: string;
    missedFollowup: boolean;
    missedMetric: boolean;
    wpm?: number;
    confidenceScore?: number;
    loadingCoaching: boolean;
    suggestionOpen: boolean;
  }

  let debriefQa = $state<DebriefQaEntry[]>([]);

  const simulatedSuggestions = $derived(
    suggestions
      .filter(s => s.question && s.suggestion && s.source === 'simulated')
      .sort((a, b) => (a.detectedAt ?? 0) - (b.detectedAt ?? 0))
  );

  const totalPracticeCount = $derived(simulatedSuggestions.length + practiceAnswers.length);

  function extractAnswer(detectedAt: number, nextDetectedAt: number): string {
    return transcript
      .filter(e =>
        e.speaker === 'You' &&
        e.timestamp_ms > detectedAt &&
        (nextDetectedAt === 0 || e.timestamp_ms < nextDetectedAt)
      )
      .map(e => e.text)
      .join(' ')
      .trim();
  }

  function initDebriefQa() {
    const sorted = [...suggestions]
      .filter(s => s.question && s.suggestion && s.source !== 'simulated')
      .sort((a, b) => (a.detectedAt ?? 0) - (b.detectedAt ?? 0));

    debriefQa = sorted.map((s, i) => {
      const nextAt = sorted[i + 1]?.detectedAt ?? 0;
      const answerText = extractAnswer(s.detectedAt ?? 0, nextAt);
      const hasCoaching = !!s.answerFeedback;
      return {
        question: s.question,
        tag: s.tag,
        suggestion: s.suggestion,
        answerText,
        coaching: s.answerFeedback?.coaching ?? '',
        missedFollowup: s.answerFeedback?.missed_followup ?? false,
        missedMetric: s.answerFeedback?.missed_metric ?? false,
        wpm: s.vocalFeedback?.confidence_score ? undefined : undefined,
        confidenceScore: s.vocalFeedback?.confidence_score,
        loadingCoaching: !hasCoaching && !!answerText,
        suggestionOpen: false,
      };
    });
  }

  async function fetchMissingCoaching() {
    for (let i = 0; i < debriefQa.length; i++) {
      const entry = debriefQa[i];
      if (!entry.loadingCoaching) continue;
      try {
        const resp = await fetch('/api/answer-feedback', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ question: entry.question, answer: entry.answerText, suggestion: entry.suggestion }),
        });
        if (resp.ok) {
          const data = await resp.json();
          debriefQa[i] = { ...debriefQa[i], coaching: data.coaching, missedFollowup: data.missed_followup, missedMetric: data.missed_metric, loadingCoaching: false };
        } else {
          debriefQa[i] = { ...debriefQa[i], loadingCoaching: false };
        }
      } catch {
        debriefQa[i] = { ...debriefQa[i], loadingCoaching: false };
      }
      debriefQa = [...debriefQa];
    }
  }

  // ── Debrief fetch ─────────────────────────────────────────────────────────────

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

  if (!prefilled) {
    fetchDebrief();
    initDebriefQa();
    fetchMissingCoaching();
  }

  // ── Email / Save ──────────────────────────────────────────────────────────────

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
    if (debriefQa.length > 0) {
      lines.push('Q&A COACHING');
      lines.push('------------', '');
      debriefQa.forEach((qa, i) => {
        lines.push(`Q${i + 1}: ${qa.question}`);
        if (qa.answerText) lines.push(`Your answer: ${qa.answerText}`);
        if (qa.coaching) lines.push(`Coaching: ${qa.coaching}`);
        const flags: string[] = [];
        if (qa.missedFollowup) flags.push('Missing follow-up');
        if (qa.missedMetric) flags.push('Missing metric');
        if (flags.length) lines.push(`Watch: ${flags.join(' · ')}`);
        if (qa.confidenceScore) lines.push(`Confidence: ${qa.confidenceScore}%`);
        lines.push('');
      });
    }
    if (simulatedSuggestions.length > 0 || practiceAnswers.length > 0) {
      lines.push('PRACTICE ANSWERS');
      lines.push('----------------', '');
      simulatedSuggestions.forEach((s, i) => {
        lines.push(`[Practice] Q${i + 1}: ${s.question}`);
        if (s.suggestion) lines.push(`AI Suggestion: ${s.suggestion}`);
        if (s.answerFeedback?.coaching) lines.push(`Coaching: ${s.answerFeedback.coaching}`);
        lines.push('');
      });
      practiceAnswers.forEach((a, i) => {
        lines.push(`[Recorded Practice] Q${simulatedSuggestions.length + i + 1}: ${a.question}`);
        if (a.answerText) lines.push(`Your answer: ${a.answerText}`);
        if (a.score !== undefined) lines.push(`Score: ${a.score}/100`);
        if (a.coaching) lines.push(`Coaching: ${a.coaching}`);
        if (a.vocalTone) lines.push(`Vocal tone: ${a.vocalTone}${a.vocalConfidence !== undefined ? ` (${a.vocalConfidence}%)` : ''}`);
        lines.push('');
      });
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
      {#if !prefilled}
        <button class="tab" class:active={activeTab === 'timeline'} onclick={() => activeTab = 'timeline'}>
          Timeline {debriefQa.length > 0 ? `(${debriefQa.length})` : ''}
        </button>
        <button class="tab" class:active={activeTab === 'practice'} onclick={() => activeTab = 'practice'}>
          Practice {totalPracticeCount > 0 ? `(${totalPracticeCount})` : ''}
        </button>
        <button class="tab" class:active={activeTab === 'recording'} onclick={() => activeTab = 'recording'}>
          Recording
        </button>
      {/if}
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

          {#if !prefilled}
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
          {/if}

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

      <!-- Timeline tab -->
      {#if activeTab === 'timeline'}
        <div class="timeline">
          {#if debriefQa.length === 0}
            <p class="steps-empty">No questions detected this session.</p>
          {:else}
            {#each debriefQa as qa, i}
              <div class="tl-item">
                <!-- Question -->
                <div class="tl-q-row">
                  {#if qa.tag}
                    {@const tc = TAG_CONFIG[qa.tag]}
                    <span class="tl-tag" style="color: {tc.color}; background: {tc.bg}">{tc.label}</span>
                  {/if}
                  <span class="tl-q-num">{i + 1}</span>
                  <span class="tl-q-text">"{qa.question}"</span>
                </div>

                <!-- Candidate answer -->
                {#if qa.answerText}
                  <div class="tl-answer">
                    <span class="tl-answer-label">You</span>
                    <span class="tl-answer-text">{qa.answerText}</span>
                  </div>
                {/if}

                <!-- Coaching card -->
                <div class="tl-coaching">
                  {#if qa.loadingCoaching}
                    <span class="tl-coaching-loading">Analyzing answer…</span>
                  {:else}
                    {#if qa.coaching}
                      <p class="tl-coaching-note">{qa.coaching}</p>
                    {/if}
                    {#if qa.missedFollowup || qa.missedMetric || qa.confidenceScore}
                      <div class="tl-flags">
                        {#if qa.missedMetric}
                          <span class="tl-flag tl-flag-warn">Missing metric</span>
                        {/if}
                        {#if qa.missedFollowup}
                          <span class="tl-flag tl-flag-warn">Missing follow-up</span>
                        {/if}
                        {#if qa.confidenceScore && qa.confidenceScore >= 70}
                          <span class="tl-flag tl-flag-good">Strong answer</span>
                        {/if}
                      </div>
                    {/if}
                  {/if}

                  <!-- AI suggestion toggle -->
                  {#if qa.suggestion}
                    <button
                      class="tl-suggestion-toggle"
                      onclick={() => {
                        debriefQa[i] = { ...debriefQa[i], suggestionOpen: !debriefQa[i].suggestionOpen };
                        debriefQa = [...debriefQa];
                      }}
                    >
                      {debriefQa[i].suggestionOpen ? '▾' : '▸'} AI Suggestion
                    </button>
                    {#if debriefQa[i].suggestionOpen}
                      <div class="tl-suggestion-text">{qa.suggestion}</div>
                    {/if}
                  {/if}
                </div>
              </div>
            {/each}
          {/if}
        </div>
      {/if}

      <!-- Practice tab -->
      {#if activeTab === 'practice'}
        <div class="timeline">
          {#if totalPracticeCount === 0}
            <p class="steps-empty">No practice questions this session. Click questions in Example Questions or Practice Questions during the interview to practice here.</p>
          {:else}
            {#if simulatedSuggestions.length > 0}
              <div class="practice-section-label">Example & Predicted Questions</div>
              {#each simulatedSuggestions as s, i}
                <div class="tl-item tl-item-practice">
                  <div class="tl-q-row">
                    {#if s.tag}
                      {@const tc = TAG_CONFIG[s.tag]}
                      <span class="tl-tag" style="color: {tc.color}; background: {tc.bg}">{tc.label}</span>
                    {/if}
                    <span class="tl-q-num">{i + 1}</span>
                    <span class="tl-q-text">"{s.question}"</span>
                    <span class="tl-practice-badge">Practice</span>
                  </div>
                  {#if s.suggestion}
                    <div class="tl-coaching">
                      <p class="tl-coaching-note tl-suggestion-text">{s.suggestion}</p>
                      {#if s.answerFeedback?.coaching}
                        <p class="tl-coaching-note" style="margin-top: 0.4rem">{s.answerFeedback.coaching}</p>
                      {/if}
                    </div>
                  {/if}
                </div>
              {/each}
            {/if}
            {#if practiceAnswers.length > 0}
              <div class="practice-section-label" style="margin-top: {simulatedSuggestions.length > 0 ? '1rem' : '0'}">Recorded Practice Answers</div>
              {#each practiceAnswers as pa, i}
                <div class="tl-item tl-item-practice">
                  <div class="tl-q-row">
                    <span class="tl-q-num">{simulatedSuggestions.length + i + 1}</span>
                    <span class="tl-q-text">"{pa.question}"</span>
                    <span class="tl-practice-badge tl-practice-badge-recorded">Recorded</span>
                  </div>
                  {#if pa.answerText}
                    <div class="tl-answer">
                      <span class="tl-answer-label">You</span>
                      <span class="tl-answer-text">{pa.answerText}</span>
                    </div>
                  {/if}
                  <div class="tl-coaching">
                    {#if pa.score !== undefined}
                      <div class="tl-flags">
                        <span class="tl-flag" style="color: {pa.score >= 70 ? '#22c55e' : pa.score >= 50 ? '#f59e0b' : '#ef4444'}; border-color: currentColor">Score: {pa.score}/100</span>
                        {#if pa.vocalTone}
                          <span class="tl-flag" style="color: #94a3b8; border-color: #334155">Tone: {pa.vocalTone}{pa.vocalConfidence !== undefined ? ` · ${pa.vocalConfidence}%` : ''}</span>
                        {/if}
                      </div>
                    {/if}
                    {#if pa.coaching}
                      <p class="tl-coaching-note">{pa.coaching}</p>
                    {/if}
                    {#if pa.strong}
                      <p class="tl-coaching-note" style="color: #4ade80">{pa.strong}</p>
                    {/if}
                  </div>
                </div>
              {/each}
            {/if}
          {/if}
        </div>
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
    color: #64748b; font-size: var(--fs-sm); cursor: pointer; transition: all 0.15s;
  }
  .copy-btn:hover { border-color: #60a5fa; color: #60a5fa; }
  .copy-btn.copied { border-color: #4ade80; color: #4ade80; }
  .email-section { gap: 0.75rem; }
  .email-draft {
    background: #060e1a; border: 1px solid #1a2d4a; border-radius: 0.5rem;
    padding: 1rem 1.25rem; font-family: 'Georgia', serif; user-select: text; cursor: text;
  }
  .email-subject { font-size: var(--fs-base); font-weight: 700; color: #93c5fd; margin-bottom: 0.5rem; padding-bottom: 0.5rem; border-bottom: 1px solid #1a2d4a; }
  .email-line { font-size: var(--fs-base); color: #cbd5e1; line-height: 1.7; }
  .email-blank { height: 0.7rem; }
  .loading { color: #60a5fa; font-style: italic; text-align: center; padding: 2rem; }
  .error { color: #fca5a5; padding: 1rem; background: #450a0a; border-radius: 0.5rem; }

  /* ── Timeline ── */
  .timeline { display: flex; flex-direction: column; gap: 1rem; }

  .tl-item {
    background: #060e1a; border: 1px solid #1a2d4a; border-radius: 0.5rem;
    padding: 0.75rem 0.9rem; display: flex; flex-direction: column; gap: 0.5rem;
  }

  .tl-q-row {
    display: flex; align-items: baseline; gap: 0.5rem; flex-wrap: wrap;
  }
  .tl-tag {
    font-size: var(--fs-xs); font-weight: 700; text-transform: uppercase;
    letter-spacing: 0.05em; padding: 0.05em 0.35em; border-radius: 0.2em; flex-shrink: 0;
  }
  .tl-q-num {
    font-size: var(--fs-xs); font-weight: 700; color: #334155;
    background: #0d1f35; border: 1px solid #1e3a5f; border-radius: 0.25rem;
    padding: 0 0.3em; line-height: 1.6; flex-shrink: 0;
  }
  .tl-q-text {
    font-size: var(--fs-sm); font-weight: 600; color: #93c5fd; line-height: 1.4;
  }

  .tl-answer {
    display: flex; gap: 0.5rem; align-items: flex-start;
    background: #0a111e; border-radius: 0.3rem; padding: 0.4rem 0.6rem;
    border-left: 2px solid #1e3a5f;
  }
  .tl-answer-label {
    font-size: var(--fs-xs); font-weight: 700; text-transform: uppercase;
    letter-spacing: 0.05em; color: #334155; flex-shrink: 0; padding-top: 0.1rem;
  }
  .tl-answer-text {
    font-size: var(--fs-sm); color: #64748b; line-height: 1.5;
  }

  .tl-coaching {
    display: flex; flex-direction: column; gap: 0.35rem;
    border-top: 1px solid #0f1e35; padding-top: 0.45rem;
  }
  .tl-coaching-loading {
    font-size: var(--fs-xs); color: #334155; font-style: italic;
  }
  .tl-coaching-note {
    font-size: var(--fs-sm); color: #94a3b8; line-height: 1.5; margin: 0;
  }
  .tl-flags {
    display: flex; gap: 0.3rem; flex-wrap: wrap;
  }
  .tl-flag {
    font-size: var(--fs-xs); font-weight: 700; text-transform: uppercase;
    letter-spacing: 0.05em; padding: 0.05em 0.4em; border-radius: 0.2em;
  }
  .tl-flag-warn { background: #431407; color: #fb923c; }
  .tl-flag-good { background: #071a0f; color: #4ade80; }

  .tl-suggestion-toggle {
    background: none; border: none; cursor: pointer; padding: 0;
    font-size: var(--fs-xs); color: #334155; text-align: left;
    font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em;
  }
  .tl-suggestion-toggle:hover { color: #475569; }
  .tl-suggestion-text {
    font-size: var(--fs-sm); color: #475569; line-height: 1.5;
    white-space: pre-wrap; font-style: italic;
    border-left: 2px solid #1e293b; padding-left: 0.6rem;
  }

  /* ── Practice tab ── */
  .tl-item-practice { border-left-color: #1e3a5f; }
  .tl-practice-badge {
    margin-left: auto; font-size: var(--fs-xs); font-weight: 700; text-transform: uppercase;
    letter-spacing: 0.05em; color: #3b82f6; background: #0c1f3a; border: 1px solid #1e3a5f;
    border-radius: 0.25em; padding: 0.05em 0.45em;
  }
  .tl-practice-badge-recorded {
    color: #a78bfa; background: #130c2a; border-color: #3b1f7a;
  }
  .practice-section-label {
    font-size: var(--fs-xs); font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em;
    color: #334155; padding: 0 0 0.25rem; border-bottom: 1px solid #0f172a; margin-bottom: 0.5rem;
  }

  /* ── Footer ── */
  .modal-footer {
    display: flex; align-items: center; gap: 0.6rem;
    padding: 0.75rem 1.5rem; border-top: 1px solid #1e293b; flex-shrink: 0;
    background: #080f1c; border-radius: 0 0 0.75rem 0.75rem;
  }
  .footer-left { display: flex; align-items: center; flex-shrink: 0; }
  .footer-email { display: flex; align-items: center; gap: 0.6rem; flex: 1; justify-content: flex-end; }
  .save-btn {
    padding: 0.3rem 1rem; background: #1d4ed8; border: none; border-radius: 0.3rem;
    color: white; font-size: var(--fs-base); font-weight: 700; cursor: pointer; transition: all 0.15s; white-space: nowrap;
  }
  .save-btn:hover { background: #2563eb; }
  .save-btn.saved { background: #166534; }
  .email-footer-label { font-size: var(--fs-sm); color: #475569; white-space: nowrap; }
  .email-input {
    flex: 1; padding: 0.3rem 0.65rem; background: #0f172a;
    border: 1px solid #1e293b; border-radius: 0.3rem; color: #e2e8f0;
    font-size: var(--fs-base); outline: none; transition: border-color 0.15s;
  }
  .email-input:focus { border-color: #3b82f6; }
  .send-btn {
    padding: 0.3rem 0.9rem; background: #1d4ed8; border: none; border-radius: 0.3rem;
    color: white; font-size: var(--fs-base); font-weight: 600; cursor: pointer; transition: all 0.15s; white-space: nowrap;
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
</style>
