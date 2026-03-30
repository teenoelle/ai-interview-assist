<script lang="ts">
  const { brief, companyName = '', onLoad, loading = false } = $props<{
    brief: { name: string; what_they_do: string; products: string[]; culture: string; recent_news: string; why_join: string } | null;
    companyName?: string;
    onLoad?: () => void;
    loading?: boolean;
  }>();

  let expanded = $state(false);
  let loadTriggered = $state(false);

  function toggle() {
    expanded = !expanded;
    if (expanded && !loadTriggered && !brief && onLoad) {
      loadTriggered = true;
      onLoad();
    }
  }

  const displayName = $derived(brief?.name || companyName);
</script>

{#if displayName}
  <div class="brief-panel">
    <button class="brief-toggle" onclick={toggle}>
      <div class="brief-toggle-inner">
        <span class="brief-label-header">Company</span>
        <span class="brief-company">{displayName}</span>
      </div>
      <span class="brief-chevron">{expanded ? '▴' : '▾'}</span>
    </button>
    {#if expanded}
      <div class="brief-body">
        {#if loading}
          <span class="brief-loading">Loading…</span>
        {:else if brief}
          <div class="brief-row">
            <span class="brief-label">What they do</span>
            <span class="brief-val">{brief.what_they_do}</span>
          </div>
          {#if brief.products.length > 0}
            <div class="brief-row">
              <span class="brief-label">Products</span>
              <span class="brief-val">{brief.products.join(' · ')}</span>
            </div>
          {/if}
          <div class="brief-row">
            <span class="brief-label">Culture</span>
            <span class="brief-val">{brief.culture}</span>
          </div>
          {#if brief.recent_news && brief.recent_news !== 'Not found'}
            <div class="brief-row">
              <span class="brief-label">News</span>
              <span class="brief-val">{brief.recent_news}</span>
            </div>
          {/if}
          <div class="brief-row why-join">
            <span class="brief-label">Why join</span>
            <span class="brief-val">{brief.why_join}</span>
          </div>
        {/if}
      </div>
    {/if}
  </div>
{/if}

<style>
  .brief-panel { background: #060e1a; border: 1px solid #1a2d4a; border-radius: 0.5rem; overflow: hidden; }
  .brief-toggle { width: 100%; display: flex; align-items: center; justify-content: space-between; padding: 0.5rem 0.75rem; background: transparent; border: none; cursor: pointer; text-align: left; }
  .brief-toggle:hover { background: #0a1525; }
  .brief-company { font-size: var(--fs-base); font-weight: 700; color: #60a5fa; }
  .brief-chevron { font-size: var(--fs-xs); color: #334155; }
  .brief-body { display: flex; flex-direction: column; gap: 0.4rem; padding: 0.5rem 0.75rem 0.75rem; border-top: 1px solid #0f1e33; }
  .brief-row { display: flex; flex-direction: column; gap: 0.1rem; }
  .brief-label { font-size: var(--fs-xs); font-weight: 700; text-transform: uppercase; letter-spacing: 0.07em; color: #334155; }
  .brief-val { font-size: var(--fs-sm); color: #94a3b8; line-height: 1.4; }
  .why-join .brief-val { color: #94a3b8; }
  .brief-loading { font-size: var(--fs-xs); color: #475569; font-style: italic; }
  .brief-toggle-inner { display: flex; flex-direction: column; gap: 0.1rem; text-align: left; }
  .brief-label-header { font-size: var(--fs-xs); font-weight: 700; text-transform: uppercase; letter-spacing: 0.07em; color: #334155; }
</style>
