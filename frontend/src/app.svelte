<script lang="ts">
  import SetupForm from './components/SetupForm.svelte';
  import CaptureButton from './components/CaptureButton.svelte';
  import TranscriptPanel from './components/TranscriptPanel.svelte';
  import SentimentBar from './components/SentimentBar.svelte';
  import SuggestionPanel from './components/SuggestionPanel.svelte';
  import RateLimitPanel from './components/RateLimitPanel.svelte';
  import DraggablePanel from './components/DraggablePanel.svelte';
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
  let panelsVisible = $state(true);

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

  const youPct = $derived(
    youSegments + interviewerSegments === 0 ? 0 :
    Math.round((youSegments / (youSegments + interviewerSegments)) * 100)
  );
  const interviewerPct = $derived(youPct > 0 ? 100 - youPct : 0);
  const fillerTotal = $derived(totalFillers(allFillerCounts));

  // Rate limits
  interface RateLimitEntry { remaining: number; limit: number; history: Array<{ r: number; t: number }>; }
  let rateLimits = $state<Record<string, RateLimitEntry>>({});

  // WS status
  let wsStatus = $state('disconnected');
  let wsAttempt = $state(0);

  // Panel layout
  const PANEL_TITLES: Record<string, string> = {
    transcript: 'Live Transcript', suggestions: 'AI Suggestions',
    sentiment: 'Interviewer', ratelimits: 'API Usage',
  };
  const DEFAULT_ORDER = ['transcript', 'suggestions', 'sentiment', 'ratelimits'];

  function loadOrder(): string[] {
    try {
      const saved = localStorage.getItem('panel-order');
      if (saved) {
        const parsed: string[] = JSON.parse(saved);
        if (DEFAULT_ORDER.every(p => parsed.includes(p))) return parsed;
      }
    } catch {}
    return [...DEFAULT_ORDER];
  }

  let panelOrder = $state<string[]>(loadOrder());
  let draggedPanel = $state<string | null>(null);
  let dragOverPanel = $state<string | null>(null);

  function startDrag(id: string) { draggedPanel = id; }
  function setDragOver(id: string) { dragOverPanel = id; }
  function endDrag() { draggedPanel = null; dragOverPanel = null; }
  function drop(targetId: string) {
    if (!draggedPanel || draggedPanel === targetId) { endDrag(); return; }
    const next = [...panelOrder];
    const from = next.indexOf(draggedPanel), to = next.indexOf(targetId);
    [next[from], next[to]] = [next[to], next[from]];
    panelOrder = next;
    localStorage.setItem('panel-order', JSON.stringify(next));
    endDrag();
  }

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
          // Merge filler counts
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

  // Keyboard shortcuts
  $effect(() => {
    function onKey(e: KeyboardEvent) {
      if (phase !== 'interview') return;
      const tag = (e.target as HTMLElement).tagName;
      if (tag === 'INPUT' || tag === 'TEXTAREA') return;
      switch (e.key) {
        case 'h': case 'H': panelsVisible = !panelsVisible; break;
        case 'Escape': suggestions = suggestions.map(s => s.streaming ? s : { ...s, suggestion: '' }); break;
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
          <div class="shortcuts-hint">H: hide panels &nbsp;· &nbsp;P: pause &nbsp;· &nbsp;+/−: font size</div>
          <button class="debrief-btn" onclick={() => showDebrief = true}>End Interview</button>
          <CaptureButton onCapture={(v) => { capturing = v; }} />
        </div>
      </header>

      <StatsBar
        {answerMs}
        {youPct}
        {interviewerPct}
        fillerTotal={fillerTotal}
        fillerCounts={allFillerCounts}
        {wsStatus}
        {wsAttempt}
      />

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

      {#if panelsVisible}
        <div class="panel-grid">
          {#each panelOrder as panelId (panelId)}
            <DraggablePanel
              id={panelId} title={PANEL_TITLES[panelId]}
              isDragging={draggedPanel === panelId} isDragOver={dragOverPanel === panelId}
              onDragStart={startDrag} onDragOver={setDragOver} onDrop={drop} onDragEnd={endDrag}
            >
              {#if panelId === 'transcript'}
                <TranscriptPanel entries={transcript} />
              {:else if panelId === 'suggestions'}
                <SuggestionPanel {suggestions} onClear={() => (suggestions = [])} />
              {:else if panelId === 'sentiment'}
                <SentimentBar {emotion} {coaching} />
              {:else if panelId === 'ratelimits'}
                <RateLimitPanel {rateLimits} />
              {/if}
            </DraggablePanel>
          {/each}
        </div>
      {:else}
        <div class="panels-hidden-msg">Panels hidden — press H to show</div>
      {/if}
    </div>

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

  .interview-layout { display: flex; flex-direction: column; height: 100vh; }
  .interview-header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 0.5rem 1rem; background: #0f172a; border-bottom: 1px solid #1e293b; flex-shrink: 0;
  }
  .interview-header h1 {
    font-size: 1rem; font-weight: 700;
    background: linear-gradient(135deg, #60a5fa, #a78bfa);
    -webkit-background-clip: text; -webkit-text-fill-color: transparent;
  }
  .header-right { display: flex; align-items: center; gap: 0.75rem; }
  .shortcuts-hint { font-size: 0.65rem; color: #334155; white-space: nowrap; }
  .debrief-btn {
    padding: 0.35rem 0.9rem; background: transparent;
    border: 1px solid #334155; border-radius: 0.375rem;
    color: #64748b; font-size: 0.8rem; cursor: pointer; transition: all 0.15s; white-space: nowrap;
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
  .panel-grid {
    flex: 1; display: grid;
    grid-template-columns: 1fr 1fr; grid-template-rows: 1fr 1fr;
    gap: 6px; padding: 6px; overflow: hidden; background: #0a0f1a; min-height: 0;
  }
  .panels-hidden-msg {
    flex: 1; display: flex; align-items: center; justify-content: center;
    color: #334155; font-size: 0.9rem; font-style: italic;
  }
</style>
