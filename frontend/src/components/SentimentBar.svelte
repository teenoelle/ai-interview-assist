<script lang="ts">
  const { emotion } = $props<{ emotion: string }>();

  const emotionConfig: Record<string, { color: string; icon: string; label: string }> = {
    engaged:   { color: '#22c55e', icon: '🎯', label: 'Engaged' },
    curious:   { color: '#3b82f6', icon: '🔍', label: 'Curious' },
    neutral:   { color: '#94a3b8', icon: '😐', label: 'Neutral' },
    skeptical: { color: '#f59e0b', icon: '🤔', label: 'Skeptical' },
    confused:  { color: '#f97316', icon: '😕', label: 'Confused' },
    bored:     { color: '#ef4444', icon: '😑', label: 'Bored' },
    pleased:   { color: '#a78bfa', icon: '😊', label: 'Pleased' },
  };

  const config = $derived(emotionConfig[emotion] ?? emotionConfig['neutral']);
</script>

<div class="sentiment-bar">
  <h3>Interviewer Sentiment</h3>
  {#if emotion}
    <div class="emotion" style="--color: {config.color}">
      <span class="icon">{config.icon}</span>
      <span class="label">{config.label}</span>
    </div>
  {:else}
    <p class="empty">Sentiment analysis will appear when video is captured...</p>
  {/if}
</div>

<style>
  .sentiment-bar {
    padding: 1rem;
  }
  h3 {
    font-size: 1rem;
    font-weight: 600;
    color: #94a3b8;
    margin-bottom: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .emotion {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.75rem 1.25rem;
    background: #1e293b;
    border-radius: 0.75rem;
    border-left: 4px solid var(--color);
  }
  .icon {
    font-size: 1.5rem;
  }
  .label {
    font-size: 1.2rem;
    font-weight: 700;
    color: var(--color);
  }
  .empty {
    color: #475569;
    font-style: italic;
    font-size: 0.85rem;
  }
</style>
