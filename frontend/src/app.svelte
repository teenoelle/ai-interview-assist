<script lang="ts">
  import SetupForm from './components/SetupForm.svelte';
  import CaptureButton from './components/CaptureButton.svelte';
  import TranscriptPanel from './components/TranscriptPanel.svelte';
  import SentimentBar from './components/SentimentBar.svelte';
  import SuggestionPanel from './components/SuggestionPanel.svelte';
  import RateLimitPanel from './components/RateLimitPanel.svelte';
  import StatsBar from './components/StatsBar.svelte';
  import DebriefModal from './components/DebriefModal.svelte';
  import PracticePanel from './components/PracticePanel.svelte';
  import { EventWebSocket } from './lib/websocket';
  import { countFillers, totalFillers } from './lib/filler';
  import type { TranscriptEntry, SuggestionEntry, WsEvent } from './lib/types';
  import type { FillerCount } from './lib/filler';

  type Phase = 'setup' | 'practice' | 'interview';

  let phase = $state<Phase>('setup');
  let capturing = $state(false);
  let transcript = $state<TranscriptEntry[]>([]);
  let emotion = $state('');
  let coaching = $state('');
  let suggestions = $state<SuggestionEntry[]>([]);
  let statusMessages = $state<string[]>([]);
  let errorMessages = $state<string[]>([]);
  let predictedQuestions = $state<string[]>([]);
  let showDebrief = $state(false);
  let focusMode = $state(false);

  // Font size
  let fontSize = $state(Number(localStorage.getItem('font-size') ?? 14));
  $effect(() => {
    document.documentElement.style.setProperty('--font-size', `${fontSize}px`);
    localStorage.setItem('font-size', String(fontSize));
  });

  // Stats
  let answerStartTime = $state<number | null>(null);
  let answerMs = $state(0);
  let youSegments = $state(0);
  let interviewerSegments = $state(0);
  let allFillerCounts = $state<FillerCount[]>([]);
  let showFillers = $state(false);

  let answerInterval: ReturnType<typeof setInterval> | null = null;

  function startAnswerTimer() {
    if (answerStartTime !== null) return;
    answerStartTime = Date.now();
    answerInterval = setInterval(() => {
      if (answerStartTime !== null) answerMs = Date.now() - answerStartTime;
    }, 500);
  }

  function resetAnswerTimer() {
    answerStartTime = null;
    answerMs = 0;
    if (answerInterval) { clearInterval(answerInterval); answerInterval = null; }
  }

  function fmtTime(ms: number): string {
    const s = Math.floor(ms / 1000);
    const m = Math.floor(s / 60);
    return `${m}:${String(s % 60).padStart(2, '0')}`;
  }

  const youPct = $derived(
    youSegments + interviewerSegments === 0 ? 0 :
    Math.round((youSegments / (youSegments + interviewerSegments)) * 100)
  );
  const interviewerPct = $derived(youPct > 0 ? 100 - youPct : 0);
  const fillerTotal = $derived(totalFillers(allFillerCounts));
  const timerColor = $derived(
    answerMs === 0 ? '#475569' :
    answerMs < 90000 ? '#22c55e' :
    answerMs < 150000 ? '#f59e0b' : '#ef4444'
  );
  const ratioColor = $derived(
    youPct === 0 ? '#475569' : youPct < 65 ? '#22c55e' : '#f59e0b'
  );
  const latestSuggestion = $derived(
    suggestions.length > 0 ? suggestions[suggestions.length - 1] : null
  );

  // Rate limits
  interface RateLimitEntry { remaining: number; limit: number; history: Array<{ r: number; t: number }>; }
  let rateLimits = $state<Record<string, RateLimitEntry>>({});

  // WS status
  let wsStatus = $state('disconnected');
  let wsAttempt = $state(0);

  // WebSocket
  let eventWs: EventWebSocket | null = null;

  function handleSetupComplete() {
    phase = 'interview';
    connectWs();
  }

  function handlePractice(questions: string[]) {
    predictedQuestions = questions;
    phase = 'practice';
    connectWs();
  }

  function connectWs() {
    eventWs = new EventWebSocket();
    eventWs.onEvent(handleWsEvent);
    eventWs.onStatus((status, attempt) => { wsStatus = status; wsAttempt = attempt; });
    eventWs.connect();
  }

  function handleWsEvent(event: WsEvent) {
    switch (event.type) {
      case 'transcript': {
        const entry = { text: event.text, timestamp_ms: event.timestamp_ms, speaker: event.speaker };
        transcript = [...transcript, entry];
        if (event.speaker === 'You') {
          youSegments++;
          startAnswerTimer();
          const newCounts = countFillers(event.text);
          const merged: Record<string, number> = {};
          for (const f of allFillerCounts) merged[f.word] = f.count;
          for (const f of newCounts) merged[f.word] = (merged[f.word] ?? 0) + f.count;
          allFillerCounts = Object.entries(merged).map(([word, count]) => ({ word, count }));
        } else if (event.speaker === 'Interviewer') {
          interviewerSegments++;
          resetAnswerTimer();
        }
        break;
      }
      case 'sentiment':
        emotion = event.emotion;
        if (event.coaching) coaching = event.coaching;
        break;
      case 'question_detected':
        suggestions = [...suggestions, { question: event.question, suggestion: '', streaming: true }];
        resetAnswerTimer();
        break;
      case 'suggestion_token':
        suggestions = suggestions.map((s, i) =>
          i === suggestions.length - 1 && s.streaming ? { ...s, suggestion: s.suggestion + event.token } : s
        );
        break;
      case 'suggestion_complete':
        suggestions = suggestions.map((s, i) =>
          i === suggestions.length - 1 && s.streaming ? { ...s, suggestion: event.full_text, streaming: false } : s
        );
        break;
      case 'status':
        statusMessages = [...statusMessages.slice(-4), event.message];
        break;
      case 'error':
        if (!errorMessages.includes(event.message)) errorMessages = [...errorMessages, event.message];
        break;
      case 'rate_limit': {
        const prev = rateLimits[event.provider];
        const point = { r: event.requests_remaining, t: Date.now() };
        const history = prev ? [...prev.history.slice(-14), point] : [point];
        rateLimits = { ...rateLimits, [event.provider]: { remaining: event.requests_remaining, limit: event.requests_limit, history } };
        break;
      }
    }
  }

  function renderBold(text: string): string {
    return text.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>');
  }

  // Keyboard shortcuts
  $effect(() => {
    function onKey(e: KeyboardEvent) {
      if (phase !== 'interview') return;
      const tag = (e.target as HTMLElement).tagName;
      if (tag === 'INPUT' || tag === 'TEXTAREA') return;
      switch (e.key) {
        case 'f': case 'F': focusMode = !focusMode; break;
        case 'Escape':
          if (focusMode) { focusMode = false; break; }
          suggestions = suggestions.map(s => s.streaming ? s : { ...s, suggestion: '' });
          break;
        case '+': case '=': fontSize = Math.min(20, fontSize + 1); break;
        case '-': case '_': fontSize = Math.max(11, fontSize - 1); break;
      }
    }
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });

  $effect(() => { void capturing; });
