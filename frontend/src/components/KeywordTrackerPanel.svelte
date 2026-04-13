<script lang="ts">
  const { keywords, mentionedSet, flashSet = new Set(), interviewerRaisedSet = new Set(), keywordQuestionMap = {}, horizontal = false, popupBottom = 60, onLoad, loading = false } = $props<{
    keywords: string[];
    mentionedSet: Set<string>;
    flashSet?: Set<string>;
    interviewerRaisedSet?: Set<string>;
    keywordQuestionMap?: Record<string, string>;
    horizontal?: boolean;
    popupBottom?: number;
    onLoad?: () => void;
    loading?: boolean;
  }>();

  const mentioned = $derived(keywords.filter(k => mentionedSet.has(k)));
  const notYet = $derived(keywords.filter(k => !mentionedSet.has(k)));

  let selectedKw = $state<string | null>(null);
  let definitions = $state<Record<string, { definition: string; tip: string }>>({});
  let loadingKw = $state<string | null>(null);
  let fetching = new Set<string>();
  let popupOpen = $state(false);
  let chipStyle = $state<'highlight' | 'invert'>(
    (localStorage.getItem('kw-chip-style') as 'highlight' | 'invert') ?? 'highlight'
  );
  function toggleChipStyle() {
    chipStyle = chipStyle === 'highlight' ? 'invert' : 'highlight';
    localStorage.setItem('kw-chip-style', chipStyle);
  }

  async function fetchDefinition(kw: string) {
    if (definitions[kw] || fetching.has(kw)) return;
    fetching.add(kw);
    try {
      const resp = await fetch('/api/keyword-definition', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ keyword: kw }),
      });
      if (resp.ok) {
        const data = await resp.json();
        definitions = { ...definitions, [kw]: { definition: data.definition ?? '', tip: data.tip ?? '' } };
      }
    } catch { /* ignore */ }
    fetching.delete(kw);
  }

  // Preload all keyword definitions with stagger to avoid hammering the API
  $effect(() => {
    const kws = keywords.slice();
    kws.forEach((kw, i) => {
      if (!definitions[kw]) setTimeout(() => fetchDefinition(kw), i * 350);
    });
  });

  async function showDefinition(kw: string, e?: MouseEvent) {
    if (selectedKw === kw) { selectedKw = null; popupOpen = false; return; }
    selectedKw = kw;
    if (horizontal) popupOpen = true;
    if (!definitions[kw]) {
      loadingKw = kw;
      await fetchDefinition(kw);
      loadingKw = null;
    }
  }

  function closePopup() { selectedKw = null; popupOpen = false; }
</script>

