<script lang="ts">
  import SetupForm from './components/SetupForm.svelte';
  import CaptureButton from './components/CaptureButton.svelte';
  import TranscriptPanel from './components/TranscriptPanel.svelte';
  import SentimentBar from './components/SentimentBar.svelte';
  import SuggestionPanel from './components/SuggestionPanel.svelte';
  import { EventWebSocket } from './lib/websocket';
  import type { TranscriptEntry, SuggestionEntry, WsEvent } from './lib/types';

  type Phase = 'setup' | 'interview';

  let phase = $state<Phase>('setup');
  let capturing = $state(false);
  let transcript = $state<TranscriptEntry[]>([]);
  let emotion = $state('');
  let suggestions = $state<SuggestionEntry[]>([]);
  let statusMessages = $state<string[]>([]);
  let errorMessages = $state<string[]>([]);

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
        <div class="header-controls">
          <SentimentBar {emotion} />
          <CaptureButton onCapture={(v) => { capturing = v; }} />
        </div>
      </header>

      {#if errorMessages.length > 0}
        <div class="error-banner">
          <div class="error-list">
            {#each errorMessages as msg}
              <div>{msg}</div>
            {/each}
          </div>
          <div class="error-actions">
            <button class="error-btn" onclick={() => navigator.clipboard.writeText(errorMessages.join('\n'))}>Copy</button>
            <button class="error-btn" onclick={() => (errorMessages = [])}>✕</button>
          </div>
        </div>
      {/if}

      {#if statusMessages.length > 0}
        <div class="status-banner">
          {statusMessages[statusMessages.length - 1]}
        </div>
      {/if}

      <div class="panels">
        <div class="panel transcript">
          <TranscriptPanel entries={transcript} />
        </div>
        <div class="panel suggestions">
          <SuggestionPanel {suggestions} onClear={() => (suggestions = [])} />
        </div>
      </div>
    </div>
  {/if}
</main>

<style>
  main {
    min-height: 100vh;
  }

  /* Setup */
  .setup-container {
    max-width: 800px;
    margin: 0 auto;
  }
  .setup-header {
    text-align: center;
    padding: 3rem 2rem 1rem;
  }
  .setup-header h1 {
    font-size: 2.5rem;
    font-weight: 800;
    background: linear-gradient(135deg, #60a5fa, #a78bfa);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
  }
  .setup-header p {
    color: #64748b;
    margin-top: 0.5rem;
  }

  /* Interview layout */
  .interview-layout {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }
  .interview-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.75rem 1.5rem;
    background: #0f172a;
    border-bottom: 1px solid #1e293b;
  }
  .interview-header h1 {
    font-size: 1.25rem;
    font-weight: 700;
    background: linear-gradient(135deg, #60a5fa, #a78bfa);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
  }
  .header-controls {
    display: flex;
    align-items: center;
    gap: 1rem;
  }
  .error-banner {
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
    padding: 0.5rem 1rem 0.5rem 1.5rem;
    background: #450a0a;
    color: #fca5a5;
    font-size: 0.8rem;
  }
  .error-list {
    flex: 1;
    max-height: 6rem;
    overflow-y: auto;
  }
  .error-actions {
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    align-self: flex-start;
    margin-top: 0.1rem;
  }
  .error-btn {
    padding: 0.15rem 0.5rem;
    background: transparent;
    border: 1px solid #7f1d1d;
    border-radius: 0.25rem;
    color: #fca5a5;
    font-size: 0.75rem;
    cursor: pointer;
    white-space: nowrap;
  }
  .error-btn:hover {
    background: #7f1d1d;
  }
  .status-banner {
    padding: 0.25rem 1.5rem;
    background: #1e3a5f;
    color: #93c5fd;
    font-size: 0.8rem;
  }
  .panels {
    display: flex;
    flex: 1;
    overflow: hidden;
    gap: 1px;
    background: #1e293b;
  }
  .panel {
    flex: 1;
    background: #0f172a;
    padding: 1.25rem;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
  .transcript {
    flex: 1.2;
  }
  .suggestions {
    flex: 0.8;
  }
</style>
