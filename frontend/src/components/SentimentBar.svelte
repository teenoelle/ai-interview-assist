<script lang="ts">
  const {
    videoEmotion,
    videoReason,
    coaching,
    audioEmotion,
    audioReason,
  } = $props<{
    videoEmotion: string;
    videoReason?: string;
    coaching?: string;
    audioEmotion?: string;
    audioReason?: string;
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
</script>

<div class="sentiment-bar">
  <!-- Video Sentiment -->
  <div class="section-label">
    <span class="label-icon">📷</span> Visual
  </div>
  {#if videoEmotion}
    <div class="emotion" style="--color: {videoConfig.color}">
      <span class="icon">{videoConfig.icon}</span>
      <div class="emotion-detail">
        <span class="label">{videoConfig.label}</span>
        {#if videoReason}
          <span class="reason">{videoReason}</span>
        {/if}
      </div>
    </div>
    {#if coaching}
      <div class="coaching-tip">
        <span class="coaching-icon">💡</span>
        <span class="coaching-text">{coaching}</span>
      </div>
    {/if}
  {:else}
    <p class="empty">Waiting for screen capture...</p>
  {/if}

  <!-- Audio Sentiment -->
  <div class="section-label" style="margin-top: 0.65rem;">
    <span class="label-icon">🎙️</span> Voice
  </div>
  {#if audioEmotion && audioConfig}
    <div class="emotion" style="--color: {audioConfig.color}">
      <span class="icon">{audioConfig.icon}</span>
      <div class="emotion-detail">
        <span class="label">{audioConfig.label}</span>
        {#if audioReason}
          <span class="reason">{audioReason}</span>
        {/if}
      </div>
    </div>
  {:else}
    <p class="empty">Waiting for speech...</p>
  {/if}
</div>

<style>
  .sentiment-bar {
    padding: 0.75rem;
  }
  .section-label {
    font-size: 0.58rem;
    font-weight: 700;
    color: #334155;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    margin-bottom: 0.3rem;
    display: flex;
    align-items: center;
    gap: 0.3rem;
  }
  .label-icon { font-size: 0.7rem; }
  .emotion {
    display: flex;
    align-items: flex-start;
    gap: 0.6rem;
    padding: 0.5rem 0.9rem;
    background: #1e293b;
    border-radius: 0.6rem;
    border-left: 3px solid var(--color);
  }
  .icon { font-size: 1.2rem; flex-shrink: 0; margin-top: 0.05rem; }
  .emotion-detail {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    min-width: 0;
  }
  .label {
    font-size: 0.9rem;
    font-weight: 700;
    color: var(--color);
    line-height: 1.2;
  }
  .reason {
    font-size: 0.65rem;
    color: #475569;
    line-height: 1.3;
    font-style: italic;
  }
  .coaching-tip {
    display: flex;
    align-items: flex-start;
    gap: 0.4rem;
    margin-top: 0.4rem;
    padding: 0.4rem 0.6rem;
    background: #1a2744;
    border-radius: 0.4rem;
    border-left: 2px solid #f59e0b;
  }
  .coaching-icon { flex-shrink: 0; font-size: 0.8rem; margin-top: 0.05rem; }
  .coaching-text { font-size: 0.72rem; color: #fcd34d; line-height: 1.35; }
  .empty {
    color: #334155;
    font-style: italic;
    font-size: 0.75rem;
    padding: 0.25rem 0;
  }
</style>