{#if horizontal}
  <!-- Horizontal bottom-bar mode: chips wrap, definition shows as fixed popup -->
  <div class="kw-hbar">
    <div class="kw-hbar-row">
      <div class="kw-hbar-chips">
        {#if keywords.length === 0}
          {#if loading}
            <span class="kw-empty">Loading…</span>
          {:else if onLoad}
            <button class="kw-load-btn" onclick={onLoad}>↻ Load keywords</button>
          {/if}
        {:else}
          {#each mentioned as kw}
            <button class="kw-chip kw-done"
              class:kw-flash={flashSet.has(kw)}
              class:kw-active-highlight={selectedKw === kw && chipStyle === 'highlight'}
              class:kw-active-invert={selectedKw === kw && chipStyle === 'invert'}
              onclick={(e) => showDefinition(kw, e)}>✓ {kw}{#if selectedKw === kw} ▾{/if}</button>
          {/each}
          {#each notYet.filter(k => interviewerRaisedSet.has(k)) as kw}
            <button class="kw-chip kw-raised"
              class:kw-active-highlight={selectedKw === kw && chipStyle === 'highlight'}
              class:kw-active-invert={selectedKw === kw && chipStyle === 'invert'}
              onclick={(e) => showDefinition(kw, e)}>↑ {kw}{#if selectedKw === kw} ▾{/if}</button>
          {/each}
          {#each notYet.filter(k => !interviewerRaisedSet.has(k)) as kw}
            <button class="kw-chip kw-todo"
              class:kw-active-highlight={selectedKw === kw && chipStyle === 'highlight'}
              class:kw-active-invert={selectedKw === kw && chipStyle === 'invert'}
              onclick={(e) => showDefinition(kw, e)}>{kw}{#if selectedKw === kw} ▾{/if}</button>
          {/each}
        {/if}
      </div>
      <div class="kw-hbar-meta">
        {#if keywords.length > 0}
          <span class="kw-hbar-stats">{mentioned.length}/{keywords.length}</span>
          <button class="kw-style-toggle" onclick={toggleChipStyle} title="Toggle selected chip style">
            {chipStyle === 'highlight' ? '1' : '3'}
          </button>
        {/if}
      </div>
    </div>
  </div>
  {#if selectedKw && popupOpen}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="kw-popup-fixed" style="bottom:{popupBottom}px" onclick={(e) => e.stopPropagation()}>
      <div class="kw-def-header">
        <span class="kw-def-word">{selectedKw}</span>
        <button class="kw-def-close" onclick={closePopup}>✕</button>
      </div>
      {#if loadingKw === selectedKw}
        <p class="kw-def-text kw-def-loading">Loading…</p>
      {:else if definitions[selectedKw]}
        <p class="kw-def-text">{definitions[selectedKw].definition}</p>
        {#if definitions[selectedKw].tip}
          <p class="kw-def-tip">{definitions[selectedKw].tip}</p>
        {/if}
      {/if}
    </div>
  {/if}
{:else}
  <div class="kw-panel">
    {#if keywords.length === 0}
      {#if loading}
        <p class="kw-empty">Loading…</p>
      {:else if onLoad}
        <button class="kw-load-btn" onclick={onLoad}>↻ Load keywords</button>
      {:else}
        <p class="kw-empty">No keywords loaded. Add a job description in setup.</p>
      {/if}
    {:else}
      <!-- Header row: count + hover-reveal style toggle -->
      <div class="kw-header">
        <span class="kw-count">{mentioned.length}/{keywords.length}</span>
        <button class="kw-style-toggle kw-style-toggle-inline" onclick={toggleChipStyle} title="Toggle selected chip style — style 1 (highlight) vs style 3 (invert)">{chipStyle === 'highlight' ? '1' : '3'}</button>
      </div>
      <!-- Chips-only row -->
      <div class="kw-list">
        {#each mentioned as kw}
          <button class="kw-chip kw-done"
            class:kw-flash={flashSet.has(kw)}
            class:kw-active-highlight={selectedKw === kw && chipStyle === 'highlight'}
            class:kw-active-invert={selectedKw === kw && chipStyle === 'invert'}
            onclick={() => showDefinition(kw)}>✓ {kw}{#if selectedKw === kw} ▾{/if}</button>
        {/each}
        {#each notYet.filter(k => interviewerRaisedSet.has(k)) as kw}
          <button class="kw-chip kw-raised"
            class:kw-active-highlight={selectedKw === kw && chipStyle === 'highlight'}
            class:kw-active-invert={selectedKw === kw && chipStyle === 'invert'}
            onclick={() => showDefinition(kw)}>↑ {kw}{#if selectedKw === kw} ▾{/if}</button>
        {/each}
        {#each notYet.filter(k => !interviewerRaisedSet.has(k)) as kw}
          <button class="kw-chip kw-todo"
            class:kw-active-highlight={selectedKw === kw && chipStyle === 'highlight'}
            class:kw-active-invert={selectedKw === kw && chipStyle === 'invert'}
            onclick={() => showDefinition(kw)}>{kw}{#if selectedKw === kw} ▾{/if}</button>
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
            <p class="kw-def-text">{definitions[selectedKw].definition}</p>
            {#if definitions[selectedKw].tip}
              <p class="kw-def-tip">{definitions[selectedKw].tip}</p>
            {/if}
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
  .kw-hbar-meta {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 0.3rem;
    flex-shrink: 0;
  }
  .kw-hbar-stats {
    font-size: var(--fs-xs);
    color: #475569;
    white-space: nowrap;
  }

  .kw-panel { display: flex; flex-direction: column; gap: 0.4rem; }
  .kw-empty { font-size: var(--fs-sm); color: #334155; font-style: italic; margin: 0; }

  /* Header row: count + hover-reveal style toggle */
  .kw-header {
    display: flex;
    align-items: center;
    gap: 0.3rem;
  }
  .kw-header:not(:hover) .kw-style-toggle-inline {
    opacity: 0;
    pointer-events: none;
  }
  .kw-style-toggle-inline {
    transition: opacity 0.15s;
  }

  /* Chips-only row */
  .kw-list {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.25rem 0.3rem;
  }

  .kw-count {
    font-size: var(--fs-xs);
    color: #475569;
    white-space: nowrap;
    margin-right: 0.1rem;
  }

  .kw-style-toggle {
    font-size: var(--fs-xs); color: #334155; background: none;
    border: 1px solid #1e293b; border-radius: 0.2rem;
    padding: 0.05rem 0.35rem; cursor: pointer; white-space: nowrap;
    transition: all 0.15s;
  }
  .kw-style-toggle:hover { border-color: #334155; color: #64748b; }
  .kw-style-toggle-inline { transition: opacity 0.15s, border-color 0.15s, color 0.15s; }

  .kw-load-btn {
    font-size: var(--fs-xs); color: #475569; background: none;
    border: 1px solid #1e293b; border-radius: 0.3rem;
    padding: 0.15rem 0.5rem; cursor: pointer;
    transition: all 0.15s;
  }
  .kw-load-btn:hover { color: #94a3b8; border-color: #334155; }

  .kw-chip {
    font-size: var(--fs-sm); padding: 0.1rem 0.45rem;
    border-radius: 0.3rem; border: 1px solid;
    white-space: nowrap; cursor: pointer;
    background: none; transition: opacity 0.15s;
  }
  .kw-chip:hover { opacity: 0.75; }
  .kw-done { color: #22c55e; background: #071a0f; border-color: #14532d; }
  .kw-flash { animation: kw-flash 0.6s ease-out 3; }
  @keyframes kw-flash { 0% { background: #22c55e; color: #031a07; border-color: #22c55e; } 100% { background: #071a0f; color: #22c55e; border-color: #14532d; } }
  .kw-todo { color: #334155; background: #080d18; border-color: #1e293b; }
  .kw-raised {
    color: #fbbf24; background: #1a1000; border-color: #92400e;
    animation: kw-pulse 1.5s ease-in-out infinite;
  }
  @keyframes kw-pulse { 0%, 100% { border-color: #92400e; } 50% { border-color: #f59e0b; } }

  /* Style 1: highlight — bright border + glow, chip keeps its color */
  .kw-active-highlight {
    border-width: 2px;
    box-shadow: 0 0 0 2px rgba(96, 165, 250, 0.25);
    filter: brightness(1.5);
    animation: none;
  }
  /* Style 3: invert — fill with the chip's accent color, dark text */
  .kw-done.kw-active-invert  { background: #22c55e; color: #031a07; border-color: #22c55e; animation: none; }
  .kw-todo.kw-active-invert  { background: #475569; color: #f1f5f9; border-color: #475569; animation: none; }
  .kw-raised.kw-active-invert { background: #fbbf24; color: #1a1000; border-color: #fbbf24; animation: none; }

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
  .kw-def-tip { margin: 0; font-size: var(--fs-sm); color: #60a5fa; line-height: 1.5; border-top: 1px solid #1e293b; padding-top: 0.3rem; }
  .kw-def-loading { color: #334155; font-style: italic; }
  .kw-popup-fixed {
    position: fixed;
    left: 8px;
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
