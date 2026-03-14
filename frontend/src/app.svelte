<script lang="ts">
  import SetupForm from './components/SetupForm.svelte';
  import CaptureButton from './components/CaptureButton.svelte';
  import TranscriptPanel from './components/TranscriptPanel.svelte';
  import SentimentBar from './components/SentimentBar.svelte';
  import BodyLanguagePanel from './components/BodyLanguagePanel.svelte';
  import SuggestionPanel from './components/SuggestionPanel.svelte';
  import RateLimitPanel from './components/RateLimitPanel.svelte';
  import DebriefModal from './components/DebriefModal.svelte';
  import PracticePanel from './components/PracticePanel.svelte';
  import { EventWebSocket } from './lib/websocket';
  import { countFillers, totalFillers } from './lib/filler';
  import type { TranscriptEntry, SuggestionEntry, WsEvent } from './lib/types';
  import type { FillerCount } from './lib/filler';
  import type { MediaCapture } from './lib/capture';

  type Phase = 'setup' | 'practice' | 'interview';

  let phase = $state<Phase>('setup');
  let capturing = $state(false);
  let transcript = $state<TranscriptEntry[]>([]);
  let emotion = $state('');
  let emotionReason = $state('');
  let coaching = $state('');
  let coachingWhy = $state('');
  let suggestions = $state<SuggestionEntry[]>([]);
  let statusMessages = $state<string[]>([]);
  let errorMessages = $state<string[]>([]);
  let predictedQuestions = $state<string[]>([]);
  let showDebrief = $state(false);
  let focusMode = $state(false);

  // Capture instance (for triggerFrameCapture)
  let captureInst = $state<MediaCapture | null>(null);
  let lastSentimentTrigger = 0;

  // Webcam self-view
  let webcamStream = $state<MediaStream | null>(null);
  let webcamEl: HTMLVideoElement | undefined = $state();
  $effect(() => {
    if (webcamEl && webcamStream) webcamEl.srcObject = webcamStream;
  });

  // Screen share preview (shows interviewer's video in Zoom/Teams)
  let screenStream = $state<MediaStream | null>(null);
  let screenEl: HTMLVideoElement | undefined = $state();
  $effect(() => {
    if (screenEl && screenStream) screenEl.srcObject = screenStream;
  });

  // Column widths (resizable, persisted)
  let leftW = $state(Number(localStorage.getItem('col-left') ?? 240));
  let centerW = $state(Number(localStorage.getItem('col-center') ?? 320));

  function startResize(col: 'left' | 'center', e: MouseEvent) {
    e.preventDefault();
    const startX = e.clientX;
    const startW = col === 'left' ? leftW : centerW;
    const min = col === 'left' ? 150 : 200;
    const max = col === 'left' ? 450 : 550;
    function onMove(ev: MouseEvent) {
      const w = Math.max(min, Math.min(max, startW + ev.clientX - startX));
      if (col === 'left') leftW = w; else centerW = w;
    }
    function onUp() {
      window.removeEventListener('mousemove', onMove);
      window.removeEventListener('mouseup', onUp);
      localStorage.setItem('col-left', String(leftW));
      localStorage.setItem('col-center', String(centerW));
    }
    window.addEventListener('mousemove', onMove);
    window.addEventListener('mouseup', onUp);
  }

  // TTS voice hints
  let ttsEnabled = $state(false);
  let ttsVoices = $state<SpeechSynthesisVoice[]>([]);
  let ttsVoiceURI = $state('');
  let ttsRate = $state(Number(localStorage.getItem('tts-rate') ?? 1.5));
  let ttsVolume = $state(Number(localStorage.getItem('tts-volume') ?? 1.0));
  let showVoiceMenu = $state(false);
  // Silence gating: track last time anyone spoke
  let lastSpeechAt = 0; // ms timestamp
  const TTS_SILENCE_GAP_MS = 2500; // wait this long after last speech before speaking
  const TTS_MAX_WORDS = 12; // ~3s at 1.5x rate

  function loadVoices() {
    const voices = speechSynthesis.getVoices();
    if (voices.length > 0) {
      ttsVoices = voices;
      if (!ttsVoiceURI) ttsVoiceURI = voices[0]?.voiceURI ?? '';
    }
  }
  $effect(() => {
    loadVoices();
    speechSynthesis.addEventListener('voiceschanged', loadVoices);
    return () => speechSynthesis.removeEventListener('voiceschanged', loadVoices);
  });
  $effect(() => { localStorage.setItem('tts-rate', String(ttsRate)); });
  $effect(() => { localStorage.setItem('tts-volume', String(ttsVolume)); });

  function speakText(text: string) {
    if (!ttsEnabled || !text) return;
    // Don't speak if someone spoke within TTS_SILENCE_GAP_MS
    if (Date.now() - lastSpeechAt < TTS_SILENCE_GAP_MS) return;
    // Also don't speak if user is actively answering
    if (answerStartTime !== null) return;
    // Trim to max words
    const words = text.split(/\s+/);
    const trimmed = words.slice(0, TTS_MAX_WORDS).join(' ');
    speechSynthesis.cancel();
    const utt = new SpeechSynthesisUtterance(trimmed);
    const voice = ttsVoices.find(v => v.voiceURI === ttsVoiceURI);
    if (voice) utt.voice = voice;
    utt.rate = ttsRate;
    utt.volume = ttsVolume;
    speechSynthesis.speak(utt);
  }

  // Audio sentiment (client-side, free — based on interviewer text)
  let audioEmotion = $state('');
  let audioReason = $state('');

  function analyzeAudioTone(text: string): { emotion: string; reason: string } {
    const lower = text.toLowerCase();
    const excited = ['excellent', 'impressive', 'love that', 'great answer', 'fantastic', 'brilliant', 'perfect'].some(w => lower.includes(w));
    const skeptical = ['however,', 'but ', "i'm not sure", "don't think", 'concern', 'challenging', 'struggle', 'worry'].some(w => lower.includes(w));
    const closing = ['thank you for', "we'll be in touch", 'next steps', 'any questions for us', 'do you have any questions'].some(w => lower.includes(w));
    const curious = (lower.includes('?') || ['interesting', 'tell me more', 'curious', 'explain'].some(w => lower.includes(w)));
    if (excited) return { emotion: 'enthusiastic', reason: 'positive affirming language' };
    if (skeptical) return { emotion: 'skeptical', reason: 'qualifying/hedging language' };
    if (closing) return { emotion: 'wrapping up', reason: 'closing language detected' };
    if (curious) return { emotion: 'curious', reason: 'inquiry or question language' };
    return { emotion: 'neutral', reason: 'neutral conversational tone' };
  }

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
    answerMs < 15000 ? '#22c55e' :
    answerMs < 30000 ? '#f59e0b' : '#ef4444'
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

  let eventWs: EventWebSocket | null = null;

  function handleSetupComplete() { phase = 'interview'; connectWs(); }
  function handlePractice(questions: string[]) { predictedQuestions = questions; phase = 'practice'; connectWs(); }

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
          lastSpeechAt = Date.now();
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
          const tone = analyzeAudioTone(event.text);
          audioEmotion = tone.emotion;
          audioReason = tone.reason;
          // If we haven't captured a sentiment frame recently, trigger one now
          // (catches interviewer turning camera on mid-interview)
          const now = Date.now();
          if (captureInst && now - lastSentimentTrigger > 10000) {
            lastSentimentTrigger = now;
            captureInst.triggerFrameCapture();
          }
        }
        break;
      }
      case 'sentiment':
        emotion = event.emotion;
        if (event.reason) emotionReason = event.reason;
        if (event.coaching) coaching = event.coaching;
        if (event.coaching_why) coachingWhy = event.coaching_why;
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
      case 'suggestion_complete': {
        suggestions = suggestions.map((s, i) =>
          i === suggestions.length - 1 && s.streaming ? { ...s, suggestion: event.full_text, streaming: false } : s
        );
        // Speak only the Say: cue line, capped at TTS_MAX_WORDS
        const sayLine = event.full_text.split('\n')[0]
          ?.replace(/^(Say|Answer|Tell|Ask):\s*/i, '')
          ?.trim();
        if (sayLine) speakText(sayLine);
        break;
      }
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

  $effect(() => {
    function onKey(e: KeyboardEvent) {
      if (phase !== 'interview') return;
      const tag = (e.target as HTMLElement).tagName;
      if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return;
      switch (e.key) {
        case 'f': case 'F': focusMode = !focusMode; break;
        case 'Escape':
          if (focusMode) { focusMode = false; break; }
          suggestions = suggestions.map(s => s.streaming ? s : { ...s, suggestion: '' });
          break;
        case '+': case '=': fontSize = Math.min(20, fontSize + 1); break;
        case '-': case '_': fontSize = Math.max(11, fontSize - 1); break;
        case 't': case 'T': if (!showVoiceMenu) ttsEnabled = !ttsEnabled; break;
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
          <div class="shortcuts-hint">F: focus · T: voice · Esc: clear · +/−: font</div>

          <!-- TTS controls -->
          <div class="tts-controls">
            <button
              class="tts-btn"
              class:tts-on={ttsEnabled}
              onclick={() => ttsEnabled = !ttsEnabled}
              title="Toggle voice hints (T)"
            >{ttsEnabled ? '🔊' : '🔇'} Voice</button>

            {#if ttsEnabled}
              <!-- Speed slider -->
              <label class="rate-label" title="Speech speed">
                <span class="rate-val">{ttsRate.toFixed(1)}×</span>
                <input type="range" min="0.7" max="2.0" step="0.1" bind:value={ttsRate} class="rate-slider" />
              </label>
              <!-- Volume slider -->
              <label class="rate-label" title="Voice volume">
                <span class="rate-val">{Math.round(ttsVolume * 100)}%</span>
                <input type="range" min="0" max="1" step="0.05" bind:value={ttsVolume} class="rate-slider vol-slider" />
              </label>
              <button class="voice-pick-btn" onclick={() => showVoiceMenu = !showVoiceMenu} title="Choose voice">▾</button>
            {/if}

            {#if showVoiceMenu}
              <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
              <div class="voice-menu" role="menu" onmouseleave={() => showVoiceMenu = false}>
                {#each ttsVoices as v}
                  <button
                    class="voice-option"
                    class:selected={v.voiceURI === ttsVoiceURI}
                    onclick={() => { ttsVoiceURI = v.voiceURI; showVoiceMenu = false; }}
                  >{v.name} ({v.lang})</button>
                {/each}
              </div>
            {/if}
          </div>

          <button class="debrief-btn" onclick={() => showDebrief = true}>End Interview</button>
          <CaptureButton
            onCapture={(v) => { capturing = v; if (!v) { webcamStream = null; screenStream = null; captureInst = null; } }}
            onStreams={(screen, webcam) => { screenStream = screen; webcamStream = webcam; }}
            onReady={(cap) => { captureInst = cap; }}
          />
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

      <!-- Resizable 3-column layout -->
      <div class="three-col">
        <!-- Left: Transcript -->
        <div class="col col-left" style="width: {leftW}px">
          <div class="col-label">Transcript</div>
          <div class="col-body">
            <TranscriptPanel entries={transcript} />
          </div>
        </div>

        <!-- Resize handle: left | center -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="resize-handle" onmousedown={(e) => startResize('left', e)} title="Drag to resize"></div>

        <!-- Center: AI Suggestions -->
        <div class="col col-center" style="width: {centerW}px">
          {#if webcamStream}
            <div class="selfview-strip">
              <!-- svelte-ignore a11y_media_has_caption -->
              <video bind:this={webcamEl} class="selfview" autoplay muted playsinline></video>
              <div class="selfview-label">You</div>
            </div>
          {/if}
          <div class="col-body">
            <SuggestionPanel {suggestions} onClear={() => (suggestions = [])} teleprompter={true} />
          </div>
        </div>

        <!-- Resize handle: center | right -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="resize-handle" onmousedown={(e) => startResize('center', e)} title="Drag to resize"></div>

        <!-- Right: Sentiment + Stats -->
        <div class="col col-right">
          <div class="col-label">Interviewer</div>
          {#if screenStream}
            <div class="interviewer-preview">
              <!-- svelte-ignore a11y_media_has_caption -->
              <video bind:this={screenEl} class="interviewer-video" autoplay muted playsinline></video>
              <div class="interviewer-label">Live Screen · Sentiment from interviewer's camera</div>
            </div>
          {/if}
          <div class="col-body col-right-body">
            <SentimentBar
              videoEmotion={emotion}
              videoReason={emotionReason}
              {coaching}
              {audioEmotion}
              {audioReason}
            />

            <BodyLanguagePanel emotion={emotion} coaching={coaching} coachingWhy={coachingWhy} />

            <div class="side-stats">
              <div class="side-stat" title="Time since you started answering — aim for under 30 seconds">
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
                <button class="filler-btn" class:has-fillers={fillerTotal > 0} onclick={() => showFillers = !showFillers}>
                  {fillerTotal > 0 ? fillerTotal : '—'}
                </button>
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
                <span class="side-value ws-dot" class:connected={wsStatus === 'connected'} class:reconnecting={wsStatus === 'reconnecting'}>
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
        <div class="focus-hint">glance at bold keywords · F or click outside to exit</div>
      </div>
    {/if}

    {#if showDebrief}
      <DebriefModal {transcript} {suggestions} onClose={() => showDebrief = false} />
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
    color: #64748b; font-size: 0.75rem; cursor: pointer; white-space: nowrap;
  }
  .debrief-btn:hover { border-color: #a78bfa; color: #a78bfa; }

  /* TTS */
  .tts-controls { position: relative; display: flex; align-items: center; gap: 0.25rem; }
  .tts-btn {
    padding: 0.25rem 0.5rem; background: transparent;
    border: 1px solid #334155; border-radius: 0.375rem;
    color: #64748b; font-size: 0.72rem; cursor: pointer; white-space: nowrap;
  }
  .tts-btn.tts-on { border-color: #22c55e; color: #22c55e; }
  .rate-label {
    display: flex; align-items: center; gap: 0.25rem;
    font-size: 0.68rem; color: #64748b;
  }
  .rate-val { min-width: 2rem; text-align: right; font-variant-numeric: tabular-nums; }
  .rate-slider { width: 56px; accent-color: #22c55e; cursor: pointer; }
  .vol-slider { accent-color: #60a5fa; }
  .voice-pick-btn {
    padding: 0.2rem 0.35rem; background: transparent;
    border: 1px solid #334155; border-radius: 0.25rem;
    color: #64748b; font-size: 0.75rem; cursor: pointer;
  }
  .voice-menu {
    position: absolute; top: calc(100% + 4px); right: 0; z-index: 200;
    background: #1e293b; border: 1px solid #334155; border-radius: 0.375rem;
    min-width: 200px; max-height: 250px; overflow-y: auto;
    display: flex; flex-direction: column;
  }
  .voice-option {
    padding: 0.3rem 0.75rem; background: transparent; border: none;
    color: #94a3b8; font-size: 0.72rem; cursor: pointer; text-align: left;
  }
  .voice-option:hover { background: #334155; color: #e2e8f0; }
  .voice-option.selected { color: #60a5fa; }

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
  .status-banner { padding: 0.2rem 1rem; background: #1e3a5f; color: #93c5fd; font-size: 0.8rem; flex-shrink: 0; }

  /* Resizable 3-column layout */
  .three-col {
    flex: 1;
    display: flex;
    overflow: hidden;
    background: #070c14;
    min-height: 0;
  }

  .col {
    display: flex;
    flex-direction: column;
    overflow: hidden;
    flex-shrink: 0;
  }

  .col-right {
    flex: 1; /* right column takes remaining space */
    min-width: 180px;
    background: #080d18;
  }

  /* Drag resize handles */
  .resize-handle {
    width: 5px;
    flex-shrink: 0;
    background: #0f172a;
    cursor: col-resize;
    transition: background 0.15s;
  }
  .resize-handle:hover { background: #1e293b; }

  .col-label {
    font-size: 0.6rem; font-weight: 700; color: #334155;
    text-transform: uppercase; letter-spacing: 0.1em;
    padding: 0.35rem 0.75rem 0; flex-shrink: 0;
  }
  .col-body {
    flex: 1; overflow: hidden;
    padding: 0.5rem 0.75rem 0.75rem;
    display: flex; flex-direction: column;
  }

  .col-left { background: #080d18; }
  .col-left .col-body { opacity: 0.75; }
  .col-left .col-body:hover { opacity: 1; transition: opacity 0.2s; }

  .col-center {
    background: #07101e;
    border-right: 1px solid #0f172a;
    border-left: 1px solid #0f172a;
  }
  .col-center .col-body { padding: 0.5rem 0.75rem; }

  /* Webcam self-view */
  .selfview-strip {
    display: flex; align-items: center; gap: 0.5rem;
    padding: 0.35rem 0.75rem; border-bottom: 1px solid #1e293b;
    background: #060e1a; flex-shrink: 0;
  }
  .selfview {
    width: 80px; height: 60px; object-fit: cover;
    border-radius: 0.375rem; border: 1px solid #1e293b;
    background: #0f172a; transform: scaleX(-1);
  }
  .selfview-label { font-size: 0.6rem; color: #334155; text-transform: uppercase; letter-spacing: 0.08em; }

  /* Interviewer screen preview */
  .interviewer-preview {
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    background: #060e1a;
    border-bottom: 1px solid #1e293b;
  }
  .interviewer-video {
    width: 100%;
    aspect-ratio: 16 / 9;
    object-fit: cover;
    display: block;
    background: #0a1525;
  }
  .interviewer-label {
    font-size: 0.55rem;
    color: #1e3a5f;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    padding: 0.2rem 0.5rem;
    text-align: center;
  }

  .col-right-body { gap: 0.75rem; overflow-y: auto; }

  .side-stats {
    display: flex; flex-direction: column; gap: 0.1rem;
    padding: 0.5rem 0.25rem;
    border-top: 1px solid #1e293b; border-bottom: 1px solid #1e293b; flex-shrink: 0;
  }
  .side-stat {
    display: flex; align-items: center; justify-content: space-between;
    padding: 0.2rem 0.25rem; position: relative;
  }
  .side-label { font-size: 0.62rem; color: #475569; text-transform: uppercase; letter-spacing: 0.06em; font-weight: 600; }
  .side-value { font-size: 0.75rem; font-weight: 700; font-variant-numeric: tabular-nums; color: #475569; }
  .ws-dot { font-size: 0.8rem; }
  .ws-dot.connected { color: #22c55e; }
  .ws-dot.reconnecting { color: #f59e0b; }
  .filler-stat { position: relative; }
  .filler-btn { background: none; border: none; cursor: pointer; font-size: 0.75rem; font-weight: 700; color: #475569; padding: 0; }
  .filler-btn.has-fillers { color: #f59e0b; }
  .filler-popup {
    position: absolute; top: 100%; right: 0; z-index: 50;
    background: #1e293b; border: 1px solid #334155; border-radius: 0.375rem; padding: 0.5rem;
    display: flex; flex-direction: column; gap: 0.2rem; white-space: nowrap; min-width: 120px;
  }
  .filler-item { font-size: 0.72rem; color: #f59e0b; }
  .side-ratelimits { flex: 1; overflow-y: auto; min-height: 0; }

  /* Focus overlay */
  .focus-overlay {
    position: fixed; inset: 0; background: rgba(0,0,0,0.93);
    z-index: 1000; display: flex; flex-direction: column;
    align-items: center; padding: 1.5rem 2rem 2rem; cursor: pointer;
  }
  .focus-card {
    width: 100%; max-width: 680px; background: #07101e;
    border: 1px solid #1e3a5f; border-radius: 1rem; padding: 1.75rem 2rem;
    cursor: default; box-shadow: 0 0 60px rgba(59,130,246,0.08);
  }
  .focus-question {
    color: #60a5fa; font-style: italic; font-size: 0.95rem; line-height: 1.5;
    margin-bottom: 1.25rem; padding-bottom: 1rem; border-bottom: 1px solid #1e293b;
  }
  .focus-suggestion { color: #cbd5e1; line-height: 2.4; white-space: pre-wrap; font-size: 1.05rem; }
  :global(.focus-suggestion strong) { color: #fff; font-size: 1.45rem; font-weight: 800; }
  .focus-cursor { animation: blink 1s step-end infinite; color: #60a5fa; }
  @keyframes blink { 50% { opacity: 0; } }
  .focus-loading { color: #60a5fa; font-style: italic; }
  .focus-empty { color: #334155; font-style: italic; font-size: 1rem; text-align: center; padding: 3rem 0; }
  .focus-hint { margin-top: 0.75rem; font-size: 0.65rem; color: #1e293b; }
</style>
