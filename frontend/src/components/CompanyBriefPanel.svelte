<script lang="ts">
  const { brief } = $props<{
    brief: { name: string; what_they_do: string; products: string[]; culture: string; recent_news: string; why_join: string } | null;
  }>();

  let expanded = $state(true);
</script>

{#if brief && brief.name}
  <div class="brief-panel">
    <button class="brief-toggle" onclick={() => expanded = !expanded}>
      <span class="brief-company">{brief.name}</span>
      <span class="brief-chevron">{expanded ? '▴' : '▾'}</span>
    </button>
    {#if expanded}
      <div class="brief-body">
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
      </div>
    {/if}
  </div>
{/if}

<style>
  .brief-panel { background: #060e1a; border: 1px solid #1a2d4a; border-radius: 0.5rem; overflow: hidden; }
  .brief-toggle { width: 100%; display: flex; align-items: center; justify-content: space-between; padding: 0.5rem 0.75rem; background: transparent; border: none; cursor: pointer; text-align: left; }
  .brief-toggle:hover { background: #0a1525; }
  .brief-company { font-size: 0.78rem; font-weight: 700; color: #60a5fa; }
  .brief-chevron { font-size: 0.6rem; color: #334155; }
  .brief-body { display: flex; flex-direction: column; gap: 0.4rem; padding: 0.5rem 0.75rem 0.75rem; border-top: 1px solid #0f1e33; }
  .brief-row { display: flex; flex-direction: column; gap: 0.1rem; }
  .brief-label { font-size: 0.58rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.07em; color: #334155; }
  .brief-val { font-size: 0.75rem; color: #94a3b8; line-height: 1.4; }
  .why-join .brief-val { color: #4ade80; }
</style>
