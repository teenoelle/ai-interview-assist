<script lang="ts">
  const { keywords, mentionedSet } = $props<{
    keywords: string[];
    mentionedSet: Set<string>;
  }>();

  const mentioned = $derived(keywords.filter(k => mentionedSet.has(k)));
  const notYet = $derived(keywords.filter(k => !mentionedSet.has(k)));
  const pct = $derived(keywords.length > 0 ? Math.round((mentioned.length / keywords.length) * 100) : 0);
</script>

<div class="kw-panel">
  {#if keywords.length === 0}
    <p class="kw-empty">No keywords loaded. Add a job description in setup.</p>
  {:else}
    <div class="kw-progress">
      <div class="kw-bar" style="width: {pct}%"></div>
    </div>
    <div class="kw-stats">{mentioned.length}/{keywords.length} keywords mentioned</div>
    <div class="kw-list">
      {#each mentioned as kw}
        <span class="kw-chip kw-done">✓ {kw}</span>
      {/each}
      {#each notYet as kw}
        <span class="kw-chip kw-todo">{kw}</span>
      {/each}
    </div>
  {/if}
</div>

<style>
  .kw-panel { display: flex; flex-direction: column; gap: 0.4rem; }
  .kw-empty { font-size: 0.72rem; color: #334155; font-style: italic; margin: 0; }
  .kw-progress { height: 3px; background: #1e293b; border-radius: 9999px; overflow: hidden; }
  .kw-bar { height: 100%; background: #22c55e; border-radius: 9999px; transition: width 0.3s; }
  .kw-stats { font-size: 0.62rem; color: #475569; }
  .kw-list { display: flex; flex-wrap: wrap; gap: 0.25rem 0.3rem; }
  .kw-chip {
    font-size: 0.65rem; padding: 0.1rem 0.45rem;
    border-radius: 9999px; border: 1px solid;
    white-space: nowrap;
  }
  .kw-done { color: #22c55e; background: #071a0f; border-color: #14532d; }
  .kw-todo { color: #334155; background: #080d18; border-color: #1e293b; }
</style>
