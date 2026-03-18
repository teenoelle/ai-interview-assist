<script lang="ts">
  const { emotion, coaching, coachingWhy } = $props<{
    emotion: string;
    coaching?: string;
    coachingWhy?: string;
  }>();

  // Body language checklist — each item has:
  //   id, icon, label, why, and which emotions should highlight it
  const items = [
    {
      id: 'eye',
      icon: '👁️',
      label: 'Eye contact with camera',
      why: 'Looking at the camera lens (not the screen) simulates direct eye contact — it signals confidence and holds the interviewer\'s attention.',
      trigger: ['skeptical', 'bored', 'confused'],
    },
    {
      id: 'posture',
      icon: '🪑',
      label: 'Upright, open posture',
      why: 'Sitting straight with shoulders back projects authority and energy. Slouching reads as disinterest or low confidence.',
      trigger: ['bored', 'neutral', 'skeptical'],
    },
    {
      id: 'nod',
      icon: '🙂',
      label: 'Nod and react visibly',
      why: 'Visible reactions (nodding, small smiles) signal active listening and make you appear engaged even when the interviewer is speaking.',
      trigger: ['neutral', 'bored'],
    },
    {
      id: 'hands',
      icon: '🤲',
      label: 'Calm, open hand gestures',
      why: 'Using hands expressively (visible to camera, below chin) conveys enthusiasm and openness. Hiding or fidgeting hands signals anxiety.',
      trigger: ['skeptical', 'confused'],
    },
    {
      id: 'expression',
      icon: '😊',
      label: 'Warm, natural expression',
      why: 'A genuine, relaxed expression builds rapport. An overly serious or tense face can make the interviewer feel they\'re interrogating rather than conversing.',
      trigger: ['bored', 'neutral', 'skeptical'],
    },
    {
      id: 'pace',
      icon: '⏱️',
      label: 'Pause before answering',
      why: 'A deliberate 1–2 second pause before replying shows you\'re thoughtful, not rehearsed. It also slows your pace, reducing filler words.',
      trigger: ['curious', 'engaged', 'skeptical'],
    },
  ];

  let expandedId = $state<string | null>(null);

  function isHighlighted(item: typeof items[0]): boolean {
    return item.trigger.includes(emotion);
  }

  function toggle(id: string) {
    expandedId = expandedId === id ? null : id;
  }
</script>

<div class="bl-panel">
  <div class="bl-header">
    <span class="bl-title">Your Presence</span>
  </div>

  <!-- Dynamic AI coaching tip -->
  {#if coaching}
    <div class="ai-tip">
      <div class="ai-tip-header">
        <span class="ai-tip-icon">💡</span>
        <span class="ai-tip-label">Right now</span>
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

  <!-- Standing checklist -->
  <div class="checklist">
    {#each items as item (item.id)}
      {@const highlighted = isHighlighted(item)}
      <div class="check-item" class:highlighted onclick={() => toggle(item.id)}>
        <div class="check-row">
          <span class="check-icon">{item.icon}</span>
          <span class="check-label">{item.label}</span>
          {#if highlighted}
            <span class="check-dot"></span>
          {/if}
          <span class="check-chevron">{expandedId === item.id ? '▴' : '▾'}</span>
        </div>
        {#if expandedId === item.id}
          <div class="check-why">{item.why}</div>
        {/if}
      </div>
    {/each}
  </div>
</div>

<style>
  .bl-panel {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    border-top: 1px solid #1e293b;
    padding-top: 0.65rem;
  }

  .bl-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .bl-title {
    font-size: var(--fs-xs);
    font-weight: 700;
    color: #334155;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

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
  @keyframes itemglow {
    0%, 100% { border-color: #14532d; }
    50% { border-color: #166534; }
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
  .check-item.highlighted .check-label { color: #4ade80; }

  .check-dot {
    width: 5px; height: 5px;
    border-radius: 50%;
    background: #4ade80;
    flex-shrink: 0;
    animation: dotpulse 1.2s ease-in-out infinite;
  }
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
</style>
