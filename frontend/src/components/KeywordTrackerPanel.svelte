<script lang="ts">
  const { keywords, mentionedSet, interviewerRaisedSet = new Set(), keywordQuestionMap = {}, horizontal = false } = $props<{
    keywords: string[];
    mentionedSet: Set<string>;
    interviewerRaisedSet?: Set<string>;
    keywordQuestionMap?: Record<string, string>;
    horizontal?: boolean;
  }>();

  const mentioned = $derived(keywords.filter(k => mentionedSet.has(k)));
  const notYet = $derived(keywords.filter(k => !mentionedSet.has(k)));
  const pct = $derived(keywords.length > 0 ? Math.round((mentioned.length / keywords.length) * 100) : 0);

  let selectedKw = $state<string | null>(null);
  let definitions = $state<Record<string, string>>({});
  let loadingKw = $state<string | null>(null);
  let popupPos = $state<{ x: number; y: number } | null>(null);

  async function showDefinition(kw: string, e?: MouseEvent) {
    if (selectedKw === kw) { selectedKw = null; popupPos = null; return; }
    selectedKw = kw;
    if (horizontal && e) {
      const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
      popupPos = { x: rect.left, y: window.innerHeight - rect.top + 6 };
    }
    if (definitions[kw]) return;
    loadingKw = kw;
    try {
      const resp = await fetch('/api/keyword-definition', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ keyword: kw }),
      });
      if (resp.ok) {
        const data = await resp.json();
        definitions = { ...definitions, [kw]: data.definition };
      }
    } catch { /* ignore */ }
    loadingKw = null;
  }
</script>

