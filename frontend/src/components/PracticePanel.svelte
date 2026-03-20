<script lang="ts">
  import { onMount } from 'svelte';
  import { countFillers, totalFillers } from '../lib/filler';
  import { parseSuggestion, parseCues } from '../lib/parseSuggestion';
  import * as ttsClient from '../lib/ttsClient';
  import type { CombinedVoice } from '../lib/ttsClient';

  const { questions, systemPrompt, onStartInterview, onBackToSetup } = $props<{
    questions: string[];
    systemPrompt: string;
    onStartInterview: () => void;
    onBackToSetup?: () => void;
  }>();

  // Font picker (mirrors app.svelte FONTS)
  const FONTS = [
    { id: 'Inter', label: 'Inter' },
    { id: 'DM Sans', label: 'DM Sans' },
    { id: 'Plus Jakarta Sans', label: 'Plus Jakarta Sans' },
    { id: 'IBM Plex Sans', label: 'IBM Plex Sans' },
    { id: 'Outfit', label: 'Outfit' },
    { id: 'system-ui', label: 'System Default' },
  ] as const;
  let appFont = $state(localStorage.getItem('app-font') ?? 'Inter');
  $effect(() => {
    const stack = appFont === 'system-ui'
      ? 'system-ui, -apple-system, sans-serif'
      : `'${appFont}', system-ui, sans-serif`;
    document.documentElement.style.setProperty('--ff-base', stack);
    localStorage.setItem('app-font', appFont);
  });

  // Voice settings
  let voices = $state<CombinedVoice[]>([]);
  let voiceId = $state(localStorage.getItem('tts-voice-id') ?? '');
  let voiceRate = $state(Math.max(0.7, Number(localStorage.getItem('tts-rate') ?? 1.5)));
  let voiceVolume = $state(Math.max(0.1, Number(localStorage.getItem('tts-volume') ?? 1.0)));

  onMount(async () => {
    const all = await ttsClient.loadAllVoices();
    voices = all;
    if (!voiceId || !all.find(v => v.id === voiceId)) voiceId = all[0]?.id ?? '';
  });

  $effect(() => { localStorage.setItem('tts-voice-id', voiceId); });
  $effect(() => { localStorage.setItem('tts-rate', String(voiceRate)); });
  $effect(() => { localStorage.setItem('tts-volume', String(voiceVolume)); });

  let readOn = $state(false);

  // Poll to reset button label after speech ends naturally
  $effect(() => {
    const id = setInterval(() => {
      if (readOn && !ttsClient.isSpeaking()) readOn = false;
    }, 250);
    return () => clearInterval(id);
  });

  function speak(text: string) {
    ttsClient.speak(text, voiceId, voiceRate, voiceVolume);
  }

  function speakSay(text: string) {
    if (readOn) {
      readOn = false;
      ttsClient.stopSpeaking();
      return;
    }
    readOn = true;
    const parsed = parseSuggestion(text);
    if (!parsed.acknowledge) { readOn = false; return; }
    ttsClient.speak(parsed.acknowledge, voiceId, voiceRate, voiceVolume);
  }


  let currentIdx = $state(0);
  let hints = $state<Record<number, string>>({});
  let loadingHint = $state(false);
  let loadingAll = $state(false);
  let answers = $state<Record<number, string>>({});
  let scores = $state<Record<number, { score: number; star_complete: boolean; has_metric: boolean; length_ok: boolean; coaching: string; strong: string }>>({});
  let scoringIdx = $state<number | null>(null);

  // Speech recording
  type VocalResult = { tone: string; pace: string; confidence_score: number; coaching: string; fillers_noted: string };
  let recording = $state(false);
  let recordingDuration = $state(0);
  let recordInterval: ReturnType<typeof setInterval> | null = null;
  let recordStart = 0;
  let recognition: any = null;
  let interimText = $state('');
  let vocalResults = $state<Record<number, VocalResult>>({});
  let loadingVocal = $state<number | null>(null);

  // Expanded cue sentences: key = `${questionIdx}-${cueIdx}`
  let expandedCues = $state<Record<string, string>>({});
  let loadingCue = $state<string | null>(null);
  let openCues = $state<Record<string, boolean>>({});
  function toggleCueOpen(key: string) { openCues = { ...openCues, [key]: !openCues[key] }; }

  async function toggleCue(qIdx: number, cueIdx: number, cue: string) {
    const key = `${qIdx}-${cueIdx}`;
    if (expandedCues[key]) { expandedCues = { ...expandedCues, [key]: '' }; return; }
    loadingCue = key;
    try {
      const resp = await fetch('/api/expand-cue', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ question: questions[qIdx], cue }),
      });
      if (resp.ok) {
        const data = await resp.json();
        expandedCues = { ...expandedCues, [key]: data.sentence };
      }
    } catch { /* ignore */ }
    loadingCue = null;
  }
  let srSupported = $state(false);

  $effect(() => {
    const SR = (window as any).SpeechRecognition || (window as any).webkitSpeechRecognition;
    srSupported = !!SR;
  });

  const currentQuestion = $derived(questions[currentIdx] ?? '');

  function startRecording() {
    const SR = (window as any).SpeechRecognition || (window as any).webkitSpeechRecognition;
    if (!SR) return;
    recognition = new SR();
    recognition.continuous = true;
    recognition.interimResults = true;
    recognition.lang = 'en-US';

    let finalText = answers[currentIdx] ?? '';
    interimText = '';

    recognition.onresult = (e: any) => {
      let interim = '';
      for (let i = e.resultIndex; i < e.results.length; i++) {
        const t = e.results[i][0].transcript;
        if (e.results[i].isFinal) {
          finalText += (finalText ? ' ' : '') + t.trim();
          answers = { ...answers, [currentIdx]: finalText };
        } else {
          interim += t;
        }
      }
      interimText = interim;
    };

    recognition.onend = () => {
      if (recording) {
        // Restarted by browser — keep going
        try { recognition.start(); } catch { stopRecording(); }
      }
    };

    recognition.onerror = () => stopRecording();

    recordStart = Date.now();
    recordingDuration = 0;
    recordInterval = setInterval(() => {
      recordingDuration = Math.floor((Date.now() - recordStart) / 1000);
    }, 500);

    recording = true;
    try { recognition.start(); } catch { recording = false; }
  }

  function stopRecording() {
    recording = false;
    interimText = '';
    if (recordInterval) { clearInterval(recordInterval); recordInterval = null; }
    if (recognition) { try { recognition.stop(); } catch {} recognition = null; }

    // Auto-assess vocal delivery
    const answer = answers[currentIdx]?.trim();
    if (answer) {
      assessVocal(currentIdx, answer, Math.max(1, recordingDuration));
    }
  }

  async function assessVocal(idx: number, answer: string, durationSec: number) {
    loadingVocal = idx;
    const words = answer.split(/\s+/).filter(Boolean);
    const fillerCounts = countFillers(answer);
    const fillerTotal = totalFillers(fillerCounts);
    const fillerDetail = fillerCounts.map(f => `${f.word} ×${f.count}`).join(', ');
    try {
      const resp = await fetch('/api/vocal-sentiment', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          question: questions[idx],
          transcript: answer,
          duration_seconds: durationSec,
          word_count: words.length,
          filler_count: fillerTotal,
          filler_detail: fillerDetail,
        }),
      });
      if (resp.ok) vocalResults = { ...vocalResults, [idx]: await resp.json() };
    } catch { /* ignore */ }
    loadingVocal = null;
  }

  async function getHint(idx = currentIdx) {
    if (hints[idx] || loadingHint) return;
    loadingHint = true;
    try {
      const resp = await fetch('/api/practice-question', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ question: questions[idx] }),
      });
      if (resp.ok) {
        const data = await resp.json();
        hints = { ...hints, [idx]: data.suggestion };
      }
    } catch { /* ignore */ }
    loadingHint = false;
  }

  // Auto-load hint for the current question when it changes
  $effect(() => {
    const idx = currentIdx;
    if (questions.length > 0 && !hints[idx]) getHint(idx);
  });

  async function scoreAnswer(idx: number) {
    const answer = answers[idx];
    if (!answer?.trim() || scoringIdx === idx) return;
    scoringIdx = idx;
    try {
      const resp = await fetch('/api/score-practice', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ question: questions[idx], answer }),
      });
      if (resp.ok) {
        const data = await resp.json();
        scores = { ...scores, [idx]: data };
      }
    } catch { /* ignore */ }
    scoringIdx = null;
  }

  async function getAllHints() {
    if (loadingAll) return;
    loadingAll = true;
    for (let i = 0; i < questions.length; i++) {
      if (!hints[i]) {
        try {
          const resp = await fetch('/api/practice-question', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ question: questions[i] }),
          });
          if (resp.ok) {
            const data = await resp.json();
            hints = { ...hints, [i]: data.suggestion };
          }
        } catch { /* continue */ }
      }
    }
    loadingAll = false;
  }

  function fmtDur(s: number): string {
    return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
  }

  const toneColor: Record<string, string> = {
    confident: '#22c55e', enthusiastic: '#22c55e',
    hesitant: '#f59e0b', flat: '#f59e0b',
    nervous: '#ef4444', neutral: '#64748b',
  };
