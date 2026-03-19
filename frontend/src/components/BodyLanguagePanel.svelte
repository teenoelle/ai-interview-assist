<script lang="ts">
  const {
    emotion,
    liveEmotion = '',
    coaching,
    coachingWhy,
    consecutiveCount = 1,
    fillerTotal = 0,
    speakerMode = 'idle',
    triggerCounts = {},
    presenceIssues = [],
    presencePositive = null,
  } = $props<{
    emotion: string;
    liveEmotion?: string;  // fast client-side face detection, drives checklist highlights
    coaching?: string;
    coachingWhy?: string;
    consecutiveCount?: number;
    fillerTotal?: number;
    speakerMode?: 'listening' | 'answering' | 'idle';
    triggerCounts?: Record<string, number>;
    presenceIssues?: string[];
    presencePositive?: string | null;
  }>();

  // Use liveEmotion for highlights when available, fall back to backend emotion
  const highlightEmotion = $derived(liveEmotion || emotion);

  const items = [
    {
      id: 'eye',
      icon: '👁️',
      label: 'Eye contact with camera',
      why: 'Looking at the camera lens (not the screen) simulates direct eye contact — it signals confidence and holds the interviewer\'s attention.',
      trigger: ['skeptical', 'bored', 'confused'],
      listenMode: true,
      answerMode: false,
      listeningTip: 'Hold camera contact while they speak — it signals you\'re fully present.',
      answeringTip: 'Return to the camera lens as you land your main point.',
    },
    {
      id: 'posture',
      icon: '🪑',
      label: 'Upright, open posture',
      why: 'Sitting straight with shoulders back projects authority and energy. Slouching reads as disinterest or low confidence.',
      trigger: ['bored', 'neutral', 'skeptical'],
      listenMode: false,
      answerMode: true,
      listeningTip: 'Sit forward slightly — it signals active engagement.',
      answeringTip: 'Shoulders back, sit tall — posture projects confidence while you speak.',
    },
    {
      id: 'nod',
      icon: '🙂',
      label: 'Nod and react visibly',
      why: 'Visible reactions (nodding, small smiles) signal active listening and make you appear engaged even when the interviewer is speaking.',
      trigger: ['neutral', 'bored'],
      listenMode: true,
      answerMode: false,
      listeningTip: 'Nod slowly as they make each point — it encourages them to continue.',
      answeringTip: 'Smile and nod when they acknowledge your answer.',
    },
    {
      id: 'hands',
      icon: '🤲',
      label: 'Calm, open hand gestures',
      why: 'Using hands expressively (visible to camera, below chin) conveys enthusiasm and openness. Hiding or fidgeting hands signals anxiety.',
      trigger: ['skeptical', 'confused'],
      listenMode: false,
      answerMode: true,
      listeningTip: 'Keep hands visible and still — hidden hands read as defensive.',
      answeringTip: 'Use open gestures below chin level to emphasise key points.',
    },
    {
      id: 'expression',
      icon: '😊',
      label: 'Warm, natural expression',
      why: 'A genuine, relaxed expression builds rapport. An overly serious or tense face can make the interviewer feel they\'re interrogating rather than conversing.',
      trigger: ['bored', 'neutral', 'skeptical'],
      listenMode: true,
      answerMode: false,
      listeningTip: 'Soft smile and relaxed jaw — avoid a fixed neutral stare.',
      answeringTip: 'Let your expression match your content — enthusiasm shows in your face.',
    },
    {
      id: 'pace',
      icon: '⏱️',
      label: 'Pause before answering',
      why: 'A deliberate 1–2 second pause before replying shows you\'re thoughtful, not rehearsed. It also slows your pace, reducing filler words.',
      trigger: ['curious', 'engaged', 'skeptical'],
      listenMode: false,
      answerMode: true,
      listeningTip: 'Take a breath before you start — silence reads as confidence.',
      answeringTip: 'Slow down. One pause is worth ten filler words.',
    },
  ];

  let expandedId = $state<string | null>(null);
  let standingOpen = $state(false);

  function isEmotionHighlighted(item: typeof items[0]): boolean {
    return item.trigger.includes(highlightEmotion);
  }

  function isModeHighlighted(item: typeof items[0]): boolean {
    if (speakerMode === 'listening') return item.listenMode;
    if (speakerMode === 'answering') return item.answerMode;
    return false;
  }

  function isFillerHighlighted(item: typeof items[0]): boolean {
    return item.id === 'pace' && fillerTotal >= 3;
  }

  function isHighlighted(item: typeof items[0]): boolean {
    return isEmotionHighlighted(item) || isModeHighlighted(item) || isFillerHighlighted(item);
  }

  function getModeHint(item: typeof items[0]): string | null {
    if (speakerMode === 'listening') return item.listeningTip;
    if (speakerMode === 'answering') return item.answeringTip;
    return null;
  }

  const urgent = $derived(consecutiveCount >= 2 && highlightEmotion !== 'neutral' && highlightEmotion !== '');

  const highlighted = $derived(items.filter(i => isHighlighted(i)));
  const standing = $derived(items.filter(i => !isHighlighted(i)));

  function toggle(id: string) {
    expandedId = expandedId === id ? null : id;
  }