</script>

<main style="font-size: var(--font-size, 14px)">
  {#if phase === 'setup'}
    <div class="setup-container">
      <header class="setup-header">
        <h1>AI Interview Assistant</h1>
        <p>Real-time AI coaching during your job interview</p>
      </header>
      <SetupForm onSetupComplete={handleSetupComplete} onPractice={handlePractice} />
    </div>

  {:else if phase === 'practice'}
    <PracticePanel
      questions={predictedQuestions}
      systemPrompt=""
      onStartInterview={() => { phase = 'interview'; }}
    />

  {:else}
    <div class="interview-layout">
      <header class="interview-header">
        <h1>AI Interview Assistant</h1>
        <div class="header-right">
          <div class="shortcuts-hint">F: focus &nbsp;· &nbsp;Esc: clear &nbsp;· &nbsp;+/−: font</div>
          <button class="debrief-btn" onclick={() => showDebrief = true}>End Interview</button>
          <CaptureButton onCapture={(v) => { capturing = v; }} />
        </div>
      </header>

      {#if errorMessages.length > 0}
        <div class="error-banner">
          <div class="error-list">{#each errorMessages as msg}<div>{msg}</div>{/each}</div>
          <div class="error-actions">
            <button class="error-btn" onclick={() => navigator.clipboard.writeText(errorMessages.join('\n'))}>Copy</button>
            <button class="error-btn" onclick={() => (errorMessages = [])}>✕</button>
          </div>
        </div>
      {/if}

      {#if statusMessages.length > 0}
        <div class="status-banner">{statusMessages[statusMessages.length - 1]}</div>
      {/if}

      <!-- 3-column teleprompter layout -->
      <div class="three-col">
        <!-- Left: Transcript (dim reference) -->
        <div class="col col-left">
          <div class="col-label">Transcript</div>
          <div class="col-body">
            <TranscriptPanel entries={transcript} />
          </div>
        </div>

        <!-- Center: AI Suggestions (teleprompter — directly under webcam) -->
        <div class="col col-center">
          <div class="col-body">
            <SuggestionPanel {suggestions} onClear={() => (suggestions = [])} teleprompter={true} />
          </div>
        </div>

        <!-- Right: Sentiment + Stats + Rate limits -->
        <div class="col col-right">
          <div class="col-label">Interviewer</div>
          <div class="col-body col-right-body">
            <SentimentBar {emotion} {coaching} />

            <div class="side-stats">
              <div class="side-stat" title="Time since you started your current answer">
                <span class="side-label">Answer</span>
                <span class="side-value" style="color: {timerColor}">
                  {answerMs > 0 ? fmtTime(answerMs) : '—'}
                </span>
              </div>
              <div class="side-stat" title="Your share of speaking time vs interviewer">
                <span class="side-label">You / Them</span>
                <span class="side-value" style="color: {ratioColor}">
                  {youPct > 0 ? `${youPct}% / ${interviewerPct}%` : '—'}
                </span>
              </div>
              <div class="side-stat filler-stat" title="Filler word count">
                <span class="side-label">Fillers</span>
                <button
                  class="filler-btn"
                  class:has-fillers={fillerTotal > 0}
                  onclick={() => showFillers = !showFillers}
                >{fillerTotal > 0 ? fillerTotal : '—'}</button>
                {#if showFillers && allFillerCounts.length > 0}
                  <div class="filler-popup">
                    {#each allFillerCounts as f}
                      <span class="filler-item">"{f.word}" ×{f.count}</span>
                    {/each}
                  </div>
                {/if}
              </div>
              <div class="side-stat" title="WebSocket connection">
                <span class="side-label">WS</span>
                <span
                  class="side-value ws-dot"
                  class:connected={wsStatus === 'connected'}
                  class:reconnecting={wsStatus === 'reconnecting'}
                >
                  {wsStatus === 'connected' ? '●' : wsStatus === 'reconnecting' ? `↻ #${wsAttempt}` : '○'}
                </span>
              </div>
            </div>

            <div class="side-ratelimits">
              <div class="col-label" style="margin-bottom: 0.5rem;">API Usage</div>
              <RateLimitPanel {rateLimits} />
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Focus mode overlay (F key) — suggestion fills screen near webcam -->
    {#if focusMode}
      <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
      <div class="focus-overlay" onclick={() => focusMode = false}>
        <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
        <div class="focus-card" onclick={(e) => e.stopPropagation()}>
          {#if latestSuggestion}
            <div class="focus-question">"{latestSuggestion.question}"</div>
            <div class="focus-suggestion">
              {#if latestSuggestion.suggestion}
                {@html renderBold(latestSuggestion.suggestion)}
                {#if latestSuggestion.streaming}<span class="focus-cursor">|</span>{/if}
              {:else if latestSuggestion.streaming}
                <span class="focus-loading">Generating...</span>
              {/if}
            </div>
          {:else}
            <div class="focus-empty">Waiting for a question...</div>
          {/if}
        </div>
        <div class="focus-hint">glance at bold keywords &nbsp;· &nbsp;F or click outside to exit</div>
      </div>
    {/if}

    {#if showDebrief}
      <DebriefModal
        {transcript} {suggestions}
        onClose={() => showDebrief = false}
      />
    {/if}
  {/if}
</main>

<style>
  main { min-height: 100vh; }

  .setup-container { max-width: 800px; margin: 0 auto; }
  .setup-header { text-align: center; padding: 3rem 2rem 1rem; }
  .setup-header h1 {
    font-size: 2.5rem; font-weight: 800;
    background: linear-gradient(135deg, #60a5fa, #a78bfa);
    -webkit-background-clip: text; -webkit-text-fill-color: transparent;
  }
  .setup-header p { color: #64748b; margin-top: 0.5rem; }

  /* Interview layout */
  .interview-layout { display: flex; flex-direction: column; height: 100vh; }
  .interview-header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 0.4rem 1rem; background: #0f172a; border-bottom: 1px solid #1e293b; flex-shrink: 0;
  }
  .interview-header h1 {
    font-size: 0.9rem; font-weight: 700;
    background: linear-gradient(135deg, #60a5fa, #a78bfa);
    -webkit-background-clip: text; -webkit-text-fill-color: transparent;
  }
  .header-right { display: flex; align-items: center; gap: 0.75rem; }
  .shortcuts-hint { font-size: 0.62rem; color: #334155; white-space: nowrap; }
  .debrief-btn {
    padding: 0.3rem 0.8rem; background: transparent;
    border: 1px solid #334155; border-radius: 0.375rem;
    color: #64748b; font-size: 0.75rem; cursor: pointer; transition: all 0.15s; white-space: nowrap;
  }
  .debrief-btn:hover { border-color: #a78bfa; color: #a78bfa; }

  .error-banner {
    display: flex; align-items: flex-start; gap: 0.75rem;
    padding: 0.5rem 1rem; background: #450a0a; color: #fca5a5; font-size: 0.8rem; flex-shrink: 0;
  }
  .error-list { flex: 1; max-height: 6rem; overflow-y: auto; }
  .error-actions { flex-shrink: 0; display: flex; flex-direction: column; gap: 0.25rem; }
  .error-btn {
    padding: 0.15rem 0.5rem; background: transparent;
    border: 1px solid #7f1d1d; border-radius: 0.25rem;
    color: #fca5a5; font-size: 0.75rem; cursor: pointer;
  }
  .error-btn:hover { background: #7f1d1d; }
  .status-banner {
    padding: 0.2rem 1rem; background: #1e3a5f; color: #93c5fd; font-size: 0.8rem; flex-shrink: 0;
  }

  /* 3-column layout */
  .three-col {
    flex: 1;
    display: grid;
    grid-template-columns: 22% 1fr 26%;
    gap: 0;
    overflow: hidden;
    background: #070c14;
    min-height: 0;
  }

  .col {
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border-right: 1px solid #0f172a;
  }
  .col:last-child { border-right: none; }

  .col-label {
    font-size: 0.6rem;
    font-weight: 700;
    color: #334155;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    padding: 0.35rem 0.75rem 0;
    flex-shrink: 0;
  }

  .col-body {
    flex: 1;
    overflow: hidden;
    padding: 0.5rem 0.75rem 0.75rem;
    display: flex;
    flex-direction: column;
  }

  /* Left column: transcript — slightly dimmer */
  .col-left { background: #080d18; }
  .col-left .col-body { opacity: 0.75; }
  .col-left .col-body:hover { opacity: 1; transition: opacity 0.2s; }

  /* Center column: suggestions — bright focal point */
  .col-center {
    background: #07101e;
    border-right: 1px solid #1e293b;
    border-left: 1px solid #1e293b;
  }
  .col-center .col-body { padding: 0.75rem 1rem; }

  /* Right column: stats */
  .col-right { background: #080d18; }
  .col-right-body {
    gap: 0.75rem;
    overflow-y: auto;
  }

  /* Side stats */
  .side-stats {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    padding: 0.5rem 0.25rem;
    border-top: 1px solid #1e293b;
    border-bottom: 1px solid #1e293b;
    flex-shrink: 0;
  }
  .side-stat {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.2rem 0.25rem;
    position: relative;
  }
  .side-label {
    font-size: 0.62rem;
    color: #475569;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    font-weight: 600;
  }
  .side-value {
    font-size: 0.75rem;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    color: #475569;
  }
  .ws-dot { font-size: 0.8rem; }
  .ws-dot.connected { color: #22c55e; }
  .ws-dot.reconnecting { color: #f59e0b; }
  .filler-stat { position: relative; }
  .filler-btn {
    background: none; border: none; cursor: pointer;
    font-size: 0.75rem; font-weight: 700; color: #475569; padding: 0;
  }
  .filler-btn.has-fillers { color: #f59e0b; }
  .filler-popup {
    position: absolute; top: 100%; right: 0; z-index: 50;
    background: #1e293b; border: 1px solid #334155;
    border-radius: 0.375rem; padding: 0.5rem;
    display: flex; flex-direction: column; gap: 0.2rem;
    white-space: nowrap; min-width: 120px;
  }
  .filler-item { font-size: 0.72rem; color: #f59e0b; }

  .side-ratelimits { flex: 1; overflow-y: auto; min-height: 0; }

  /* Focus mode overlay */
  .focus-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.93);
    z-index: 1000;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 1.5rem 2rem 2rem;
    cursor: pointer;
  }

  .focus-card {
    width: 100%;
    max-width: 680px;
    background: #07101e;
    border: 1px solid #1e3a5f;
    border-radius: 1rem;
    padding: 1.75rem 2rem;
    cursor: default;
    box-shadow: 0 0 60px rgba(59, 130, 246, 0.08);
  }

  .focus-question {
    color: #60a5fa;
    font-style: italic;
    font-size: 0.95rem;
    line-height: 1.5;
    margin-bottom: 1.25rem;
    padding-bottom: 1rem;
    border-bottom: 1px solid #1e293b;
  }

  .focus-suggestion {
    color: #cbd5e1;
    line-height: 2.4;
    white-space: pre-wrap;
    font-size: 1.05rem;
  }

  :global(.focus-suggestion strong) {
    color: #ffffff;
    font-size: 1.45rem;
    font-weight: 800;
    letter-spacing: 0.01em;
  }

  .focus-cursor {
    animation: blink 1s step-end infinite;
    color: #60a5fa;
  }
  @keyframes blink { 50% { opacity: 0; } }

  .focus-loading { color: #60a5fa; font-style: italic; }

  .focus-empty {
    color: #334155;
    font-style: italic;
    font-size: 1rem;
    text-align: center;
    padding: 3rem 0;
  }

  .focus-hint {
    margin-top: 0.75rem;
    font-size: 0.65rem;
    color: #1e293b;
  }
</style>
