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

  import type { TranscriptEntry, SuggestionEntry, PracticeAnswer, DebriefResult } from '../lib/types';
  import { TAG_CONFIG } from '../lib/questionTagger';

  const { report, onClose, onDelete, onPractice, liveSuggestions, liveTranscript, practiceAnswers, fillerCounts, hedgeCounts, recordingUrl, onSave, initialTab } = $props<{
    report?: ReviewReport;
    onClose: () => void;
    onDelete?: (id: string) => void;
    onPractice?: (question: string) => void;
    liveSuggestions?: SuggestionEntry[];
    liveTranscript?: TranscriptEntry[];
    practiceAnswers?: PracticeAnswer[];
    fillerCounts?: { word: string; count: number }[];
    hedgeCounts?: { word: string; count: number }[];
    recordingUrl?: string;
    onSave?: (result: DebriefResult) => void;
    initialTab?: 'qa' | 'transcript' | 'review' | 'recording';
  }>();

  type Tab = 'qa' | 'transcript' | 'review' | 'recording';
  let activeTab = $state<Tab>(initialTab ?? 'qa');
  let mediaEl = $state<HTMLVideoElement | HTMLAudioElement | null>(null);
  let currentMs = $state(0);
  let qaListEl = $state<HTMLDivElement | null>(null);
  let prevVisibleCount = $state(0);
  let copyMdState = $state<'idle' | 'copied'>('idle');
  let emailTo = $state(localStorage.getItem('review-email') ?? '');
  let emailSent = $state(false);
  let confirmDelete = $state(false);
  let coachingExpanded = $state(false);

  // ── Live debrief state ─────────────────────────────────────────────────────
  interface DebriefQaEntry {
    question: string; tag?: string; suggestion: string; answerText: string;
    coaching: string; missedFollowup: boolean; missedMetric: boolean;
    confidenceScore?: number; loadingCoaching: boolean; suggestionOpen: boolean;
  }
  // ── Hedge alternatives ────────────────────────────────────────────────────
  const HEDGE_ALTS: Record<string, string[]> = {
    'i think':       ['State it directly', 'My view is…', 'From my experience…'],
    'i believe':     ['State it directly', 'From my experience…', 'The data shows…'],
    'i guess':       ['Commit to the answer', 'My best estimate is…', 'Typically…'],
    'i feel like':   ['My take is…', 'In my experience…', 'State it directly'],
    'kind of':       ['Be specific about the nuance', 'Somewhat', 'Approximately'],
    'sort of':       ['Be specific about the nuance', 'Somewhat', 'Roughly'],
    'basically':     ['Drop it — just say the thing', 'In short…', 'The key point is…'],
    'probably':      ['Commit: "it is"', 'Typically…', 'In most cases…'],
    'maybe':         ['Commit to the answer', 'One option is…', 'It depends on X'],
    'perhaps':       ['Commit to the answer', 'One option is…', 'It depends on X'],
    'hopefully':     ['The plan is…', 'The goal is…', 'We expect…'],
    'i just':        ['Drop "just" — "I wanted to…"', 'State directly'],
    'just':          ['Drop it entirely', 'State directly'],
    'a little bit':  ['Be specific: how much?', 'Somewhat', 'Drop it'],
    'you know':      ['Pause instead', 'Drop it'],
    'like':          ['Pause instead', 'Drop it'],
    'um':            ['Pause silently', 'Breathe, then continue'],
    'uh':            ['Pause silently', 'Breathe, then continue'],
    'actually':      ['Drop it', 'State directly'],
    'honestly':      ['Drop it — everything you say should be honest', 'State directly'],
    'to be honest':  ['Drop it', 'State directly'],
    "i'm not sure but": ["I don't know the exact figure, but typically…", 'Let me think… [pause]', "I'd estimate…"],
    'i could be wrong': ['Own your answer', 'My best read is…', "I'd verify, but…"],
  };
  let hedgePopover = $state<{ word: string; top: number; left: number } | null>(null);
  $effect(() => {
    if (!hedgePopover) return;
    const close = () => { hedgePopover = null; };
    window.addEventListener('click', close);
    return () => window.removeEventListener('click', close);
  });

  let debriefLoading = $state(false);
  let debriefResult = $state<DebriefResult | null>(null);
  let debriefError = $state('');
  let nextSteps = $state<string[]>([]);
  let loadingNextSteps = $state(false);
  let debriefQa = $state<DebriefQaEntry[]>([]);
  let debriefFetched = $state(false);
  let debriefEmailCopied = $state(false);
  let emailDraft = $state('');
  let emailLoading = $state(false);
  let emailGenerated = $state(false);

  const simulatedSuggestions = $derived(
    (liveSuggestions ?? [])
      .filter(s => s.question && s.suggestion && s.source === 'simulated')
      .sort((a, b) => (a.detectedAt ?? 0) - (b.detectedAt ?? 0))
  );
  const totalPracticeCount = $derived(simulatedSuggestions.length + (practiceAnswers?.length ?? 0));

  function extractAnswer(detectedAt: number, nextDetectedAt: number): string {
    return (liveTranscript ?? [])
      .filter(e => e.speaker === 'You' && e.timestamp_ms > detectedAt && (nextDetectedAt === 0 || e.timestamp_ms < nextDetectedAt))
      .map(e => e.text).join(' ').trim();
  }

  function initDebriefQa() {
    if (!liveSuggestions) return;
    const sorted = [...liveSuggestions]
      .filter(s => s.question && s.suggestion && s.source !== 'simulated')
      .sort((a, b) => (a.detectedAt ?? 0) - (b.detectedAt ?? 0));
    debriefQa = sorted.map((s, i) => {
      const nextAt = sorted[i + 1]?.detectedAt ?? 0;
      const answerText = extractAnswer(s.detectedAt ?? 0, nextAt)
        || (practiceAnswers ?? []).find(pa => pa.question === s.question)?.answerText
        || '';
      return {
        question: s.question, tag: s.tag, suggestion: s.suggestion, answerText,
        coaching: s.answerFeedback?.coaching ?? '',
        missedFollowup: s.answerFeedback?.missed_followup ?? false,
        missedMetric: s.answerFeedback?.missed_metric ?? false,
        confidenceScore: s.vocalFeedback?.confidence_score,
        loadingCoaching: !s.answerFeedback && !!answerText,
        suggestionOpen: false,
      };
    });
  }

  async function fetchMissingCoaching() {
    for (let i = 0; i < debriefQa.length; i++) {
      if (!debriefQa[i].loadingCoaching) continue;
      try {
        const resp = await fetch('/api/answer-feedback', {
          method: 'POST', headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ question: debriefQa[i].question, answer: debriefQa[i].answerText, suggestion: debriefQa[i].suggestion }),
        });
        const data = resp.ok ? await resp.json() : null;
        debriefQa[i] = { ...debriefQa[i], ...(data ? { coaching: data.coaching, missedFollowup: data.missed_followup, missedMetric: data.missed_metric } : {}), loadingCoaching: false };
      } catch { debriefQa[i] = { ...debriefQa[i], loadingCoaching: false }; }
      debriefQa = [...debriefQa];
    }
  }

  async function fetchNextSteps() {
    if (!liveTranscript?.length) return;
    loadingNextSteps = true;
    try {
      const resp = await fetch('/api/next-steps', { method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ transcript: liveTranscript.map(e => ({ speaker: e.speaker, text: e.text })) }) });
      if (resp.ok) { const d = await resp.json(); nextSteps = d.steps ?? []; }
    } catch { /**/ }
    loadingNextSteps = false;
  }

  async function fetchDebrief() {
    if (debriefFetched || !liveSuggestions) return;
    debriefFetched = true;

    // Skip AI call if there's no real interview content to analyze
    const hasRealContent =
      (liveTranscript ?? []).some(e => e.speaker === 'You' && e.text.trim().length > 5) ||
      liveSuggestions.some(s => s.suggestion && s.source !== 'simulated');
    if (!hasRealContent) {
      debriefResult = { summary: 'No interview activity to analyze — complete a live interview or practice answering questions to get feedback.', strong_points: [], improvement_areas: [], followup_email: [], followup_email_draft: '' };
      onSave?.(debriefResult);
      initDebriefQa();
      return;
    }

    debriefLoading = true;
    fetchNextSteps();
    initDebriefQa();
    try {
      const allSuggestions = liveSuggestions.filter(s => s.suggestion);
      const answeredCount = allSuggestions.filter(s =>
        s.source !== 'simulated' && (liveTranscript ?? []).some(t =>
          t.speaker === 'You' && s.detectedAt != null && t.timestamp_ms > s.detectedAt
        )
      ).length;
      const resp = await fetch('/api/debrief', { method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          transcript: (liveTranscript ?? []).map(e => ({ speaker: e.speaker, text: e.text })),
          suggestions: allSuggestions.map(s => ({ question: s.question, suggestion: s.suggestion })),
          filler_counts: (fillerCounts ?? []).map(f => ({ word: f.word, count: f.count })),
          hedge_counts: (hedgeCounts ?? []).map(h => ({ word: h.word, count: h.count })),
          practice_answers: (practiceAnswers ?? []).map(p => ({ question: p.question, answer: p.answerText ?? '' })),
          answered_count: answeredCount,
          viewed_count: allSuggestions.filter(s => s.source !== 'simulated').length,
        }) });
      if (!resp.ok) throw new Error(`Debrief failed: ${resp.status}`);
      debriefResult = await resp.json();
      onSave?.(debriefResult);
    } catch (e) { debriefError = String(e); }
    finally { debriefLoading = false; }
    fetchMissingCoaching();
  }

  async function generateEmail() {
    if (emailLoading || emailGenerated || !debriefResult) return;
    emailLoading = true;
    try {
      const resp = await fetch('/api/followup-email', { method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          transcript: (liveTranscript ?? []).map(e => ({ speaker: e.speaker, text: e.text })),
          followup_bullets: debriefResult.followup_email ?? [],
        }) });
      if (resp.ok) { const d = await resp.json(); emailDraft = d.email ?? ''; emailGenerated = true; }
    } catch { /**/ }
    emailLoading = false;
  }

  // Auto-run debrief on mount when live — ensures saveInterview fires regardless of which tab user visits
  if (liveSuggestions) fetchDebrief();

  $effect(() => {
    // If user navigates to review before debrief finishes, init Q&A entries immediately
    if (activeTab === 'review' && !debriefFetched) { debriefFetched = true; initDebriefQa(); fetchMissingCoaching(); }
  });

  async function copyDebriefEmail() {
    const text = emailDraft || (debriefResult?.followup_email ?? []).map(p => `• ${p}`).join('\n');
    await navigator.clipboard.writeText(text);
    debriefEmailCopied = true;
    setTimeout(() => { debriefEmailCopied = false; }, 2000);
  }

  const isVideo = $derived(report ? ['mp4','webm','mov','mkv','avi'].some(
    ext => report.source_filename.toLowerCase().endsWith(`.${ext}`)
  ) : false);

  const sourceUrl = $derived(report ? `/api/review/${report.id}/source` : '');
  const downloadUrl = $derived(report ? `/api/review/${report.id}/download` : '');

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
    report ? (currentMs > 0
      ? report.qa_pairs.filter(qa => currentMs >= qa.start_ms)
      : report.qa_pairs) : []
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
    if (!report) return -1;
    for (let i = report.transcript.length - 1; i >= 0; i--) {
      if (report.transcript[i].start_ms <= ms) return i;
    }
    return -1;
  });

  const currentSentiment = $derived(() => {
    const events = report?.sentiment_events ?? [];
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
        <h2>{report?.source_filename || 'Interview Review'}</h2>
        {#if report}
          <span class="meta">
            {fmtDate(report.created_at)} · {fmtDuration(report.duration_secs)}
            {#if report.source_type === 'live'}
              <span class="badge live">Live</span>
            {:else}
              <span class="badge upload">Upload</span>
            {/if}
          </span>
        {/if}
      </div>
      <button class="close-btn" onclick={onClose}>✕</button>
    </div>

    <!-- Summary row — only when report is loaded -->
    {#if report}
      <div class="summary-bar">
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
          <span class="stat-label">keywords</span>
        </div>
      </div>
    {/if}

    <!-- Tab bar -->
    <div class="tab-bar">
      {#if report}
        <button class="tab" class:active={activeTab === 'qa'} onclick={() => activeTab = 'qa'}>
          Q&amp;A Review {#if report.qa_pairs.length > 0}<span class="tab-count">{report.qa_pairs.length}</span>{/if}
        </button>
        <button class="tab" class:active={activeTab === 'transcript'} onclick={() => activeTab = 'transcript'}>Transcript</button>
      {/if}
      {#if liveSuggestions}
        <button class="tab" class:active={activeTab === 'review'} onclick={() => activeTab = 'review'}>
          Review {debriefQa.length > 0 ? `(${debriefQa.length})` : ''}
        </button>
        {#if recordingUrl}
          <button class="tab" class:active={activeTab === 'recording'} onclick={() => activeTab = 'recording'}>Recording</button>
        {/if}
      {/if}
    </div>

    <div class="modal-body">

      {#if activeTab === 'qa' && !report}
        <div class="deb-loading">Saving session report…</div>
      {:else if activeTab === 'qa' && report}
        <!-- Media player compact bar -->
        <div class="replay-bar">
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
                  {#if coachingExpanded}<span class="sent-coaching">{sent.coaching}</span>{/if}
                {/if}
              </div>
            {/if}
          {/if}
        </div>

        {#if report.keywords_mentioned.length > 0}
          <div class="keyword-row">
            {#each report.keywords_mentioned as kw}
              <span class="kw-chip">{kw}</span>
            {/each}
          </div>
        {/if}

        {#if report.qa_pairs.length === 0}
          <div class="qa-empty">No Q&amp;A pairs were detected in this recording.</div>
        {:else}
          {#if currentMs === 0}
            <div class="qa-replay-hint">▶ Play to reveal cards in real time, or all shown below</div>
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
                      {#if qa.missed_followup}<span class="qa-stat-sep">·</span><span class="qa-flag">no follow-up</span>{/if}
                      {#if qa.missed_metric}<span class="qa-stat-sep">·</span><span class="qa-flag">no metric</span>{/if}
                    </div>
                    <div class="qa-col-label">What you said</div>
                    <p class="qa-answer">{qa.answer_text}</p>
                  </div>
                  <div class="qa-col-right">
                    <div class="qa-col-label">Coaching</div>
                    <p class="qa-coaching">{qa.coaching}</p>
                    {#if onPractice}
                      <button class="practice-btn" onclick={() => { onPractice?.(qa.question); onClose(); }}>Practice this →</button>
                    {/if}
                  </div>
                </div>
              </div>
            {/each}
          </div>
        {/if}

      {:else if activeTab === 'transcript' && report}
        <!-- Transcript tab -->
        <div class="replay-bar">
          {#if isVideo}
            <!-- svelte-ignore a11y_media_has_caption -->
            <video bind:this={mediaEl} src={sourceUrl} controls class="media-el" ontimeupdate={onTimeUpdate}></video>
          {:else}
            <audio bind:this={mediaEl} src={sourceUrl} controls class="audio-el" ontimeupdate={onTimeUpdate}></audio>
          {/if}
        </div>
        <div class="transcript-full">
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

      {:else if activeTab === 'review'}
        <!-- Combined Review tab: AI summary + questions + practice + follow-up -->

        <!-- AI Summary -->
        {#if debriefLoading}
          <div class="deb-loading">Analyzing your interview…</div>
        {:else if debriefError}
          <div class="deb-error">{debriefError}</div>
        {:else if debriefResult}
          <p class="deb-summary">{debriefResult.summary}</p>
          {#if debriefResult.strong_points.length > 0 || debriefResult.improvement_areas.length > 0}
            <div class="deb-two-col">
              <section class="deb-section">
                <h3 class="deb-h3 deb-green">Strong Moments</h3>
                <ul class="deb-list">{#each debriefResult.strong_points as p}<li>{p}</li>{/each}</ul>
              </section>
              <section class="deb-section">
                <h3 class="deb-h3 deb-yellow">Areas to Improve</h3>
                <ul class="deb-list">{#each debriefResult.improvement_areas as p}<li>{p}</li>{/each}</ul>
              </section>
            </div>
          {/if}
        {/if}

        <!-- Performance: filler + hedge chips -->
        {#if (fillerCounts ?? []).length > 0 || (hedgeCounts ?? []).length > 0}
          <div class="review-section-divider">Word Choice</div>
          <div class="perf-chips-wrap">
            {#if (fillerCounts ?? []).length > 0}
              <div class="perf-group">
                <span class="perf-group-label">Filler</span>
                {#each (fillerCounts ?? []) as f}
                  <span class="perf-chip perf-chip-filler">{f.word} ×{f.count}</span>
                {/each}
              </div>
            {/if}
            {#if (hedgeCounts ?? []).length > 0}
              <div class="perf-group">
                <span class="perf-group-label">Hedge</span>
                {#each (hedgeCounts ?? []) as h}
                  {@const alts = HEDGE_ALTS[h.word.toLowerCase()] ?? []}
                  <button class="perf-chip perf-chip-hedge" class:active={hedgePopover?.word === h.word}
                    onclick={(e) => { e.stopPropagation(); if (hedgePopover?.word === h.word) { hedgePopover = null; return; } const r = (e.currentTarget as HTMLElement).getBoundingClientRect(); const popW = 240; const left = r.right + popW > window.innerWidth ? r.right - popW : r.left; hedgePopover = alts.length ? { word: h.word, top: r.bottom + 5, left } : null; }}>
                    {h.word} ×{h.count}{alts.length ? ' ▾' : ''}
                  </button>
                {/each}
              </div>
            {/if}
          </div>
        {/if}

        <!-- Live questions -->
        {#if debriefQa.length > 0}
          <div class="review-section-divider">Interview Questions ({debriefQa.length})</div>
          <div class="deb-timeline">
            {#each debriefQa as qa, i}
              <div class="tl-item">
                <div class="tl-q-row">
                  {#if qa.tag}
                    {@const tc = TAG_CONFIG[qa.tag]}
                    <span class="tl-tag" style="color: {tc.color}; background: {tc.bg}">{tc.label}</span>
                  {/if}
                  <span class="tl-num">{i + 1}</span>
                  <span class="tl-q-text">"{qa.question}"</span>
                </div>
                {#if qa.answerText}
                  <div class="tl-answer">
                    <span class="tl-answer-label">You</span>
                    <span class="tl-answer-text">{qa.answerText}</span>
                  </div>
                {/if}
                <div class="tl-coaching">
                  {#if qa.loadingCoaching}
                    <span class="deb-muted">Analyzing answer…</span>
                  {:else}
                    {#if qa.coaching}<p class="tl-coaching-note">{qa.coaching}</p>{/if}
                    {#if qa.missedFollowup || qa.missedMetric || qa.confidenceScore}
                      <div class="tl-flags">
                        {#if qa.missedMetric}<span class="tl-flag tl-flag-warn">Missing metric</span>{/if}
                        {#if qa.missedFollowup}<span class="tl-flag tl-flag-warn">Missing follow-up</span>{/if}
                        {#if qa.confidenceScore && qa.confidenceScore >= 70}<span class="tl-flag tl-flag-good">Strong answer</span>{/if}
                      </div>
                    {/if}
                  {/if}
                  {#if qa.suggestion}
                    <button class="tl-toggle" onclick={() => { debriefQa[i] = { ...debriefQa[i], suggestionOpen: !debriefQa[i].suggestionOpen }; debriefQa = [...debriefQa]; }}>
                      {debriefQa[i].suggestionOpen ? '▾' : '▸'} AI Suggestion
                    </button>
                    {#if debriefQa[i].suggestionOpen}
                      <div class="tl-suggestion">{qa.suggestion}</div>
                    {/if}
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        {/if}

        <!-- Practice questions -->
        {#if totalPracticeCount > 0}
          <div class="review-section-divider">Practice Questions ({totalPracticeCount})</div>
          <div class="deb-timeline">
            {#each simulatedSuggestions as s, i}
              <div class="tl-item tl-item-practice">
                <div class="tl-q-row">
                  {#if s.tag}
                    {@const tc = TAG_CONFIG[s.tag]}
                    <span class="tl-tag" style="color: {tc.color}; background: {tc.bg}">{tc.label}</span>
                  {/if}
                  <span class="tl-num">{i + 1}</span>
                  <span class="tl-q-text">"{s.question}"</span>
                  <span class="tl-practice-badge">Practice</span>
                </div>
                {#if s.suggestion}
                  <div class="tl-coaching">
                    <p class="tl-suggestion">{s.suggestion}</p>
                  </div>
                {/if}
              </div>
            {/each}
            {#each practiceAnswers ?? [] as pa, i}
              <div class="tl-item tl-item-practice">
                <div class="tl-q-row">
                  <span class="tl-num">{simulatedSuggestions.length + i + 1}</span>
                  <span class="tl-q-text">"{pa.question}"</span>
                  <span class="tl-practice-badge tl-practice-badge-rec">Recorded</span>
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
                      <span class="tl-flag" style="color:{pa.score >= 70 ? '#22c55e' : pa.score >= 50 ? '#f59e0b' : '#ef4444'}; border-color:currentColor">Score: {pa.score}/100</span>
                      {#if pa.vocalTone}<span class="tl-flag" style="color:#94a3b8; border-color:#334155">Tone: {pa.vocalTone}{pa.vocalConfidence !== undefined ? ` · ${pa.vocalConfidence}%` : ''}</span>{/if}
                    </div>
                  {/if}
                  {#if pa.coaching}<p class="tl-coaching-note">{pa.coaching}</p>{/if}
                  {#if pa.strong}<p class="tl-coaching-note" style="color:#4ade80">{pa.strong}</p>{/if}
                </div>
              </div>
            {/each}
          </div>
        {/if}

        <!-- Follow-up section -->
        {#if debriefResult}
          {#if nextSteps.length > 0 || loadingNextSteps}
            <div class="review-section-divider">Next Steps</div>
            {#if loadingNextSteps}
              <p class="deb-muted">Extracting next steps…</p>
            {:else}
              <ul class="deb-list deb-list-pad">{#each nextSteps as s}<li>{s}</li>{/each}</ul>
            {/if}
          {/if}
          <div class="review-section-divider">
            Follow-up Email
            {#if emailGenerated}
              <button class="deb-copy-btn" class:copied={debriefEmailCopied} onclick={copyDebriefEmail}>
                {debriefEmailCopied ? '✓ Copied!' : 'Copy'}
              </button>
            {/if}
          </div>
          {#if emailGenerated && emailDraft}
            <div class="deb-email-draft">
              {#each emailDraft.split('\n') as line}
                {#if line.trim() === ''}<div class="deb-email-blank"></div>
                {:else}<div class="deb-email-line">{line}</div>{/if}
              {/each}
            </div>
          {:else if emailLoading}
            <p class="deb-muted">Drafting email…</p>
          {:else}
            <ul class="deb-list deb-list-pad">{#each debriefResult.followup_email as p}<li>{p}</li>{/each}</ul>
            <button class="deb-gen-email-btn" onclick={generateEmail}>Write full draft ›</button>
          {/if}
        {/if}

      {:else if activeTab === 'recording'}
        <!-- Recording tab -->
        {#if recordingUrl}
          <div class="rec-section">
            <!-- svelte-ignore a11y_media_has_caption -->
            <video class="rec-player" src={recordingUrl} controls></video>
            <a class="share-btn" href={recordingUrl} download="interview-recording.webm">⬇ Download recording</a>
          </div>
        {/if}

      {/if}

    </div>

    <!-- Export footer -->
    <div class="share-footer">
      {#if report}
        <div class="share-group">
          <button class="share-btn" onclick={downloadPrepSheet}>↓ Prep Sheet</button>
          <button class="share-btn" class:copied={copyMdState === 'copied'} onclick={copyMarkdown}>
            {copyMdState === 'copied' ? '✓ Copied!' : 'Copy Markdown'}
          </button>
          <a class="share-btn" href={downloadUrl} download title="Full server-generated report">↓ Full Report</a>
        </div>
      {:else}
        <span class="share-saving">Saving report…</span>
      {/if}
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
      {#if onDelete && report}
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

{#if hedgePopover}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="hedge-popover-fixed" style="top:{hedgePopover.top}px;left:{hedgePopover.left}px" onclick={(e) => e.stopPropagation()}>
    <div class="hedge-popover-title">Instead of "{hedgePopover.word}"</div>
    {#each HEDGE_ALTS[hedgePopover.word.toLowerCase()] ?? [] as alt}
      <div class="hedge-popover-alt">{alt}</div>
    {/each}
  </div>
{/if}

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

  /* Summary bar */
  .summary-bar { display: grid; grid-template-columns: repeat(4, 1fr); gap: 0.5rem; padding: 0.75rem 1.5rem 0; flex-shrink: 0; }
  .stat-card { background: #0a1628; border: 1px solid #1e293b; border-radius: 0.5rem; padding: 0.55rem; display: flex; flex-direction: column; align-items: center; gap: 0.1rem; }
  .stat-val { font-size: 1.2rem; font-weight: 700; color: #60a5fa; }
  .stat-label { font-size: 0.68rem; color: #64748b; text-align: center; }

  /* Tab bar */
  .tab-bar { display: flex; border-bottom: 1px solid #1e293b; padding: 0 1.5rem; flex-shrink: 0; margin-top: 0.5rem; }
  .tab { background: none; border: none; border-bottom: 2px solid transparent; color: #475569; font-size: 0.78rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; padding: 0.4rem 0.75rem; cursor: pointer; transition: color 0.12s, border-color 0.12s; margin-bottom: -1px; display: flex; align-items: center; gap: 0.35rem; }
  .tab:hover { color: #94a3b8; }
  .tab.active { color: #60a5fa; border-bottom-color: #60a5fa; }
  .tab-count { background: #1e3a5f; color: #60a5fa; border-radius: 0.75rem; font-size: 0.65rem; padding: 0.05rem 0.4rem; font-weight: 700; }

  .modal-body { overflow-y: auto; padding: 0.75rem 1.5rem 1rem; display: flex; flex-direction: column; gap: 0.85rem; flex: 1; }

  .keyword-row { display: flex; flex-wrap: wrap; gap: 0.35rem; }
  .kw-chip { padding: 0.15rem 0.6rem; border-radius: 1rem; background: rgba(59,130,246,0.12); border: 1px solid rgba(59,130,246,0.3); color: #93c5fd; font-size: 0.75rem; }

  .badge { font-size: 0.68rem; padding: 0.1rem 0.45rem; border-radius: 0.25rem; font-weight: 600; letter-spacing: 0.03em; }
  .badge.live { background: rgba(167,139,250,0.15); color: #a78bfa; }
  .badge.upload { background: rgba(59,130,246,0.12); color: #60a5fa; }

  /* Replay */
  .replay-bar { display: flex; flex-direction: column; gap: 0.5rem; }
  .media-el { width: 100%; border-radius: 0.5rem; background: #000; max-height: 200px; object-fit: contain; }
  .audio-el { width: 100%; }

  .transcript-full { flex: 1; border: 1px solid #1e293b; border-radius: 0.5rem; display: flex; flex-direction: column; overflow-y: auto; }
  .qa-empty { color: #475569; font-style: italic; font-size: var(--fs-sm); padding: 2rem; text-align: center; }
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
  .share-saving { font-size: var(--fs-xs); color: #334155; font-style: italic; }
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

  /* ── AI Review / debrief tabs ── */
  .deb-loading { color: #475569; font-style: italic; text-align: center; padding: 2rem; font-size: var(--fs-base); }
  .deb-error { color: #f87171; font-size: var(--fs-sm); padding: 1rem; background: #1c0a0a; border: 1px solid #7f1d1d; border-radius: 0.4rem; }
  .deb-muted { color: #475569; font-style: italic; font-size: var(--fs-sm); margin: 0; }
  .deb-section { display: flex; flex-direction: column; gap: 0.5rem; }
  .deb-h3 { font-size: var(--fs-sm); font-weight: 700; text-transform: uppercase; letter-spacing: 0.07em; color: #64748b; margin: 0; }
  .deb-h3.deb-green { color: #4ade80; }
  .deb-h3.deb-yellow { color: #f59e0b; }
  .deb-h3.deb-amber { color: #fb923c; }
  .deb-summary { color: #cbd5e1; line-height: 1.6; font-size: var(--fs-base); margin: 0; }
  .deb-two-col { display: grid; grid-template-columns: 1fr 1fr; gap: 1.5rem; }
  .deb-list { margin: 0; padding-left: 1.25rem; display: flex; flex-direction: column; gap: 0.3rem; }
  .deb-list li { color: #94a3b8; font-size: 0.875rem; line-height: 1.5; }
  .deb-email-header { display: flex; align-items: center; justify-content: space-between; }
  .deb-copy-btn { padding: 0.22rem 0.65rem; background: transparent; border: 1px solid #334155; border-radius: 0.25rem; color: #64748b; font-size: var(--fs-sm); cursor: pointer; transition: all 0.15s; }
  .deb-copy-btn:hover { border-color: #60a5fa; color: #60a5fa; }
  .deb-copy-btn.copied { border-color: #4ade80; color: #4ade80; }
  .deb-gen-email-btn { margin-top: 0.75rem; padding: 0.3rem 0.8rem; background: transparent; border: 1px solid #334155; border-radius: 0.25rem; color: #94a3b8; font-size: var(--fs-sm); cursor: pointer; transition: all 0.15s; }
  .deb-gen-email-btn:hover { border-color: #818cf8; color: #818cf8; }
  .review-section-divider { display: flex; align-items: center; gap: 0.75rem; font-size: var(--fs-xs); font-weight: 700; text-transform: uppercase; letter-spacing: 0.07em; color: #334155; padding: 1rem 0 0.4rem; }
  .review-section-divider::after { content: ''; flex: 1; height: 1px; background: #1e293b; }
  .perf-chips-wrap { display: flex; flex-direction: column; gap: 0.6rem; }
  .perf-group { display: flex; align-items: center; flex-wrap: wrap; gap: 0.4rem; }
  .perf-group-label { font-size: var(--fs-xs); font-weight: 700; text-transform: uppercase; letter-spacing: 0.07em; color: #475569; width: 3.5rem; flex-shrink: 0; }
  .perf-chip { font-size: 0.78rem; padding: 0.2rem 0.55rem; border-radius: 999px; white-space: nowrap; }
  .perf-chip-filler { background: #1e293b; color: #64748b; }
  .perf-chip-hedge { background: #1c1407; border: 1px solid #78350f; color: #fbbf24; cursor: pointer; }
  .perf-chip-hedge:hover, .perf-chip-hedge.active { background: #292008; border-color: #f59e0b; color: #fde68a; }
  .perf-chip-wrap { position: relative; }
  .hedge-popover-fixed {
    position: fixed; z-index: 500;
    background: #07101e; border: 1px solid #1e3a5f; border-radius: 0.5rem;
    padding: 0.6rem 0.75rem; min-width: 200px; max-width: 280px;
    box-shadow: 0 8px 24px rgba(0,0,0,0.7);
  }
  .hedge-popover-title { font-size: var(--fs-xs); color: #475569; margin-bottom: 0.4rem; font-style: italic; }
  .hedge-popover-alt { font-size: 0.82rem; color: #94a3b8; padding: 0.25rem 0; border-bottom: 1px solid #0d1f35; line-height: 1.4; }
  .hedge-popover-alt:last-child { border-bottom: none; }
  .deb-list-pad { padding-left: 1.25rem; }
  .deb-email-draft { background: #060e1a; border: 1px solid #1a2d4a; border-radius: 0.5rem; padding: 0.85rem 1.1rem; }
  .deb-email-line { font-size: var(--fs-base); color: #cbd5e1; line-height: 1.7; }
  .deb-email-blank { height: 0.6rem; }

  /* ── Timeline / Practice shared ── */
  .deb-timeline { display: flex; flex-direction: column; gap: 0.75rem; }
  .tl-item { background: #060e1a; border: 1px solid #1a2d4a; border-left: 3px solid #1e3a5f; border-radius: 0.4rem; padding: 0.65rem 0.85rem; display: flex; flex-direction: column; gap: 0.4rem; }
  .tl-item-practice { border-left-color: #1e3a5f; }
  .tl-q-row { display: flex; align-items: baseline; gap: 0.45rem; flex-wrap: wrap; }
  .tl-tag { font-size: 0.65rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em; border-radius: 0.25em; padding: 0.05em 0.4em; flex-shrink: 0; }
  .tl-num { font-size: var(--fs-xs); color: #334155; font-weight: 700; background: #0d1f35; border: 1px solid #1e3a5f; border-radius: 0.2rem; padding: 0 0.3em; line-height: 1.6; flex-shrink: 0; }
  .tl-q-text { font-size: var(--fs-sm); color: #93c5fd; font-weight: 600; line-height: 1.4; flex: 1; }
  .tl-answer { display: flex; gap: 0.5rem; align-items: baseline; padding: 0.4rem 0.5rem; background: #0a1628; border-radius: 0.3rem; }
  .tl-answer-label { font-size: var(--fs-xs); font-weight: 700; text-transform: uppercase; color: #60a5fa; flex-shrink: 0; }
  .tl-answer-text { font-size: var(--fs-sm); color: #94a3b8; line-height: 1.5; flex: 1; }
  .tl-coaching { display: flex; flex-direction: column; gap: 0.3rem; }
  .tl-coaching-note { font-size: var(--fs-sm); color: #7dd3fc; line-height: 1.5; margin: 0; font-style: italic; }
  .tl-flags { display: flex; gap: 0.35rem; flex-wrap: wrap; }
  .tl-flag { font-size: var(--fs-xs); border: 1px solid; border-radius: 0.25em; padding: 0.05em 0.4em; font-weight: 600; }
  .tl-flag-warn { color: #f59e0b; border-color: #92400e; }
  .tl-flag-good { color: #4ade80; border-color: #166534; }
  .tl-toggle { background: none; border: none; color: #334155; font-size: var(--fs-xs); font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em; cursor: pointer; padding: 0; transition: color 0.12s; align-self: flex-start; }
  .tl-toggle:hover { color: #475569; }
  .tl-suggestion { font-size: var(--fs-sm); color: #475569; line-height: 1.5; white-space: pre-wrap; font-style: italic; border-left: 2px solid #1e293b; padding-left: 0.6rem; margin: 0; }
  .tl-practice-badge { margin-left: auto; font-size: var(--fs-xs); font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em; color: #3b82f6; background: #0c1f3a; border: 1px solid #1e3a5f; border-radius: 0.25em; padding: 0.05em 0.45em; }
  .tl-practice-badge-rec { color: #a78bfa; background: #130c2a; border-color: #3b1f7a; }
  .practice-label { font-size: var(--fs-xs); font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em; color: #334155; padding-bottom: 0.25rem; border-bottom: 1px solid #0f172a; }

  /* ── Recording tab ── */
  .rec-section { display: flex; flex-direction: column; gap: 0.75rem; }
  .rec-player { width: 100%; border-radius: 0.5rem; background: #000; max-height: 300px; object-fit: contain; }
</style>