</script>

<div class="bl-panel">
  <!-- Mode badge -->
  {#if speakerMode !== 'idle'}
    <div class="mode-badge" class:mode-listening={speakerMode === 'listening'} class:mode-answering={speakerMode === 'answering'}>
      {speakerMode === 'listening' ? '👂 Listening mode' : '🎤 Answering mode'}
    </div>
  {/if}

  <!-- AI coaching tip -->
  {#if coaching}
    <div class="ai-tip" class:ai-tip-urgent={urgent}>
      <div class="ai-tip-header">
        <span class="ai-tip-icon">💡</span>
        <span class="ai-tip-label" class:urgent-label={urgent}>
          {urgent ? `⚠ Still ${emotion} (${consecutiveCount} reads)` : 'Right now'}
        </span>
      </div>
      <div class="ai-tip-text">{coaching}</div>
      {#if coachingWhy}
        <div class="ai-tip-why">{coachingWhy}</div>
      {/if}
    </div>
  {:else}
    <div class="ai-tip ai-tip-empty">
      <span class="ai-tip-icon">💡</span>
      <span class="ai-tip-waiting">Coaching tips will appear after first sentiment read</span>
    </div>
  {/if}

  <!-- Highlighted items (prioritized) -->
  {#if highlighted.length > 0}
    <div class="checklist">
      {#each highlighted as item (item.id)}
        {@const modeHint = getModeHint(item)}
        {@const count = triggerCounts[item.id] ?? 0}
        <div class="check-item highlighted" class:check-urgent={urgent && isEmotionHighlighted(item)} onclick={() => toggle(item.id)}>
          <div class="check-row">
            <span class="check-icon">{item.icon}</span>
            <span class="check-label highlighted-label">{item.label}</span>
            {#if count > 1}<span class="trigger-count">×{count}</span>{/if}
            <span class="check-dot"></span>
            <span class="check-chevron">{expandedId === item.id ? '▴' : '▾'}</span>
          </div>
          {#if modeHint}
            <div class="mode-hint">{modeHint}</div>
          {/if}
          {#if expandedId === item.id}
            <div class="check-why">{item.why}</div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}

  <!-- Standing checks (non-highlighted, collapsible) -->
  {#if standing.length > 0}
    <button class="standing-toggle" onclick={() => standingOpen = !standingOpen}>
      <span class="standing-label">Standing checks</span>
      <span class="standing-chevron">{standingOpen ? '▴' : '▾'}</span>
    </button>
    {#if standingOpen}
      <div class="checklist checklist-standing">
        {#each standing as item (item.id)}
          {@const count = triggerCounts[item.id] ?? 0}
          <div class="check-item" onclick={() => toggle(item.id)}>
            <div class="check-row">
              <span class="check-icon">{item.icon}</span>
              <span class="check-label">{item.label}</span>
              {#if count > 1}<span class="trigger-count">{count}×</span>{/if}
              <span class="check-chevron">{expandedId === item.id ? '▴' : '▾'}</span>
            </div>
            {#if expandedId === item.id}
              <div class="check-why">{item.why}</div>
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  {/if}

  <!-- Webcam presence issues -->
  {#if presenceIssues.length > 0}
    <div class="presence-section">
      <span class="presence-label">📷 Your camera</span>
      {#each presenceIssues as issue}
        <div class="presence-issue">{issue}</div>
      {/each}
    </div>
  {:else if presencePositive}
    <div class="presence-positive">✓ {presencePositive}</div>
  {/if}
</div>

<style>
  .bl-panel {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    border-top: 1px solid #1e293b;
    padding-top: 0.5rem;
  }

  .mode-badge {
    font-size: var(--fs-xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    padding: 0.15rem 0.5rem;
    border-radius: 0.25rem;
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    align-self: flex-start;
  }
  .mode-listening { background: #0d1a2b; color: #60a5fa; border: 1px solid #1e3a5f; }
  .mode-answering { background: #0f1a0f; color: #4ade80; border: 1px solid #14532d; }

  /* AI coaching tip */
  .ai-tip {
    background: #0d1a2b;
    border: 1px solid #1a2d4a;
    border-left: 3px solid #f59e0b;
    border-radius: 0.5rem;
    padding: 0.55rem 0.7rem;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
  .ai-tip-urgent {
    border-left-color: #ef4444;
    background: #1a0808;
  }
  .ai-tip-empty {
    flex-direction: row;
    align-items: center;
    gap: 0.4rem;
    border-left-color: #1e293b;
  }
  .ai-tip-header {
    display: flex;
    align-items: center;
    gap: 0.3rem;
  }
  .ai-tip-icon { font-size: var(--fs-base); flex-shrink: 0; }
  .ai-tip-label {
    font-size: var(--fs-xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: #f59e0b;
  }
  .urgent-label { color: #ef4444; }
  .ai-tip-text {
    font-size: var(--fs-base);
    color: #e2e8f0;
    line-height: 1.45;
    font-weight: 500;
  }
  .ai-tip-why {
    font-size: var(--fs-sm);
    color: #64748b;
    line-height: 1.4;
    font-style: italic;
    border-top: 1px solid #1a2d4a;
    padding-top: 0.25rem;
    margin-top: 0.1rem;
  }
  .ai-tip-waiting { font-size: var(--fs-sm); color: #334155; font-style: italic; }

  /* Checklist */
  .checklist {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }
  .checklist-standing { margin-top: 0.1rem; }

  .check-item {
    border-radius: 0.375rem;
    border: 1px solid transparent;
    cursor: pointer;
    transition: background 0.15s, border-color 0.15s;
    padding: 0.3rem 0.5rem;
    background: #080d18;
  }
  .check-item:hover { background: #0d1a2b; }
  .check-item.highlighted {
    background: #0a1a10;
    border-color: #14532d;
    animation: itemglow 2s ease-in-out infinite;
  }
  .check-item.check-urgent {
    background: #1a0808;
    border-color: #7f1d1d;
    animation: urgentglow 1.5s ease-in-out infinite;
  }
  @keyframes itemglow {
    0%, 100% { border-color: #14532d; }
    50% { border-color: #166534; }
  }
  @keyframes urgentglow {
    0%, 100% { border-color: #7f1d1d; }
    50% { border-color: #991b1b; }
  }

  .check-row {
    display: flex;
    align-items: flex-start;
    gap: 0.35rem;
  }
  .check-icon { font-size: var(--fs-base); flex-shrink: 0; }
  .check-label {
    flex: 1;
    font-size: var(--fs-sm);
    color: #64748b;
    line-height: 1.3;
  }
  .highlighted-label { color: #4ade80; }
  .check-item.check-urgent .highlighted-label { color: #fca5a5; }

  .check-dot {
    width: 5px; height: 5px;
    border-radius: 50%;
    background: #4ade80;
    flex-shrink: 0;
    margin-top: 0.3rem;
    animation: dotpulse 1.2s ease-in-out infinite;
  }
  .check-item.check-urgent .check-dot { background: #ef4444; }
  @keyframes dotpulse {
    0%, 100% { opacity: 1; transform: scale(1); }
    50% { opacity: 0.4; transform: scale(0.7); }
  }

  .check-chevron {
    font-size: var(--fs-xs);
    color: #1e293b;
    flex-shrink: 0;
  }
  .check-why {
    font-size: var(--fs-sm);
    color: #475569;
    line-height: 1.45;
    padding-top: 0.3rem;
    padding-left: 1.1rem;
    border-top: 1px solid #0f172a;
    margin-top: 0.25rem;
  }
  .check-item.highlighted .check-why { color: #64748b; }

  .mode-hint {
    font-size: var(--fs-sm);
    color: #22c55e;
    font-style: italic;
    padding: 0.15rem 0 0 1.5rem;
    line-height: 1.3;
  }
  .check-item.check-urgent .mode-hint { color: #f87171; }

  .trigger-count {
    font-size: var(--fs-xs);
    color: #475569;
    flex-shrink: 0;
    font-variant-numeric: tabular-nums;
    margin-top: 0.1rem;
  }
  .check-item.highlighted .trigger-count { color: #166534; }

  /* Standing checks toggle */
  .standing-toggle {
    display: flex;
    align-items: center;
    justify-content: space-between;
    background: transparent;
    border: none;
    border-top: 1px solid #1e293b;
    padding: 0.2rem 0.3rem;
    cursor: pointer;
    width: 100%;
  }
  .standing-label {
    font-size: var(--fs-xs);
    color: #334155;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    font-weight: 600;
  }
  .standing-chevron { font-size: var(--fs-xs); color: #334155; }
  .standing-toggle:hover .standing-label,
  .standing-toggle:hover .standing-chevron { color: #475569; }

  /* Presence check */
  .presence-section {
    background: #0d1a0d;
    border: 1px solid #1a3a1a;
    border-left: 3px solid #15803d;
    border-radius: 0.4rem;
    padding: 0.35rem 0.6rem;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }
  .presence-label {
    font-size: var(--fs-xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: #4ade80;
  }
  .presence-issue {
    font-size: var(--fs-sm);
    color: #fca5a5;
    line-height: 1.3;
  }
  .presence-positive {
    font-size: var(--fs-sm);
    color: #4ade80;
    padding: 0.15rem 0.5rem;
    font-style: italic;
  }
</style>
