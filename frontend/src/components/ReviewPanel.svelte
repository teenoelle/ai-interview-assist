<script lang="ts">
  export interface ReviewSummary {
    id: string;
    created_at: number;
    duration_secs: number;
    source_filename: string;
    source_type: string;
    qa_count: number;
    avg_wpm: number;
    you_pct: number;
  }

  interface SentimentEvent {
    timestamp_ms: number;
    emotion: string;
    reason?: string;
    coaching?: string;
  }

  export interface ReviewReport {
    id: string;
    created_at: number;
    duration_secs: number;
    source_filename: string;
    source_type: string;
    transcript: ReviewSegment[];
    qa_pairs: QaPair[];
    vocal_summary: { avg_wpm: number; total_answers: number };
    speaker_summary: { you_pct: number; them_pct: number; you_word_count: number; them_word_count: number; turn_count: number };
    keywords_mentioned: string[];
    sentiment_events?: SentimentEvent[];
  }

  interface ReviewSegment {
    speaker: string;
    text: string;
    start_ms: number;
    end_ms: number;
  }

  interface QaPair {
    question: string;
    answer_text: string;
    coaching: string;
    missed_followup: boolean;
    missed_metric: boolean;
    wpm: number;
    duration_secs: number;
    start_ms: number;
  }

  const { report, onClose, onDelete, onPractice } = $props<{
    report: ReviewReport;
    onClose: () => void;
    onDelete?: (id: string) => void;
    onPractice?: (question: string) => void;
  }>();

  let mediaEl = $state<HTMLVideoElement | HTMLAudioElement | null>(null);
  let currentMs = $state(0);
  let qaListEl = $state<HTMLDivElement | null>(null);
  let prevVisibleCount = $state(0);
  let copyMdState = $state<'idle' | 'copied'>('idle');
  let emailTo = $state(localStorage.getItem('review-email') ?? '');
  let emailSent = $state(false);
  let confirmDelete = $state(false);
  let coachingExpanded = $state(false);

  const isVideo = $derived(['mp4','webm','mov','mkv','avi'].some(
    ext => report.source_filename.toLowerCase().endsWith(`.${ext}`)
  ));

  const sourceUrl = $derived(`/api/review/${report.id}/source`);
  const downloadUrl = $derived(`/api/review/${report.id}/download`);

  function fmtDuration(secs: number): string {
    const m = Math.floor(secs / 60);
    const s = Math.floor(secs % 60);
    return m > 0 ? `${m}:${s.toString().padStart(2, '0')}` : `${s}s`;
  }

  function fmtDate(unixMs: number): string {
    return new Date(unixMs).toLocaleString(undefined, {
      month: 'short', day: 'numeric', year: 'numeric',
      hour: 'numeric', minute: '2-digit',
    });
  }

  function fmtWpm(wpm: number): string {
    if (wpm < 90) return 'slow';
    if (wpm > 180) return 'fast';
    return 'good';
  }

  function wpmColor(wpm: number): string {
    if (wpm < 90) return '#f59e0b';
    if (wpm > 180) return '#f87171';
    return '#4ade80';
  }

  function qaGrade(qa: QaPair): 'A' | 'B' | 'C' | 'D' {
    let strikes = 0;
    if (qa.wpm < 90 || qa.wpm > 190) strikes++;
    if (qa.missed_followup) strikes++;
    if (qa.missed_metric) strikes++;
    if (strikes === 0) return 'A';
    if (strikes === 1) return 'B';
    if (strikes === 2) return 'C';
    return 'D';
  }

  function gradeColor(grade: 'A' | 'B' | 'C' | 'D'): string {
    if (grade === 'A') return '#4ade80';
    if (grade === 'B') return '#60a5fa';
    if (grade === 'C') return '#f59e0b';
    return '#f87171';
  }

  // Progressive Q&A reveal
  const visibleQa = $derived(
    currentMs > 0
      ? report.qa_pairs.filter(qa => currentMs >= qa.start_ms)
      : report.qa_pairs
  );

  $effect(() => {
    const count = visibleQa.length;
    if (currentMs > 0 && count > prevVisibleCount && qaListEl) {
      const cards = qaListEl.querySelectorAll('.qa-card');
      cards[count - 1]?.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
    }
    prevVisibleCount = count;
  });

  const activeSegIdx = $derived(() => {
    const ms = currentMs;
    for (let i = report.transcript.length - 1; i >= 0; i--) {
      if (report.transcript[i].start_ms <= ms) return i;
    }
    return -1;
  });

  const currentSentiment = $derived(() => {
    const events = report.sentiment_events ?? [];
    if (!events.length || currentMs === 0) return null;
    for (let i = events.length - 1; i >= 0; i--) {
      if (events[i].timestamp_ms <= currentMs) return events[i];
    }
    return null;
  });

  const EMOTION_COLOR: Record<string, string> = {
    engaged: '#4ade80', curious: '#60a5fa', pleased: '#34d399',
    neutral: '#94a3b8', skeptical: '#f59e0b', confused: '#fb923c', bored: '#f87171',
  };

  function seekTo(ms: number) {
    if (!mediaEl) return;
    mediaEl.currentTime = ms / 1000;
    mediaEl.play();
  }

  function onTimeUpdate() {
    if (mediaEl) currentMs = mediaEl.currentTime * 1000;
  }

  // ── Export helpers ─────────────────────────────────────────────────────────

  function buildPrepText(): string {
    const lines: string[] = [
      `INTERVIEW PREP SHEET — ${fmtDate(report.created_at)}`,
      report.source_filename,
      '─'.repeat(52),
      `${report.vocal_summary.avg_wpm} avg wpm  ·  ${report.speaker_summary.you_pct.toFixed(0)}% you spoke  ·  ${report.qa_pairs.length} Q&A pairs`,
      '',
    ];
    for (const qa of report.qa_pairs) {
      const grade = qaGrade(qa);
      const flags: string[] = [`${qa.wpm} wpm (${fmtWpm(qa.wpm)})`, `${fmtDuration(qa.duration_secs)}`];
      if (qa.missed_followup) flags.push('no follow-up');
      if (qa.missed_metric) flags.push('no metric');
      lines.push(`[${grade}]  ${qa.question}`);
      lines.push(`     ${flags.join('  ·  ')}`);
      lines.push('');
      lines.push('  What you said:');
      lines.push(`  ${qa.answer_text.replace(/\n/g, '\n  ')}`);
      lines.push('');
      lines.push('  Coaching:');
      lines.push(`  ${qa.coaching.replace(/\n/g, '\n  ')}`);
      lines.push('');
      lines.push('─'.repeat(52));
      lines.push('');
    }
    return lines.join('\n');
  }

  function buildPrepMarkdown(): string {
    const lines: string[] = [
      `# Interview Prep Sheet`,
      `**${report.source_filename}** · ${fmtDate(report.created_at)}`,
      '',
      `> ${report.vocal_summary.avg_wpm} avg wpm · ${report.speaker_summary.you_pct.toFixed(0)}% you spoke · ${report.qa_pairs.length} Q&A pairs`,
      '',
    ];
    for (const qa of report.qa_pairs) {
      const grade = qaGrade(qa);
      const flags: string[] = [`${qa.wpm} wpm`];
      if (qa.missed_followup) flags.push('⚠ no follow-up');
      if (qa.missed_metric) flags.push('⚠ no metric');
      lines.push(`## [${grade}] ${qa.question}`);
      lines.push(`*${fmtDuration(qa.duration_secs)} · ${flags.join(' · ')}*`);
      lines.push('');
      lines.push('**What you said:**');
      lines.push(`> ${qa.answer_text}`);
      lines.push('');
      lines.push('**Coaching:**');
      lines.push(qa.coaching);
      lines.push('');
      lines.push('---');
      lines.push('');
    }
    return lines.join('\n');
  }

  function downloadPrepSheet() {
    const text = buildPrepText();
    const blob = new Blob([text], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `prep-sheet-${new Date(report.created_at).toISOString().slice(0,10)}.txt`;
    a.click();
    URL.revokeObjectURL(url);
  }

  async function copyMarkdown() {
    const text = buildPrepMarkdown();
    await navigator.clipboard.writeText(text);
    copyMdState = 'copied';
    setTimeout(() => { copyMdState = 'idle'; }, 2000);
  }

  function sendEmail() {
    if (!emailTo.trim()) return;
    localStorage.setItem('review-email', emailTo.trim());
    const prep = buildPrepText();
    const MAX_BODY = 1600;
    const body = prep.length > MAX_BODY
      ? prep.slice(0, MAX_BODY) + '\n\n[Truncated — download full prep sheet from the app]'
      : prep;
    const sub = encodeURIComponent(`Interview Prep Sheet — ${new Date(report.created_at).toLocaleDateString()}`);
    const mailto = `mailto:${encodeURIComponent(emailTo.trim())}?subject=${sub}&body=${encodeURIComponent(body)}`;
    window.open(mailto.slice(0, 2000), '_self');
    emailSent = true;
    setTimeout(() => { emailSent = false; }, 3000);
  }

  async function doDelete() {
    const resp = await fetch(`/api/review/${report.id}`, { method: 'DELETE' });
    if (resp.ok) onDelete?.(report.id);
    onClose();
  }
</script>

<div class="modal-backdrop" onclick={onClose} role="none">
  <div class="modal" onclick={(e) => e.stopPropagation()} role="dialog">

    <!-- Header -->
    <div class="modal-header">
      <div class="header-left">
        <h2>{report.source_filename || 'Interview Review'}</h2>
        <span class="meta">
          {fmtDate(report.created_at)} · {fmtDuration(report.duration_secs)}
          {#if report.source_type === 'live'}
            <span class="badge live">Live</span>
          {:else}
            <span class="badge upload">Upload</span>
          {/if}
        </span>
      </div>
      <button class="close-btn" onclick={onClose}>✕</button>
    </div>

    <div class="modal-body">

      <!-- Summary row -->
      <div class="summary-row">
        <div class="stat-card">
          <span class="stat-val">{report.speaker_summary.you_pct.toFixed(0)}%</span>
          <span class="stat-label">You spoke</span>
        </div>
        <div class="stat-card">
          <span class="stat-val">{report.vocal_summary.avg_wpm}</span>
          <span class="stat-label">avg WPM</span>
        </div>
        <div class="stat-card">
          <span class="stat-val">{report.qa_pairs.length}</span>
          <span class="stat-label">Q&amp;A pairs</span>
        </div>
        <div class="stat-card">
          <span class="stat-val">{report.keywords_mentioned.length}</span>
          <span class="stat-label">keywords used</span>
        </div>
      </div>

      {#if report.keywords_mentioned.length > 0}
        <div class="keyword-row">
          {#each report.keywords_mentioned as kw}
            <span class="kw-chip">{kw}</span>
          {/each}
        </div>
      {/if}

      <!-- Replay -->
      <div class="replay-section">
        <div class="section-label">REPLAY</div>
        {#if isVideo}
          <!-- svelte-ignore a11y_media_has_caption -->
          <video bind:this={mediaEl} src={sourceUrl} controls class="media-el" ontimeupdate={onTimeUpdate}></video>
        {:else}
          <audio bind:this={mediaEl} src={sourceUrl} controls class="audio-el" ontimeupdate={onTimeUpdate}></audio>
        {/if}

        {#if (report.sentiment_events?.length ?? 0) > 0}
          {@const sent = currentSentiment()}
          {#if sent}
            {@const color = EMOTION_COLOR[sent.emotion] ?? '#94a3b8'}
            <div class="sentiment-strip">
              <span class="sent-emotion" style="color: {color}">{sent.emotion}</span>
              {#if sent.reason}<span class="sent-reason">{sent.reason}</span>{/if}
              {#if sent.coaching}
                <button class="sent-coaching-btn" onclick={() => { coachingExpanded = !coachingExpanded; }}>
                  {coachingExpanded ? '▾' : '▸'} tip
                </button>
                {#if coachingExpanded}
                  <span class="sent-coaching">{sent.coaching}</span>
                {/if}
              {/if}
            </div>
          {:else if currentMs > 0}
            <div class="sentiment-strip sentiment-waiting">
              <span class="sent-waiting">Interviewer sentiment will appear at speaker turns…</span>
            </div>
          {/if}
        {/if}

        <div class="transcript-scroll">
          {#each report.transcript as seg, i}
            <div
              class="seg"
              class:active={activeSegIdx() === i}
              class:you={seg.speaker === 'You'}
              role="button"
              tabindex="0"
              onclick={() => seekTo(seg.start_ms)}
              onkeydown={(e) => { if (e.key === 'Enter') seekTo(seg.start_ms); }}
            >
              <span class="seg-speaker">{seg.speaker}</span>
              <span class="seg-time">{fmtDuration(seg.start_ms / 1000)}</span>
              <span class="seg-text">{seg.text}</span>
            </div>
          {/each}
        </div>
      </div>

      <!-- Q&A two-column cards -->
      {#if report.qa_pairs.length > 0}
        <div class="section-label">Q&amp;A REVIEW</div>
        {#if currentMs === 0}
          <div class="qa-replay-hint">▶ Play recording to reveal cards in real time, or review all below</div>
        {/if}
        <div class="qa-list" bind:this={qaListEl}>
          {#each visibleQa as qa}
            {@const grade = qaGrade(qa)}
            {@const gcolor = gradeColor(grade)}
            <div class="qa-card">
              <div class="qa-card-header">
                <span class="qa-grade" style="color:{gcolor}; border-color:{gcolor}40">{grade}</span>
                <span class="qa-question">{qa.question}</span>
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <span class="seek-btn" role="button" tabindex="0"
                  title="Jump to this question"
                  onclick={() => seekTo(qa.start_ms)}
                  onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') seekTo(qa.start_ms); }}
                >⏩ {fmtDuration(qa.start_ms / 1000)}</span>
              </div>
              <div class="qa-card-body">
                <div class="qa-col-left">
                  <div class="qa-stats">
                    <span style="color:{wpmColor(qa.wpm)}">{qa.wpm} wpm</span>
                    <span class="qa-stat-sep">·</span>
                    <span>{fmtDuration(qa.duration_secs)}</span>
                    {#if qa.missed_followup}
                      <span class="qa-stat-sep">·</span>
                      <span class="qa-flag">no follow-up</span>
                    {/if}
                    {#if qa.missed_metric}
                      <span class="qa-stat-sep">·</span>
                      <span class="qa-flag">no metric</span>
                    {/if}
                  </div>
                  <div class="qa-col-label">What you said</div>
                  <p class="qa-answer">{qa.answer_text}</p>
                </div>
                <div class="qa-col-right">
                  <div class="qa-col-label">Coaching</div>
                  <p class="qa-coaching">{qa.coaching}</p>
                  {#if onPractice}
                    <button class="practice-btn" onclick={() => { onPractice?.(qa.question); onClose(); }}>
                      Practice this →
                    </button>
                  {/if}
                </div>
              </div>
            </div>
          {/each}
        </div>
      {/if}

    </div>

    <!-- Export footer -->
    <div class="share-footer">
      <div class="share-group">
        <button class="share-btn" onclick={downloadPrepSheet}>↓ Prep Sheet</button>
        <button class="share-btn" class:copied={copyMdState === 'copied'} onclick={copyMarkdown}>
          {copyMdState === 'copied' ? '✓ Copied!' : 'Copy Markdown'}
        </button>
        <a class="share-btn" href={downloadUrl} download title="Full server-generated report">↓ Full Report</a>
      </div>
      <div class="email-row">
        <input
          class="email-input"
          type="email"
          placeholder="Email myself…"
          bind:value={emailTo}
          onkeydown={(e) => { if (e.key === 'Enter') sendEmail(); }}
        />
        <button class="send-btn" class:sent={emailSent} onclick={sendEmail} disabled={!emailTo.trim()}>
          {emailSent ? '✓' : '✉'}
        </button>
      </div>
      {#if onDelete}
        <div class="delete-group">
          {#if confirmDelete}
            <button class="del-confirm-btn" onclick={doDelete}>Confirm delete</button>
            <button class="del-cancel-btn" onclick={() => { confirmDelete = false; }}>Cancel</button>
          {:else}
            <button class="del-btn" onclick={() => { confirmDelete = true; }}>Delete</button>
          {/if}
        </div>
      {/if}
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
    width: min(820px, 97vw); max-height: 90vh;
    display: flex; flex-direction: column; overflow: hidden;
  }
  .modal-header {
    display: flex; align-items: flex-start; justify-content: space-between;
    padding: 1rem 1.5rem; border-bottom: 1px solid #1e293b; flex-shrink: 0; gap: 1rem;
  }
  .header-left { display: flex; flex-direction: column; gap: 0.25rem; min-width: 0; }
  h2 { font-size: 1rem; font-weight: 700; color: #f1f5f9; margin: 0; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .meta { font-size: 0.78rem; color: #64748b; display: flex; align-items: center; gap: 0.4rem; flex-wrap: wrap; }
  .close-btn { background: none; border: none; color: #64748b; font-size: 1rem; cursor: pointer; flex-shrink: 0; padding: 0.2rem 0.4rem; }
  .close-btn:hover { color: #e2e8f0; }

  .modal-body { overflow-y: auto; padding: 1.25rem 1.5rem; display: flex; flex-direction: column; gap: 1.25rem; flex: 1; }

  /* Summary */
  .summary-row { display: grid; grid-template-columns: repeat(4, 1fr); gap: 0.75rem; }
  .stat-card { background: #0a1628; border: 1px solid #1e293b; border-radius: 0.5rem; padding: 0.75rem; display: flex; flex-direction: column; align-items: center; gap: 0.2rem; }
  .stat-val { font-size: 1.4rem; font-weight: 700; color: #60a5fa; }
  .stat-label { font-size: 0.72rem; color: #64748b; text-align: center; }

  .keyword-row { display: flex; flex-wrap: wrap; gap: 0.35rem; }
  .kw-chip { padding: 0.15rem 0.6rem; border-radius: 1rem; background: rgba(59,130,246,0.12); border: 1px solid rgba(59,130,246,0.3); color: #93c5fd; font-size: 0.75rem; }

  .badge { font-size: 0.68rem; padding: 0.1rem 0.45rem; border-radius: 0.25rem; font-weight: 600; letter-spacing: 0.03em; }
  .badge.live { background: rgba(167,139,250,0.15); color: #a78bfa; }
  .badge.upload { background: rgba(59,130,246,0.12); color: #60a5fa; }

  /* Replay */
  .section-label { font-size: 0.68rem; font-weight: 700; letter-spacing: 0.1em; color: #475569; text-transform: uppercase; margin-bottom: -0.5rem; }
  .replay-section { display: flex; flex-direction: column; gap: 0.75rem; }
  .media-el { width: 100%; border-radius: 0.5rem; background: #000; max-height: 240px; object-fit: contain; }
  .audio-el { width: 100%; }

  .transcript-scroll { max-height: 180px; overflow-y: auto; border: 1px solid #1e293b; border-radius: 0.5rem; display: flex; flex-direction: column; }
  .seg { display: flex; align-items: baseline; gap: 0.5rem; padding: 0.35rem 0.75rem; cursor: pointer; transition: background 0.1s; font-size: 0.8rem; }
  .seg:hover { background: #0f1f35; }
  .seg.active { background: rgba(59,130,246,0.12); }
  .seg-speaker { color: #64748b; min-width: 5rem; font-size: 0.72rem; text-transform: uppercase; flex-shrink: 0; }
  .seg.you .seg-speaker { color: #60a5fa; }
  .seg-time { color: #334155; min-width: 2.5rem; font-size: 0.72rem; flex-shrink: 0; }
  .seg-text { color: #94a3b8; line-height: 1.4; flex: 1; }

  /* Sentiment strip */
  .sentiment-strip { display: flex; align-items: center; flex-wrap: wrap; gap: 0.4rem; padding: 0.4rem 0.75rem; background: #060e1a; border: 1px solid #1e293b; border-radius: 0.4rem; font-size: 0.78rem; }
  .sent-emotion { font-weight: 700; text-transform: capitalize; font-size: 0.82rem; flex-shrink: 0; }
  .sent-reason { color: #64748b; flex-shrink: 0; }
  .sent-coaching-btn { background: none; border: none; color: #475569; font-size: 0.75rem; cursor: pointer; padding: 0; flex-shrink: 0; }
  .sent-coaching-btn:hover { color: #94a3b8; }
  .sent-coaching { width: 100%; color: #7dd3fc; font-size: 0.78rem; line-height: 1.4; padding-top: 0.15rem; }
  .sentiment-waiting { color: #334155; font-style: italic; font-size: 0.75rem; }
  .sent-waiting { color: #334155; }

  /* Q&A two-column cards */
  .qa-replay-hint { font-size: 0.78rem; color: #475569; text-align: center; padding: 0.6rem; background: #060e1a; border: 1px dashed #1e293b; border-radius: 0.5rem; }
  .qa-list { display: flex; flex-direction: column; gap: 0.6rem; }

  .qa-card { border: 1px solid #1e293b; border-radius: 0.5rem; overflow: hidden; background: #060e1a; }
  .qa-card-header {
    display: flex; align-items: baseline; gap: 0.6rem;
    padding: 0.55rem 0.75rem; background: #0a1628;
    border-bottom: 1px solid #1a2d4a;
  }
  .qa-grade {
    font-size: 0.72rem; font-weight: 800; letter-spacing: 0.05em;
    border: 1px solid; border-radius: 0.25rem;
    padding: 0.05rem 0.35rem; flex-shrink: 0;
  }
  .qa-question { flex: 1; color: #cbd5e1; font-size: 0.85rem; line-height: 1.4; }
  .seek-btn {
    background: transparent; border: 1px solid #1e3a5f; border-radius: 0.25rem;
    color: #60a5fa; font-size: 0.68rem; padding: 0.1rem 0.4rem;
    cursor: pointer; flex-shrink: 0; white-space: nowrap; transition: background 0.15s;
  }
  .seek-btn:hover { background: rgba(59,130,246,0.12); }

  .qa-card-body {
    display: grid; grid-template-columns: 1fr 1fr; gap: 0;
  }
  .qa-col-left {
    padding: 0.65rem 0.75rem; border-right: 1px solid #1a2d4a;
    display: flex; flex-direction: column; gap: 0.35rem;
  }
  .qa-col-right {
    padding: 0.65rem 0.75rem;
    display: flex; flex-direction: column; gap: 0.35rem;
  }
  .qa-col-label { font-size: 0.65rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.08em; color: #334155; }
  .qa-stats { display: flex; align-items: center; flex-wrap: wrap; gap: 0.25rem; font-size: 0.75rem; }
  .qa-stat-sep { color: #1e293b; }
  .qa-flag { color: #f87171; font-size: 0.72rem; }
  .qa-answer { color: #94a3b8; font-size: 0.8rem; line-height: 1.5; margin: 0; }
  .qa-coaching { color: #7dd3fc; font-size: 0.8rem; line-height: 1.5; margin: 0; flex: 1; }
  .practice-btn {
    margin-top: auto; align-self: flex-start;
    padding: 0.2rem 0.6rem; background: transparent;
    border: 1px solid #1e3a5f; border-radius: 0.25rem;
    color: #60a5fa; font-size: 0.72rem; cursor: pointer;
    transition: all 0.12s; white-space: nowrap;
  }
  .practice-btn:hover { border-color: #3b82f6; color: #93c5fd; background: rgba(59,130,246,0.08); }

  /* Export footer */
  .share-footer {
    display: flex; align-items: center; gap: 0.75rem; flex-wrap: wrap;
    padding: 0.65rem 1.25rem; border-top: 1px solid #1e293b;
    background: #080f1c; flex-shrink: 0; border-radius: 0 0 0.75rem 0.75rem;
  }
  .share-group { display: flex; gap: 0.4rem; align-items: center; }
  .share-btn {
    padding: 0.28rem 0.65rem; background: transparent;
    border: 1px solid #334155; border-radius: 0.3rem;
    color: #64748b; font-size: 0.78rem; cursor: pointer; text-decoration: none;
    transition: all 0.15s; display: inline-flex; align-items: center; white-space: nowrap;
  }
  .share-btn:hover { border-color: #60a5fa; color: #60a5fa; }
  .share-btn.copied { border-color: #4ade80; color: #4ade80; }
  .email-row { display: flex; align-items: center; gap: 0.35rem; flex: 1; min-width: 160px; }
  .email-input { flex: 1; padding: 0.28rem 0.6rem; background: #0f172a; border: 1px solid #1e293b; border-radius: 0.3rem; color: #e2e8f0; font-size: 0.78rem; outline: none; transition: border-color 0.15s; }
  .email-input:focus { border-color: #3b82f6; }
  .send-btn { padding: 0.28rem 0.6rem; background: #1d4ed8; border: none; border-radius: 0.3rem; color: white; font-size: 0.85rem; cursor: pointer; transition: background 0.15s; }
  .send-btn:hover:not(:disabled) { background: #2563eb; }
  .send-btn:disabled { opacity: 0.4; cursor: default; }
  .send-btn.sent { background: #166534; }
  .delete-group { display: flex; gap: 0.35rem; margin-left: auto; }
  .del-btn { padding: 0.28rem 0.6rem; background: transparent; border: 1px solid #450a0a; border-radius: 0.3rem; color: #7f1d1d; font-size: 0.78rem; cursor: pointer; transition: all 0.15s; }
  .del-btn:hover { border-color: #f87171; color: #f87171; }
  .del-confirm-btn { padding: 0.28rem 0.6rem; background: #7f1d1d; border: none; border-radius: 0.3rem; color: #fca5a5; font-size: 0.78rem; cursor: pointer; }
  .del-cancel-btn { padding: 0.28rem 0.6rem; background: none; border: 1px solid #334155; border-radius: 0.3rem; color: #64748b; font-size: 0.78rem; cursor: pointer; }
</style>
