<script lang="ts">
  import { EMOTION_CONFIG } from '../lib/emotions';

  const {
    videoEmotion,
    audioEmotion,
    coachingWhy,
  } = $props<{
    videoEmotion: string;
    audioEmotion?: string;
    coachingWhy?: string;
  }>();

  const videoConfig = $derived(EMOTION_CONFIG[videoEmotion] ?? EMOTION_CONFIG['neutral']);
  const audioConfig = $derived(EMOTION_CONFIG[audioEmotion ?? ''] ?? null);
  const audioDiverges = $derived(audioEmotion && audioConfig && audioEmotion !== videoEmotion);

  let whyOpen = $state(false);
</script>

<div class="sentiment-bar">
  {#if videoEmotion}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="emotion" class:emotion-clickable={!!coachingWhy} style="--color: {videoConfig.color}"
      onclick={() => { if (coachingWhy) whyOpen = !whyOpen; }}>
      <span class="icon">{videoConfig.icon}</span>
      <div class="emotion-detail">
        <div class="label-row">
          <span class="label">{videoConfig.label}</span>
          {#if audioDiverges}
            <span class="audio-tag" style="--acolor: {audioConfig!.color}" title="Voice reads as {audioConfig!.label}">
              {audioConfig!.icon} {audioConfig!.label}
            </span>
          {/if}
          {#if coachingWhy}
            <span class="why-hint">{whyOpen ? '▾' : '▸'}</span>
          {/if}
        </div>
        {#if whyOpen && coachingWhy}
          <span class="why">{coachingWhy}</span>
        {/if}
      </div>
    </div>
  {:else}
    <p class="empty">Waiting for screen capture...</p>
  {/if}
</div>

<style>
  .sentiment-bar {
    padding: 0.5rem 0.75rem 0.75rem;
  }
  .emotion {
    display: flex;
    align-items: flex-start;
    gap: 0.6rem;
    padding: 0.5rem 0.9rem;
    background: #1e293b;
    border-radius: 0.6rem;
    border-left: 3px solid var(--color);
  }
  .emotion-clickable { cursor: pointer; }
  .emotion-clickable:hover { background: #243044; }
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
  .why-hint { font-size: var(--fs-xs); color: #334155; margin-left: auto; }
  .why {
    font-size: var(--fs-sm);
    color: #475569;
    line-height: 1.35;
    font-style: italic;
    padding-top: 0.15rem;
  }
  .empty {
    color: #334155;
    font-style: italic;
    font-size: var(--fs-sm);
    padding: 0.25rem 0;
  }
</style>