</script>

<div class="practice">
  <div class="practice-header">
    <div>
      <h2>Practice Mode</h2>
      <p class="subtitle">Review predicted questions and prepare your answers before the interview</p>
    </div>
    {#if questions.length > 0}
      <button class="prep-all-btn" onclick={getAllHints} disabled={loadingAll}>
        {loadingAll ? '⟳ Loading all...' : '⚡ Prep all questions'}
      </button>
    {/if}
  </div>

  <!-- Voice controls -->
  <div class="voice-bar">
    <span class="voice-bar-label">Voice</span>
    <select class="voice-select" bind:value={voiceId}>
      {#if voices.filter(v => v.source === 'piper').length > 0}
        <optgroup label="Piper (Neural)">
          {#each voices.filter(v => v.source === 'piper') as v}
            <option value={v.id}>{v.name}</option>
          {/each}
        </optgroup>
      {/if}
      {#if voices.filter(v => v.source === 'os').length > 0}
        <optgroup label="Windows (SAPI)">
          {#each voices.filter(v => v.source === 'os') as v}
            <option value={v.id}>{v.name}</option>
          {/each}
        </optgroup>
      {/if}
      {#if voices.filter(v => v.source === 'browser').length > 0}
        <optgroup label="Browser">
          {#each voices.filter(v => v.source === 'browser') as v}
            <option value={v.id}>{v.name}</option>
          {/each}
        </optgroup>
      {/if}
    </select>
    <label class="voice-slider-label" title="Speed">
      <span class="voice-val">{voiceRate.toFixed(1)}×</span>
      <input type="range" min="0.7" max="4.0" step="0.1" bind:value={voiceRate} class="voice-slider"
        oninput={(e) => { const v = Number((e.target as HTMLInputElement).value); if (Math.abs(v - 1) < 0.08) voiceRate = 1; }} />
    </label>
    <label class="voice-slider-label" title="Volume">
      <span class="voice-val">{Math.round(voiceVolume * 100)}%</span>
      <input type="range" min="0.1" max="4" step="0.05" bind:value={voiceVolume} class="voice-slider"
        oninput={(e) => {
          const v = Number((e.target as HTMLInputElement).value);
          if (Math.abs(v - 1) < 0.08) { voiceVolume = 1; return; }
          if (v < 1) { const snapped = Math.round(v / 0.05) * 0.05; if (Math.abs(v - snapped) < 0.026) voiceVolume = snapped; }
        }} />
    </label>
    <button class="voice-test-btn" onclick={() => speak("Hi, I'm excited to be here today.")}>▶ Test</button>
    <select class="font-select" bind:value={appFont} title="App font">
      {#each FONTS as f}
        <option value={f.id}>{f.label}</option>
      {/each}
    </select>
  </div>

  {#if questions.length === 0}
    <div class="empty">
      <p>No predicted questions were generated — this can happen if the AI quota was hit during setup, or if no job description / CV was provided.</p>
      <p>Go back to Setup, add your job description and CV, then re-submit to generate questions. Or click <strong>I'm Ready — Start Interview →</strong> to skip practice.</p>
    </div>
  {:else}
    <div class="progress">Question {currentIdx + 1} of {questions.length}</div>

    <div class="question-card">
      <p class="question-text">{currentQuestion}</p>
      {#if loadingHint && !hints[currentIdx]}
        <div class="hint-loading-inline">Loading suggestions…</div>
      {:else if hints[currentIdx]}
        {@const parsed = parseSuggestion(hints[currentIdx])}
        <div class="hints-card">
          <div class="h-read-row">
            <button class="read-btn" class:active={readOn} onclick={() => speakSay(hints[currentIdx])}>{readOn ? '⏹ Stop' : '🔊 Read'}</button>
          </div>

          {#if parsed.acknowledge}
            <div class="h-sec h-sec-ack">
              <span class="h-cue-badge h-cue-ack">Acknowledge</span>
              <span class="h-speak-text">{parsed.acknowledge}</span>
            </div>
          {/if}

          <div class="h-sec h-sec-say">
            <span class="h-cue-badge">{parsed.cue}</span>
            <span class="h-speak-text h-speak-main">{parsed.tell}</span>
            {#if parsed.body}
              {@const cues = parseCues(parsed.body)}
              {#if cues.length > 0}
                <div class="h-cues-section">
                  {#each cues as c, ci}
                    {@const isOpen = !!openCues[`${currentIdx}-${ci}`]}
                    <div class="h-cue-block" class:h-cue-open={isOpen}>
                      <button class="h-cue-toggle" onclick={() => { const opening = !isOpen; toggleCueOpen(`${currentIdx}-${ci}`); if (opening) toggleCue(currentIdx, ci, c.text); }}>
                        <span class="h-cue-label" class:h-cue-label-example={c.typeTag === 'Example' || c.typeTag === 'Story' || c.label === 'Example' || c.label === 'Story'}>{c.typeTag || (c.label === 'General' ? 'Point' : c.label)}</span>
                        <span class="h-cue-preview">{c.text}</span>
                        <span class="h-cue-chevron">{isOpen ? '▾' : '▸'}</span>
                      </button>
                      {#if isOpen}
                        <div class="h-cue-body">
                          {#if loadingCue === `${currentIdx}-${ci}`}
                            <div class="h-cue-sentence h-cue-loading">…</div>
                          {:else if expandedCues[`${currentIdx}-${ci}`]}
                            <div class="h-cue-sentence">{#each expandedCues[`${currentIdx}-${ci}`].split(/(?<=[.!?])\s+/) as s}{s.trim()}<br/>{/each}</div>
                          {:else}
                            <div class="h-cue-sentence h-cue-loading">Loading…</div>
                          {/if}
                        </div>
                      {/if}
                    </div>
                  {/each}
                </div>
              {/if}
            {/if}
          </div>

          {#if parsed.asks.length > 0}
            <div class="h-sec h-sec-ask">
              <span class="h-cue-badge h-cue-ask">Ask</span>
              <div class="h-cues-section">
                {#each parsed.asks as ask, ai}
                  {@const askKey = `ask-${currentIdx}-${ai}`}
                  {@const askOpen = !!openCues[askKey]}
                  <div class="h-cue-block h-cue-block-ask" class:h-cue-open={askOpen}>
                    <button class="h-cue-toggle" onclick={() => toggleCueOpen(askKey)}>
                      <span class="h-cue-label h-cue-label-ask">Q{ai + 1}</span>
                      <span class="h-cue-preview h-ask-preview">{ask.topic}</span>
                      <span class="h-cue-chevron">{askOpen ? '▾' : '▸'}</span>
                    </button>
                    {#if askOpen}
                      <div class="h-cue-body">
                        <div class="h-ask-sentence">{ask.question}</div>
                      </div>
                    {/if}
                  </div>
                {/each}
              </div>
            </div>
          {/if}
        </div>
      {/if}

      <div class="answer-section">
        <!-- Recording controls -->
        {#if srSupported}
          <div class="record-row">
            {#if !recording}
              <button class="record-btn" onclick={startRecording}>
                🎤 Speak Answer
              </button>
            {:else}
              <button class="record-stop-btn" onclick={stopRecording}>
                ⏹ Stop  {fmtDur(recordingDuration)}
              </button>
              <span class="recording-dot"></span>
              <span class="recording-label">Recording…</span>
            {/if}
          </div>
          {#if interimText}
            <div class="interim">{interimText}</div>
          {/if}
        {/if}

        <label class="answer-label" for="answer-{currentIdx}">Your practice answer</label>
        <textarea
          id="answer-{currentIdx}"
          class="answer-input"
          rows={4}
          bind:value={answers[currentIdx]}
          placeholder={srSupported ? 'Speak your answer above, or type here...' : 'Type your answer here to get it scored...'}
        ></textarea>

        <div class="score-row">
          <button
            class="score-btn"
            onclick={() => scoreAnswer(currentIdx)}
            disabled={!answers[currentIdx]?.trim() || scoringIdx === currentIdx}
          >
            {scoringIdx === currentIdx ? 'Scoring...' : '⭐ Score answer'}
          </button>
        </div>

        <!-- Content score card -->
        {#if scores[currentIdx]}
          {@const sc = scores[currentIdx]}
          <div class="score-card">
            <div class="score-top">
              <span class="score-num" style="color: {sc.score >= 70 ? '#22c55e' : sc.score >= 50 ? '#f59e0b' : '#ef4444'}">{sc.score}/100</span>
              <span class="score-sublabel">Content</span>
              <div class="score-badges">
                <span class="badge" class:badge-on={sc.star_complete}>STAR</span>
                <span class="badge" class:badge-on={sc.has_metric}>Metric</span>
                <span class="badge" class:badge-on={sc.length_ok}>Length</span>
              </div>
            </div>
            {#if sc.strong}<p class="score-strong">✓ {sc.strong}</p>{/if}
            {#if sc.coaching}<p class="score-coach">{sc.coaching}</p>{/if}
          </div>
        {/if}

        <!-- Vocal sentiment card -->
        {#if loadingVocal === currentIdx}
          <div class="vocal-card vocal-loading">Assessing vocal delivery…</div>
        {:else if vocalResults[currentIdx]}
          {@const v = vocalResults[currentIdx]}
          <div class="vocal-card">
            <div class="vocal-top">
              <span class="vocal-score" style="color: {v.confidence_score >= 70 ? '#22c55e' : v.confidence_score >= 50 ? '#f59e0b' : '#ef4444'}">{v.confidence_score}/100</span>
              <span class="score-sublabel">Delivery</span>
              <span class="tone-badge" style="background: {(toneColor[v.tone] ?? '#64748b')}22; color: {toneColor[v.tone] ?? '#64748b'}; border-color: {toneColor[v.tone] ?? '#334155'}">{v.tone}</span>
              <span class="pace-label">{v.pace}</span>
            </div>
            {#if v.fillers_noted}
              <p class="vocal-fillers">⚠ {v.fillers_noted}</p>
            {/if}
            {#if v.coaching}
              <p class="vocal-coaching">{v.coaching}</p>
            {/if}
          </div>
        {/if}
      </div>
    </div>

    <div class="nav">
      <button class="nav-btn" disabled={currentIdx === 0} onclick={() => currentIdx--}>← Previous</button>
      <div class="dots">
        {#each questions as _, i}
          <button class="dot" class:active={i === currentIdx} class:hinted={!!hints[i]}
            onclick={() => currentIdx = i} title={hints[i] ? 'Hints loaded' : `Question ${i + 1}`}></button>
        {/each}
      </div>
      <button class="nav-btn" disabled={currentIdx === questions.length - 1} onclick={() => currentIdx++}>Next →</button>
    </div>

    {@const scoredCount = Object.keys(scores).length}
    {#if scoredCount >= 1}
      {@const avgContent = Math.round(Object.values(scores).reduce((s, v) => s + v.score, 0) / scoredCount)}
      {@const avgDelivery = Object.keys(vocalResults).length > 0
        ? Math.round(Object.values(vocalResults).reduce((s, v) => s + v.confidence_score, 0) / Object.keys(vocalResults).length)
        : null}
      {@const starCount = Object.values(scores).filter(s => s.star_complete).length}
      {@const metricCount = Object.values(scores).filter(s => s.has_metric).length}
      <div class="summary-card">
        <div class="summary-header">Session Summary — {scoredCount}/{questions.length} scored</div>
        <div class="summary-scores">
          <div class="summary-stat">
            <span class="summary-val" style="color: {avgContent >= 70 ? '#22c55e' : avgContent >= 50 ? '#f59e0b' : '#ef4444'}">{avgContent}</span>
            <span class="summary-lbl">Avg Content</span>
          </div>
          {#if avgDelivery != null}
            <div class="summary-stat">
              <span class="summary-val" style="color: {avgDelivery >= 70 ? '#22c55e' : avgDelivery >= 50 ? '#f59e0b' : '#ef4444'}">{avgDelivery}</span>
              <span class="summary-lbl">Avg Delivery</span>
            </div>
          {/if}
          <div class="summary-stat">
            <span class="summary-val" style="color: #4ade80">{starCount}/{scoredCount}</span>
            <span class="summary-lbl">STAR complete</span>
          </div>
          <div class="summary-stat">
            <span class="summary-val" style="color: #60a5fa">{metricCount}/{scoredCount}</span>
            <span class="summary-lbl">Had metric</span>
          </div>
        </div>
        <div class="summary-rows">
          {#each questions as q, i}
            {#if scores[i]}
              <button class="summary-row" onclick={() => currentIdx = i}>
                <span class="sum-q">Q{i + 1}</span>
                <span class="sum-score" style="color: {scores[i].score >= 70 ? '#22c55e' : scores[i].score >= 50 ? '#f59e0b' : '#ef4444'}">{scores[i].score}</span>
                {#if vocalResults[i]}<span class="sum-del" style="color: {vocalResults[i].confidence_score >= 70 ? '#22c55e' : '#f59e0b'}">{vocalResults[i].confidence_score}d</span>{/if}
                <span class="sum-badges">
                  {#if scores[i].star_complete}<span class="sum-badge sum-on">STAR</span>{/if}
                  {#if scores[i].has_metric}<span class="sum-badge sum-on">M</span>{/if}
                </span>
                <span class="sum-q-text">{q.length > 55 ? q.slice(0, 55) + '…' : q}</span>
              </button>
            {/if}
          {/each}
        </div>
      </div>
    {/if}
  {/if}

  <!-- Bottom navigation bar — consistent with setup overview -->
  <div class="practice-action-row">
    {#if onBackToSetup}
      <button class="btn-back" onclick={onBackToSetup}>← Overview</button>
    {/if}
    <div class="practice-action-right">
      <button class="start-btn" onclick={onStartInterview}>Start Interview →</button>
    </div>
  </div>
</div>

<style>
  .practice {
    max-width: 720px; margin: 0 auto; padding: 2rem;
    display: flex; flex-direction: column; gap: 1.5rem;
  }
  .practice-header {
    display: flex; align-items: flex-start; justify-content: space-between; gap: 1rem; flex-wrap: wrap;
  }
  .practice-action-row { display: flex; align-items: center; justify-content: space-between; gap: 1rem; padding: 1.5rem 0 0.5rem; border-top: 1px solid #1e293b; margin-top: 1rem; }
  .practice-action-right { display: flex; align-items: center; gap: 0.75rem; margin-left: auto; }
  h2 { font-size: 1.5rem; font-weight: 800; color: #f1f5f9; margin: 0 0 0.25rem; }
  .subtitle { color: #64748b; font-size: 0.875rem; margin: 0; }
  .start-btn {
    padding: 0.6rem 1.5rem; background: #3b82f6; color: white;
    border: none; border-radius: 0.5rem; font-size: var(--fs-base); font-weight: 600;
    cursor: pointer; white-space: nowrap; transition: background 0.2s;
  }
  .start-btn:hover { background: #2563eb; }
  .prep-all-btn {
    padding: 0.35rem 0.9rem; background: transparent; border: 1px solid #334155;
    border-radius: 0.375rem; color: #64748b; font-size: var(--fs-base); cursor: pointer;
    transition: all 0.15s; white-space: nowrap;
  }
  .prep-all-btn:hover:not(:disabled) { border-color: #60a5fa; color: #60a5fa; }
  .prep-all-btn:disabled { opacity: 0.5; cursor: default; }
  .progress { font-size: var(--fs-sm); color: #60a5fa; text-align: center; font-weight: 700; letter-spacing: 0.04em; }
  .question-card {
    background: #0f172a; border: 1px solid #334155; border-left: 3px solid #3b82f6;
    border-radius: 0.75rem; padding: 1.5rem; display: flex; flex-direction: column; gap: 1rem;
  }
  .question-text { font-size: 0.9rem; font-weight: 500; color: #f87171; line-height: 1.5; margin: 0; font-style: normal; }
  .hint-loading-inline { color: #475569; font-size: var(--fs-sm); font-style: italic; }
  .hints-card {
    background: #07101e; border: 1px solid #1a2d4a; border-radius: 0.6rem;
    padding: 0.75rem; display: flex; flex-direction: column; gap: 0.4rem;
  }
  /* Section blocks matching interview mode */
  .h-sec {
    display: flex; flex-direction: column; gap: 0.35rem;
    padding: 0.45rem 0.6rem; border-radius: 0.4rem;
    border-left: 3px solid transparent;
  }
  .h-sec-ack    { background: #110823; border-left-color: #6d28d9; }
  .h-sec-say    { background: #060e0a; border-left-color: #166534; }
  .h-sec-ask    { background: #0e0700; border-left-color: #92400e; }
  .font-select {
    padding: 0.2rem 0.4rem; background: #0a0f1a; border: 1px solid #1e293b;
    border-radius: 0.3rem; color: #64748b; font-size: var(--fs-sm); cursor: pointer;
    font-family: inherit; margin-left: auto;
  }
  .font-select:hover { border-color: #334155; color: #94a3b8; }

  /* Cue badges */
  .h-cue-badge {
    display: inline-block; padding: 0.1rem 0.45rem; min-width: 6.2rem; text-align: center;
    background: #14532d; color: #4ade80; border-radius: 0.25rem;
    font-size: var(--fs-xs); font-weight: 800; text-transform: uppercase;
    letter-spacing: 0.06em; flex-shrink: 0; margin-top: 0.1rem;
  }
  .h-cue-badge.h-cue-ask    { background: #422006; color: #fbbf24; }
  .h-cue-badge.h-cue-ack    { background: #2e1065; color: #c084fc; }

  /* Spoken text */
  .h-speak-text { color: #e2e8f0; font-size: var(--fs-lg); line-height: 1.5; flex: 1; }
  .h-speak-main { font-weight: 600; color: #f1f5f9; }


  /* Cue blocks — matching interview teleprompter */
  .h-cues-section { display: flex; flex-direction: column; gap: 0.2rem; border-top: 1px solid #0d2010; padding-top: 0.35rem; }
  .h-cue-block { border-radius: 0.3rem; border: 1px solid #0d2010; overflow: hidden; background: #040b06; }
  .h-cue-block.h-cue-open { border-color: #14532d; }
  .h-cue-block-ask { border-color: #2d1200; background: #060300; }
  .h-cue-block-ask.h-cue-open { border-color: #78350f; }
  .h-cue-toggle {
    display: flex; align-items: center; gap: 0.4rem;
    width: 100%; padding: 0.28rem 0.5rem;
    background: none; border: none; cursor: pointer; text-align: left;
  }
  .h-cue-toggle:hover { background: #071a0f; }
  .h-cue-label {
    font-size: var(--fs-xs); font-weight: 800; text-transform: uppercase;
    letter-spacing: 0.07em; color: #4ade80; flex-shrink: 0;
  }
  .h-cue-label-ask { color: #fbbf24 !important; }
  .h-cue-preview { flex: 1; font-size: var(--fs-base); color: #3d8c52; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .h-ask-preview { color: #7c4a1a !important; white-space: normal !important; overflow: visible !important; text-overflow: unset !important; }
  .h-cue-chevron { font-size: var(--fs-sm); color: #2d6e40; flex-shrink: 0; }
  .h-cue-body { display: flex; flex-direction: column; padding: 0.1rem 0.5rem 0.35rem; border-top: 1px solid #0d2010; }
  .h-cue-sentence {
    padding: 0.3rem 0.4rem; background: #061209; border-left: 2px solid #22c55e;
    border-radius: 0 0.25rem 0.25rem 0; color: #f1f5f9; font-size: var(--fs-lg); line-height: 1.5; font-weight: 400;
  }
  .h-cue-label-example { background: #1a3a1a !important; color: #86efac !important; }
  .h-cue-loading { color: #334155; }
  .h-ask-sentence {
    padding: 0.3rem 0.4rem; background: #060300; border-left: 2px solid #92400e;
    border-radius: 0 0.25rem 0.25rem 0; color: #f1f5f9; font-size: var(--fs-lg); line-height: 1.5; font-weight: 400; overflow-wrap: break-word;
  }
  .answer-section { display: flex; flex-direction: column; gap: 0.6rem; margin-top: 0.5rem; }
  .record-row { display: flex; align-items: center; gap: 0.6rem; }
  .record-btn {
    padding: 0.4rem 1rem; background: transparent; border: 1px solid #6d28d9;
    border-radius: 0.375rem; color: #a78bfa; font-size: var(--fs-base); cursor: pointer; transition: all 0.15s;
  }
  .record-btn:hover { background: #3b0764; }
  .record-stop-btn {
    padding: 0.4rem 1rem; background: #3b0764; border: 1px solid #7c3aed;
    border-radius: 0.375rem; color: #c4b5fd; font-size: var(--fs-base); cursor: pointer;
    font-variant-numeric: tabular-nums;
  }
  .recording-dot {
    width: 8px; height: 8px; border-radius: 50%; background: #ef4444;
    animation: blink 1s infinite;
  }
  @keyframes blink { 0%, 100% { opacity: 1; } 50% { opacity: 0.2; } }
  .recording-label { font-size: var(--fs-sm); color: #ef4444; }
  .interim {
    font-size: var(--fs-base); color: #475569; font-style: italic;
    padding: 0.3rem 0.5rem; border-left: 2px solid #334155;
  }
  .answer-label { font-size: var(--fs-sm); color: #64748b; }
  .answer-input {
    width: 100%; padding: 0.6rem 0.75rem;
    background: #1e293b; border: 1px solid #334155; border-radius: 0.375rem;
    color: #e2e8f0; font-size: 0.875rem; resize: vertical; font-family: inherit;
  }
  .answer-input:focus { outline: none; border-color: #3b82f6; }
  .score-row { display: flex; gap: 0.5rem; }
  .score-btn {
    padding: 0.4rem 1rem; background: transparent; border: 1px solid #7c3aed;
    border-radius: 0.375rem; color: #a78bfa; font-size: var(--fs-base); cursor: pointer; transition: all 0.15s;
  }
  .score-btn:hover:not(:disabled) { background: #3b0764; }
  .score-btn:disabled { opacity: 0.5; cursor: default; }
  .score-card, .vocal-card {
    background: #0a0a1a; border: 1px solid #1e293b; border-radius: 0.5rem;
    padding: 0.75rem; display: flex; flex-direction: column; gap: 0.4rem;
  }
  .vocal-loading { color: #475569; font-size: var(--fs-base); font-style: italic; }
  .score-top, .vocal-top { display: flex; align-items: center; gap: 0.75rem; flex-wrap: wrap; }
  .score-num, .vocal-score { font-size: 1.5rem; font-weight: 800; font-variant-numeric: tabular-nums; }
  .score-sublabel { font-size: var(--fs-sm); font-weight: 700; text-transform: uppercase; letter-spacing: 0.07em; color: #475569; }
  .score-badges { display: flex; gap: 0.3rem; }
  .badge {
    padding: 0.1rem 0.4rem; border-radius: 0.2rem; font-size: var(--fs-xs);
    font-weight: 800; text-transform: uppercase; letter-spacing: 0.05em;
    background: #1e293b; color: #334155; border: 1px solid #1e293b;
  }
  .badge.badge-on { background: #14532d; color: #4ade80; border-color: #14532d; }
  .score-strong { margin: 0; font-size: var(--fs-base); color: #4ade80; }
  .score-coach, .vocal-coaching { margin: 0; font-size: var(--fs-base); color: #94a3b8; line-height: 1.5; font-style: italic; }
  .tone-badge {
    padding: 0.1rem 0.5rem; border-radius: 0.25rem; font-size: var(--fs-sm);
    font-weight: 700; text-transform: capitalize; border: 1px solid;
  }
  .pace-label { font-size: var(--fs-sm); color: #64748b; }
  .vocal-fillers { margin: 0; font-size: var(--fs-sm); color: #f59e0b; }
  .nav { display: flex; align-items: center; justify-content: space-between; }
  .nav-btn {
    padding: 0.4rem 1rem; background: transparent;
    border: 1px solid #334155; border-radius: 0.375rem;
    color: #94a3b8; font-size: var(--fs-base); cursor: pointer; transition: all 0.15s;
  }
  .nav-btn:hover:not(:disabled) { border-color: #60a5fa; color: #60a5fa; }
  .nav-btn:disabled { opacity: 0.3; cursor: default; }
  .dots { display: flex; gap: 0.4rem; flex-wrap: wrap; justify-content: center; }
  .dot {
    width: 8px; height: 8px; border-radius: 50%;
    background: #1e293b; border: none; cursor: pointer; transition: all 0.15s;
  }
  .dot.active {
    background: #3b82f6; transform: scale(1.6);
    box-shadow: 0 0 0 2px #1d4ed8;
  }
  .dot.hinted { background: #22c55e; }
  .dot.active.hinted { background: #3b82f6; }
  .summary-card {
    background: #0a0f1a; border: 1px solid #1e293b; border-left: 3px solid #22c55e;
    border-radius: 0.6rem; padding: 1rem; display: flex; flex-direction: column; gap: 0.75rem;
  }
  .summary-header { font-size: var(--fs-sm); font-weight: 700; text-transform: uppercase; letter-spacing: 0.07em; color: #4ade80; }
  .summary-scores { display: flex; gap: 1.5rem; flex-wrap: wrap; }
  .summary-stat { display: flex; flex-direction: column; align-items: center; gap: 0.1rem; }
  .summary-val { font-size: 1.6rem; font-weight: 800; font-variant-numeric: tabular-nums; }
  .summary-lbl { font-size: var(--fs-xs); color: #475569; text-transform: uppercase; letter-spacing: 0.06em; }
  .summary-rows { display: flex; flex-direction: column; gap: 0.3rem; }
  .summary-row {
    display: flex; align-items: center; gap: 0.5rem;
    padding: 0.3rem 0.5rem; background: #0d1525; border: 1px solid #1e2d45;
    border-radius: 0.3rem; cursor: pointer; text-align: left; width: 100%;
    transition: background 0.15s;
  }
  .summary-row:hover { background: #111e33; }
  .sum-q { font-size: var(--fs-xs); font-weight: 800; color: #475569; flex-shrink: 0; min-width: 1.5rem; }
  .sum-score { font-size: var(--fs-sm); font-weight: 800; flex-shrink: 0; min-width: 1.8rem; }
  .sum-del { font-size: var(--fs-sm); flex-shrink: 0; min-width: 1.8rem; }
  .sum-badges { display: flex; gap: 0.2rem; flex-shrink: 0; }
  .sum-badge { font-size: 0.5rem; padding: 0.05rem 0.3rem; border-radius: 0.15rem; background: #1e293b; color: #334155; }
  .sum-badge.sum-on { background: #14532d; color: #4ade80; }
  .sum-q-text { font-size: var(--fs-sm); color: #64748b; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; flex: 1; }

  .empty { color: #475569; font-style: italic; text-align: center; padding: 3rem; }
  .voice-bar {
    display: flex; align-items: center; gap: 0.6rem; flex-wrap: wrap;
    padding: 0.5rem 0.75rem; background: #0a0f1a; border: 1px solid #1e293b;
    border-radius: 0.5rem;
  }
  .voice-bar-label { font-size: var(--fs-sm); font-weight: 700; text-transform: uppercase; letter-spacing: 0.08em; color: #334155; white-space: nowrap; }
  .voice-select {
    flex: 1; min-width: 140px; padding: 0.25rem 0.4rem;
    background: #1e293b; border: 1px solid #334155; border-radius: 0.3rem;
    color: #e2e8f0; font-size: var(--fs-sm);
  }
  .voice-slider-label { display: flex; align-items: center; gap: 0.3rem; font-size: var(--fs-sm); color: #475569; white-space: nowrap; }
  .voice-val { min-width: 2.5rem; text-align: right; color: #94a3b8; font-variant-numeric: tabular-nums; font-size: var(--fs-sm); }
  .voice-slider { width: 70px; accent-color: #3b82f6; }
  .voice-test-btn {
    padding: 0.25rem 0.6rem; background: transparent; border: 1px solid #334155;
    border-radius: 0.3rem; color: #60a5fa; font-size: var(--fs-sm); cursor: pointer;
    transition: all 0.15s; white-space: nowrap;
  }
  .voice-test-btn:hover { background: #1e3a5f; border-color: #3b82f6; }
  .h-read-row { display: flex; justify-content: flex-end; padding-bottom: 0.25rem; border-bottom: 1px solid #0f1e33; margin-bottom: 0.1rem; }
  .read-btn {
    padding: 0.15rem 0.5rem; background: transparent; border: 1px solid #334155;
    border-radius: 0.25rem; color: #60a5fa; font-size: var(--fs-sm); cursor: pointer;
    transition: all 0.15s;
  }
  .read-btn:hover { background: #1e3a5f; border-color: #3b82f6; }
  .read-btn.active { background: #1e3a5f; border-color: #ef4444; color: #ef4444; }
</style>
