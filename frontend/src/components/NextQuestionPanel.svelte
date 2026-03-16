<script lang="ts">
  const { questions, loading, onPredict } = $props<{
    questions: string[];
    loading: boolean;
    onPredict: () => void;
  }>();

  let expanded = $state(true);
</script>

<div class="nq-panel">
  <div class="nq-header">
    <button class="nq-toggle" onclick={() => expanded = !expanded}>
      <span class="nq-title">Predicted Next Questions</span>
      <span class="nq-chevron">{expanded ? '▴' : '▾'}</span>
    </button>
    <button class="nq-refresh" onclick={onPredict} disabled={loading} title="Re-predict">
      {loading ? '⟳' : '↻'}
    </button>
  </div>
  {#if expanded}
    <div class="nq-list">
      {#if loading}
        <p class="nq-loading">Predicting...</p>
      {:else if questions.length === 0}
        <p class="nq-empty">Click ↻ to predict likely next questions</p>
      {:else}
        {#each questions as q, i}
          <div class="nq-item">
            <span class="nq-num">{i + 1}</span>
            <span class="nq-q">{q}</span>
          </div>
        {/each}
      {/if}
    </div>
  {/if}
</div>

<style>
  .nq-panel { background: #06101e; border: 1px solid #1a2d4a; border-radius: 0.4rem; overflow: hidden; }
  .nq-header { display: flex; align-items: center; justify-content: space-between; }
  .nq-toggle { flex: 1; display: flex; align-items: center; justify-content: space-between; padding: 0.4rem 0.6rem; background: transparent; border: none; cursor: pointer; text-align: left; }
  .nq-toggle:hover { background: #0a1525; }
  .nq-title { font-size: var(--fs-xs); font-weight: 700; text-transform: uppercase; letter-spacing: 0.07em; color: #334155; }
  .nq-chevron { font-size: var(--fs-xs); color: #1e293b; }
  .nq-refresh { padding: 0.3rem 0.5rem; background: transparent; border: none; border-left: 1px solid #1a2d4a; color: #334155; cursor: pointer; font-size: var(--fs-base); transition: color 0.15s; }
  .nq-refresh:hover:not(:disabled) { color: #60a5fa; }
  .nq-refresh:disabled { opacity: 0.4; }
  .nq-list { display: flex; flex-direction: column; gap: 0.3rem; padding: 0.4rem 0.6rem 0.5rem; border-top: 1px solid #0f1e33; }
  .nq-item { display: flex; align-items: flex-start; gap: 0.4rem; }
  .nq-num { font-size: var(--fs-xs); font-weight: 800; color: #334155; flex-shrink: 0; margin-top: 0.1rem; }
  .nq-q { font-size: 0.73rem; color: #4d7494; font-style: italic; line-height: 1.35; }
  .nq-loading, .nq-empty { font-size: var(--fs-sm); color: #334155; font-style: italic; margin: 0; text-align: center; padding: 0.25rem 0; }
</style>