{#if horizontal}
  <!-- Horizontal bottom-bar mode: chips wrap, definition shows as fixed popup -->
  <div class="kw-hbar">
    <div class="kw-hbar-row">
      <div class="kw-hbar-chips">
        {#each mentioned as kw}
          <button class="kw-chip kw-done" onclick={(e) => showDefinition(kw, e)} title="Click for definition">✓ {kw}</button>
        {/each}
        {#each notYet.filter(k => interviewerRaisedSet.has(k)) as kw}
          <button class="kw-chip kw-raised" onclick={(e) => showDefinition(kw, e)}
            title={keywordQuestionMap[kw] ? `Asked in: "${keywordQuestionMap[kw]}"` : 'Interviewer raised this — click for info'}>↑ {kw}</button>
        {/each}
        {#each notYet.filter(k => !interviewerRaisedSet.has(k)) as kw}
          <button class="kw-chip kw-todo" onclick={(e) => showDefinition(kw, e)} title="Click for definition">{kw}</button>
        {/each}
      </div>
      <span class="kw-hbar-stats">{mentioned.length}/{keywords.length}</span>
    </div>
  </div>
  {#if selectedKw && popupPos}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="kw-popup-fixed" style="left: {popupPos.x}px; bottom: {popupPos.y}px" onclick={(e) => e.stopPropagation()}>
      <div class="kw-def-header">
        <span class="kw-def-word">{selectedKw}</span>
        <button class="kw-def-close" onclick={() => { selectedKw = null; popupPos = null; }}>✕</button>
      </div>
      {#if loadingKw === selectedKw}
        <p class="kw-def-text kw-def-loading">Loading…</p>
      {:else if definitions[selectedKw]}
        <p class="kw-def-text">{definitions[selectedKw]}</p>
      {/if}
    </div>
  {/if}
{:else}
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
          <button class="kw-chip kw-done" onclick={() => showDefinition(kw)} title="Click for definition">✓ {kw}</button>
        {/each}
        {#each notYet.filter(k => interviewerRaisedSet.has(k)) as kw}
          <button class="kw-chip kw-raised" onclick={() => showDefinition(kw)}
            title={keywordQuestionMap[kw] ? `Asked in: "${keywordQuestionMap[kw]}"` : 'Interviewer raised this — click for info'}>↑ {kw}</button>
        {/each}
        {#each notYet.filter(k => !interviewerRaisedSet.has(k)) as kw}
          <button class="kw-chip kw-todo" onclick={() => showDefinition(kw)} title="Click for definition">{kw}</button>
        {/each}
      </div>

      {#if selectedKw}
        <div class="kw-def">
          <div class="kw-def-header">
            <span class="kw-def-word">{selectedKw}</span>
            <button class="kw-def-close" onclick={() => selectedKw = null}>✕</button>
          </div>
          {#if loadingKw === selectedKw}
            <p class="kw-def-text kw-def-loading">Loading…</p>
          {:else if definitions[selectedKw]}
            <p class="kw-def-text">{definitions[selectedKw]}</p>
          {/if}
        </div>
      {/if}
    {/if}
  </div>
{/if}

<style>
  /* Horizontal bar mode */
  .kw-hbar {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    width: 100%;
    min-width: 0;
  }
  .kw-hbar-row {
    display: flex;
    align-items: flex-start;
    gap: 0.5rem;
    min-width: 0;
  }
  .kw-hbar-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem 0.3rem;
    flex: 1;
    min-width: 0;
  }
  .kw-hbar-stats {
    font-size: var(--fs-xs);
    color: #475569;
    white-space: nowrap;
    flex-shrink: 0;
    padding-top: 0.15rem;
  }

  .kw-panel { display: flex; flex-direction: column; gap: 0.4rem; }
  .kw-empty { font-size: var(--fs-sm); color: #334155; font-style: italic; margin: 0; }
  .kw-progress { height: 3px; background: #1e293b; border-radius: 9999px; overflow: hidden; }
  .kw-bar { height: 100%; background: #22c55e; border-radius: 9999px; transition: width 0.3s; }
  .kw-stats { font-size: var(--fs-xs); color: #475569; }
  .kw-list { display: flex; flex-wrap: wrap; gap: 0.25rem 0.3rem; }
  .kw-chip {
    font-size: var(--fs-sm); padding: 0.1rem 0.45rem;
    border-radius: 9999px; border: 1px solid;
    white-space: nowrap; cursor: pointer;
    background: none; transition: opacity 0.15s;
  }
  .kw-chip:hover { opacity: 0.75; }
  .kw-done { color: #22c55e; background: #071a0f; border-color: #14532d; }
  .kw-todo { color: #334155; background: #080d18; border-color: #1e293b; }
  .kw-raised {
    color: #fbbf24; background: #1a1000; border-color: #92400e;
    animation: kw-pulse 1.5s ease-in-out infinite;
  }
  @keyframes kw-pulse { 0%, 100% { border-color: #92400e; } 50% { border-color: #f59e0b; } }
  .kw-def {
    background: #0a1020; border: 1px solid #1e293b; border-left: 2px solid #3b82f6;
    border-radius: 0.4rem; padding: 0.6rem 0.75rem;
    display: flex; flex-direction: column; gap: 0.35rem;
    animation: fadeIn 0.15s ease-out;
  }
  @keyframes fadeIn { from { opacity: 0; transform: translateY(-3px); } to { opacity: 1; transform: none; } }
  .kw-def-header { display: flex; align-items: center; justify-content: space-between; }
  .kw-def-word { font-size: var(--fs-sm); font-weight: 700; color: #60a5fa; }
  .kw-def-close {
    background: none; border: none; color: #334155; font-size: var(--fs-sm);
    cursor: pointer; padding: 0; line-height: 1;
  }
  .kw-def-close:hover { color: #64748b; }
  .kw-def-text { margin: 0; font-size: var(--fs-sm); color: #94a3b8; line-height: 1.5; }
  .kw-def-loading { color: #334155; font-style: italic; }
  .kw-popup-fixed {
    position: fixed;
    z-index: 100;
    background: #0a1020;
    border: 1px solid #1e293b;
    border-left: 2px solid #3b82f6;
    border-radius: 0.4rem;
    padding: 0.6rem 0.75rem;
    max-width: 320px;
    min-width: 180px;
    box-shadow: 0 -4px 24px rgba(0,0,0,0.5);
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    animation: fadeIn 0.12s ease-out;
  }
</style>
