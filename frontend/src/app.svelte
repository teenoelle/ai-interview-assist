<script lang="ts">
  import SetupForm from './components/SetupForm.svelte';
  import CaptureButton from './components/CaptureButton.svelte';
  import TranscriptPanel from './components/TranscriptPanel.svelte';
  import SentimentBar from './components/SentimentBar.svelte';
  import SuggestionPanel from './components/SuggestionPanel.svelte';
  import RateLimitPanel from './components/RateLimitPanel.svelte';
  import DraggablePanel from './components/DraggablePanel.svelte';
  import { EventWebSocket } from './lib/websocket';
  import type { TranscriptEntry, SuggestionEntry, WsEvent } from './lib/types';

  type Phase = 'setup' | 'interview';

  let phase = $state<Phase>('setup');
  let capturing = $state(false);
  let transcript = $state<TranscriptEntry[]>([]);
  let emotion = $state('');
  let coaching = $state('');
  let suggestions = $state<SuggestionEntry[]>([]);
  let statusMessages = $state<string[]>([]);
  let errorMessages = $state<string[]>([]);

  // ── Rate limits with usage history ──────────────────────────────────────────
  interface RateLimitEntry {
    remaining: number;
    limit: number;
    history: Array<{ r: number; t: number }>; // remaining, timestamp
  }
  let rateLimits = $state<Record<string, RateLimitEntry>>({});

  // ── Modular panel layout ─────────────────────────────────────────────────────
  const PANEL_TITLES: Record<string, string> = {
    transcript:  'Live Transcript',
    suggestions: 'AI Suggestions',
    sentiment:   'Interviewer',
    ratelimits:  'API Usage',
  };

  const DEFAULT_ORDER = ['transcript', 'suggestions', 'sentiment', 'ratelimits'];

  function loadOrder(): string[] {
    try {
      const saved = localStorage.getItem('panel-order');
      if (saved) {
        const parsed: string[] = JSON.parse(saved);
        // Ensure all expected panels are present
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
    const from = panelOrder.indexOf(draggedPanel);
    const to   = panelOrder.indexOf(targetId);
    const next = [...panelOrder];
    [next[from], next[to]] = [next[to], next[from]];
    panelOrder = next;
    localStorage.setItem('panel-order', JSON.stringify(next));
    endDrag();
  }

  // ── WebSocket ────────────────────────────────────────────────────────────────
  let eventWs: EventWebSocket | null = null;

  function handleSetupComplete() {
    phase = 'interview';
    eventWs = new EventWebSocket();
    eventWs.onEvent(handleWsEvent);
    eventWs.connect();
  }

  function handleWsEvent(event: WsEvent) {
    switch (event.type) {
      case 'transcript':
        transcript = [...transcript, { text: event.text, timestamp_ms: event.timestamp_ms, speaker: event.speaker }];
        break;
      case 'sentiment':
        emotion = event.emotion;
        if (event.coaching) coaching = event.coaching;
        break;
      case 'question_detected':
        suggestions = [...suggestions, { question: event.question, suggestion: '', streaming: true }];
        break;
      case 'suggestion_token':
        suggestions = suggestions.map((s, i) =>
          i === suggestions.length - 1 && s.streaming
            ? { ...s, suggestion: s.suggestion + event.token }
            : s
        );
        break;
      case 'suggestion_complete':
        suggestions = suggestions.map((s, i) =>
          i === suggestions.length - 1 && s.streaming
            ? { ...s, suggestion: event.full_text, streaming: false }
            : s
        );
        break;
      case 'status':
        statusMessages = [...statusMessages.slice(-4), event.message];
        break;
      case 'error':
        if (!errorMessages.includes(event.message)) {
          errorMessages = [...errorMessages, event.message];
        }
        break;
      case 'rate_limit': {
        const prev = rateLimits[event.provider];
        const point = { r: event.requests_remaining, t: Date.now() };
        const history = prev ? [...prev.history.slice(-14), point] : [point];
        rateLimits = {
          ...rateLimits,
          [event.provider]: { remaining: event.requests_remaining, limit: event.requests_limit, history },
        };
        break;
      }
    }
  }

  $effect(() => { void capturing; });
</script>

<main>
  {#if phase === 'setup'}
    <div class="setup-container">
      <header class="setup-header">
        <h1>AI Interview Assistant</h1>
        <p>Real-time AI coaching during your job interview</p>
      </header>
      <SetupForm onSetupComplete={handleSetupComplete} />
    </div>

  {:else}
    <div class="interview-layout">

      <header class="interview-header">
        <h1>AI Interview Assistant</h1>
        <CaptureButton onCapture={(v) => { capturing = v; }} />
      </header>

      {#if errorMessages.length > 0}
        <div class="error-banner">
          <div class="error-list">
            {#each errorMessages as msg}<div>{msg}</div>{/each}
          </div>
          <div class="error-actions">
            <button class="error-btn" onclick={() => navigator.clipboard.writeText(errorMessages.join('\n'))}>Copy</button>
            <button class="error-btn" onclick={() => (errorMessages = [])}>✕</button>
          </div>
        </div>
      {/if}

      {#if statusMessages.length > 0}
        <div class="status-banner">{statusMessages[statusMessages.length - 1]}</div>
      {/if}

      <div class="panel-grid">
        {#each panelOrder as panelId (panelId)}
          <DraggablePanel
            id={panelId}
            title={PANEL_TITLES[panelId]}
            isDragging={draggedPanel === panelId}
            isDragOver={dragOverPanel === panelId}
            onDragStart={startDrag}
            onDragOver={setDragOver}
            onDrop={drop}
            onDragEnd={endDrag}
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

    </div>
  {/if}
</main>

<style>
  main { min-height: 100vh; }

  /* ── Setup ──────────────────────────────────────────────────────────────── */
  .setup-container { max-width: 800px; margin: 0 auto; }
  .setup-header { text-align: center; padding: 3rem 2rem 1rem; }
  .setup-header h1 {
    font-size: 2.5rem; font-weight: 800;
    background: linear-gradient(135deg, #60a5fa, #a78bfa);
    -webkit-background-clip: text; -webkit-text-fill-color: transparent;
  }
  .setup-header p { color: #64748b; margin-top: 0.5rem; }

  /* ── Interview layout ───────────────────────────────────────────────────── */
  .interview-layout { display: flex; flex-direction: column; height: 100vh; }

  .interview-header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 0.6rem 1.25rem;
    background: #0f172a; border-bottom: 1px solid #1e293b;
    flex-shrink: 0;
  }
  .interview-header h1 {
    font-size: 1.1rem; font-weight: 700;
    background: linear-gradient(135deg, #60a5fa, #a78bfa);
    -webkit-background-clip: text; -webkit-text-fill-color: transparent;
  }

  .error-banner {
    display: flex; align-items: flex-start; gap: 0.75rem;
    padding: 0.5rem 1rem 0.5rem 1.5rem;
    background: #450a0a; color: #fca5a5; font-size: 0.8rem;
    flex-shrink: 0;
  }
  .error-list { flex: 1; max-height: 6rem; overflow-y: auto; }
  .error-actions {
    flex-shrink: 0; display: flex; flex-direction: column;
    gap: 0.25rem; align-self: flex-start; margin-top: 0.1rem;
  }
  .error-btn {
    padding: 0.15rem 0.5rem; background: transparent;
    border: 1px solid #7f1d1d; border-radius: 0.25rem;
    color: #fca5a5; font-size: 0.75rem; cursor: pointer; white-space: nowrap;
  }
  .error-btn:hover { background: #7f1d1d; }

  .status-banner {
    padding: 0.2rem 1.25rem; background: #1e3a5f;
    color: #93c5fd; font-size: 0.8rem; flex-shrink: 0;
  }

  /* ── Panel grid ─────────────────────────────────────────────────────────── */
  .panel-grid {
    flex: 1;
    display: grid;
    grid-template-columns: 1fr 1fr;
    grid-template-rows: 1fr 1fr;
    gap: 6px;
    padding: 6px;
    overflow: hidden;
    background: #0a0f1a;
    min-height: 0;
  }
</style>
