<script lang="ts">
  const {
    videoEmotion,
    audioEmotion,
    coachingWhy,
  } = $props<{
    videoEmotion: string;
    audioEmotion?: string;
    coachingWhy?: string;
  }>();

  const emotionConfig: Record<string, { color: string; icon: string; label: string }> = {
    engaged:      { color: '#22c55e', icon: '🎯', label: 'Engaged' },
    curious:      { color: '#3b82f6', icon: '🔍', label: 'Curious' },
    neutral:      { color: '#94a3b8', icon: '😐', label: 'Neutral' },
    skeptical:    { color: '#f59e0b', icon: '🤔', label: 'Skeptical' },
    confused:     { color: '#f97316', icon: '😕', label: 'Confused' },
    bored:        { color: '#ef4444', icon: '😑', label: 'Bored' },
    pleased:      { color: '#a78bfa', icon: '😊', label: 'Pleased' },
    enthusiastic: { color: '#10b981', icon: '✨', label: 'Enthusiastic' },
    'wrapping up':{ color: '#6366f1', icon: '🏁', label: 'Wrapping Up' },
  };

  const videoConfig = $derived(emotionConfig[videoEmotion] ?? emotionConfig['neutral']);
  const audioConfig = $derived(emotionConfig[audioEmotion ?? ''] ?? null);

  // Show audio divergence only when it differs from video
  const audioDiverges = $derived(
    audioEmotion && audioConfig && audioEmotion !== videoEmotion
  );

  const tooltipText = $derived(coachingWhy ?? '');
</script>

<div class="sentiment-bar">
  {#if videoEmotion}
    <div class="emotion" style="--color: {videoConfig.color}" title={tooltipText || undefined}>
      <span class="icon">{videoConfig.icon}</span>
      <div class="emotion-detail">
        <div class="label-row">
          <span class="label">{videoConfig.label}</span>
          {#if audioDiverges}
            <span class="audio-tag" style="--acolor: {audioConfig!.color}" title="Voice reads as {audioConfig!.label}">
              {audioConfig!.icon} {audioConfig!.label}
            </span>
          {/if}
        </div>
        {#if coachingWhy}
          <span class="why">{coachingWhy}</span>
        {/if}
      </div>
      {#if coachingWhy}
        <span class="tooltip-hint" title={coachingWhy}>?</span>
      {/if}
    </div>
  {:else}
    <p class="empty">Waiting for screen capture...</p>
  {/if}
</div>

<style>
  .sentiment-bar {
    padding: 0.75rem;
  }
  .emotion {
    display: flex;
    align-items: flex-start;
    gap: 0.6rem;
    padding: 0.5rem 0.9rem;
    background: #1e293b;
    border-radius: 0.6rem;
    border-left: 3px solid var(--color);
    cursor: default;
  }
  .icon { font-size: 1.2rem; flex-shrink: 0; margin-top: 0.05rem; }
  .emotion-detail {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    min-width: 0;
    flex: 1;
  }
  .label-row {
    display: flex;
    align-items: baseline;
    gap: 0.5rem;
    flex-wrap: wrap;
  }
  .label {
    font-size: var(--fs-base);
    font-weight: 700;
    color: var(--color);
    line-height: 1.2;
  }
  .audio-tag {
    font-size: var(--fs-xs);
    font-weight: 600;
    color: var(--acolor);
    background: #0d1a2b;
    border: 1px solid var(--acolor);
    border-radius: 0.25rem;
    padding: 0.05rem 0.3rem;
    cursor: help;
    opacity: 0.85;
  }
  .why {
    font-size: var(--fs-sm);
    color: #475569;
    line-height: 1.3;
    font-style: italic;
  }
  .tooltip-hint {
    font-size: var(--fs-xs);
    color: #334155;
    background: #0d1a2b;
    border: 1px solid #1e293b;
    border-radius: 50%;
    width: 1rem;
    height: 1rem;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    cursor: help;
    margin-top: 0.1rem;
    font-weight: 700;
  }
  .empty {
    color: #334155;
    font-style: italic;
    font-size: var(--fs-sm);
    padding: 0.25rem 0;
  }
</style>
