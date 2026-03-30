<script lang="ts">
  const { interviewers, onLoad, onReload, loading = false, onLoadProfile, loadingProfileIndices = [] } = $props<{
    interviewers: Array<{ name: string; role: string; background: string; tenure: string; rapport_tips: string[] }>;
    onLoad?: () => void;
    onReload?: () => void;
    loading?: boolean;
    onLoadProfile?: (index: number) => void;
    loadingProfileIndices?: number[];
  }>();

  let expanded = $state(false);
  let expandedCards = $state<number[]>([]);
  // Track cards that have had a load requested (prevents flash of empty state)
  let loadRequested = $state<number[]>([]);

  function toggleCard(i: number) {
    if (expandedCards.includes(i)) {
      expandedCards = expandedCards.filter(x => x !== i);
    } else {
      expandedCards = [...expandedCards, i];
      const iv = interviewers[i];
      if (iv && !iv.background && onLoadProfile) {
        loadRequested = [...loadRequested, i];
        onLoadProfile(i);
      }
    }
  }

  function retryCard(i: number) {
    loadRequested = [...loadRequested, i];
    onLoadProfile?.(i);
  }

  function firstName(name: string) { return name.trim().split(/\s+/)[0] ?? ''; }
  function lastName(name: string) { const p = name.trim().split(/\s+/); return p.length > 1 ? p.slice(1).join(' ') : ''; }

  function parseTip(tip: string): { keyword: string; text: string } {
    const m = tip.match(/^\[([^\]]+)\]\s*(.*)/s);
    return m ? { keyword: m[1], text: m[2].trim() } : { keyword: '', text: tip };
  }
</script>

<div class="profiles-panel">
    <button class="profiles-toggle" onclick={() => expanded = !expanded}>
      <div class="profiles-toggle-inner">
        <span class="profiles-label-header">Interviewers</span>
        {#if !expanded}<span class="profiles-names-preview">{interviewers.map(iv => firstName(iv.name)).join(' · ')}</span>{/if}
      </div>
      <div class="profiles-toggle-right">
        {#if onReload && interviewers.length > 0 && !loading}
          <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
          <span class="profiles-reload-btn" role="button" tabindex="0"
            onclick={(e) => { e.stopPropagation(); onReload?.(); }}
            onkeydown={(e) => e.key === 'Enter' && (e.stopPropagation(), onReload?.())}
            title="Re-run setup to update interviewers, then reload">⟳</span>
        {/if}
        <span class="profiles-chevron">{expanded ? '▴' : '▾'}</span>
      </div>
    </button>
    {#if expanded}
  <div class="profiles">
    {#if interviewers.length === 0}
      <div class="profiles-empty">
        {#if onLoad}
          <button class="profiles-load-btn" onclick={onLoad} disabled={loading}>
            {loading ? 'Loading…' : '⟳ Load interviewer profiles'}
          </button>
        {/if}
        {#if !loading}<p class="rapport-empty">No profiles loaded yet.</p>{/if}
      </div>
    {/if}
    {#each interviewers as iv, i}
      {@const cardCollapsed = !expandedCards.includes(i)}
      {@const cardLoading = loadingProfileIndices.includes(i) || (loadRequested.includes(i) && !iv.background)}
      {@const cardLoaded = !!iv.background}
      <div class="profile-card">
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="profile-header" onclick={() => toggleCard(i)} role="button" tabindex="0"
          onkeydown={(e) => e.key === 'Enter' && toggleCard(i)}>
          <div class="profile-name-block">
            <span class="profile-name">
              <span class="name-first">{firstName(iv.name)}</span>{#if lastName(iv.name)}<span class="name-last">{lastName(iv.name)}</span>{/if}
            </span>
            {#if iv.role}<span class="profile-role" class:role-collapsed={cardCollapsed}>{iv.role}</span>{/if}
          </div>
          <span class="card-chevron">{cardCollapsed ? '▾' : '▴'}</span>
        </div>
        {#if !cardCollapsed}
          {#if cardLoading}
            <span class="profile-loading">Loading profile…</span>
          {:else if cardLoaded}
            <div class="profile-field">
              <span class="profile-field-label">Background</span>
              <span class="profile-field-value">{iv.background}</span>
            </div>
            {#if iv.tenure}
              <div class="profile-field">
                <span class="profile-field-label">Tenure</span>
                <span class="profile-field-value">{iv.tenure}</span>
              </div>
            {/if}
            <div class="rapport-section">
              <span class="rapport-label">Rapport Tips</span>
              {#if iv.rapport_tips?.length > 0}
                <div class="rapport-tips-list">
                  {#each iv.rapport_tips as tip}
                    {@const p = parseTip(tip)}
                    <div class="rapport-tip">
                      {#if p.keyword}<span class="rapport-kw">{p.keyword}</span>{/if}
                      <span class="rapport-text">{p.text}</span>
                    </div>
                  {/each}
                </div>
              {:else}
                <span class="rapport-empty">No rapport tips generated.</span>
              {/if}
            </div>
          {:else}
            <button class="profile-retry-btn" onclick={(e) => { e.stopPropagation(); retryCard(i); }}>↺ Load profile</button>
          {/if}
        {/if}
      </div>
    {/each}
  </div>
    {/if}
  </div>

<style>
  .profiles-panel { background: #060e1a; border: 1px solid #1a2d4a; border-radius: 0.5rem; overflow: hidden; }
  .profiles-toggle { width: 100%; display: flex; align-items: center; justify-content: space-between; padding: 0.5rem 0.75rem; background: transparent; border: none; cursor: pointer; text-align: left; }
  .profiles-toggle:hover { background: #0a1525; }
  .profiles-toggle-inner { display: flex; flex-direction: column; gap: 0.1rem; text-align: left; }
  .profiles-label-header { font-size: var(--fs-xs); font-weight: 700; text-transform: uppercase; letter-spacing: 0.07em; color: #334155; }
  .profiles-names-preview { font-size: var(--fs-sm); color: #60a5fa; font-weight: 600; }
  .profiles-toggle-right { display: flex; align-items: center; gap: 0.4rem; }
  .profiles-chevron { font-size: var(--fs-xs); color: #334155; }
  .profiles-reload-btn { font-size: var(--fs-xs); color: #334155; cursor: pointer; padding: 0.1rem 0.2rem; border-radius: 0.2rem; transition: color 0.12s; }
  .profiles-reload-btn:hover { color: #60a5fa; }
  .profiles { display: flex; flex-direction: column; gap: 0.5rem; padding: 0.5rem 0.75rem 0.75rem; border-top: 1px solid #0f1e33; }
  .profile-card { background: #060e1a; border: 1px solid #1a2d4a; border-radius: 0.4rem; padding: 0.6rem 0.75rem; display: flex; flex-direction: column; gap: 0.5rem; }
  .profile-header { display: flex; align-items: flex-start; gap: 0.5rem; cursor: pointer; user-select: none; min-width: 0; }
  .profile-name-block { display: flex; flex-direction: column; gap: 0.1rem; flex: 1; min-width: 0; }
  .profile-name { font-size: var(--fs-base); font-weight: 700; display: inline-flex; gap: 0.3em; align-items: baseline; }
  .name-first { color: #60a5fa; }
  .profile-header:hover .name-first { color: #93c5fd; }
  .name-last { color: #94a3b8; }
  .profile-role { font-size: var(--fs-sm); color: #93c5fd; line-height: 1.3; }
  .profile-role.role-collapsed { color: #94a3b8; }
  .card-chevron { font-size: var(--fs-xs); color: #334155; flex-shrink: 0; padding-top: 0.15rem; }
  .profile-field { display: flex; flex-direction: column; gap: 0.1rem; }
  .profile-field-label { font-size: var(--fs-xs); font-weight: 700; text-transform: uppercase; letter-spacing: 0.07em; color: #475569; }
  .profile-field-value { font-size: var(--fs-sm); color: #94a3b8; line-height: 1.4; }
  .rapport-section { display: flex; flex-direction: column; gap: 0.3rem; padding: 0.35rem 0.5rem; background: #060e1a; border-left: 2px solid #60a5fa; border-radius: 0.25rem; }
  .rapport-label { font-size: var(--fs-xs); font-weight: 700; text-transform: uppercase; letter-spacing: 0.07em; color: #475569; }
  .rapport-tips-list { display: flex; flex-direction: column; gap: 0.25rem; }
  .rapport-tip { display: flex; flex-direction: column; gap: 0.05rem; }
  .rapport-kw { font-size: var(--fs-xs); font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em; color: #93c5fd; }
  .rapport-text { font-size: var(--fs-sm); color: #94a3b8; line-height: 1.4; }
  .rapport-empty { font-size: var(--fs-xs); color: #334155; font-style: italic; }
  .profile-loading { font-size: var(--fs-xs); color: #475569; font-style: italic; }
  .profile-retry-btn { background: transparent; border: 1px solid #1e3a5f; color: #475569; font-size: var(--fs-xs); padding: 0.15rem 0.5rem; border-radius: 0.25rem; cursor: pointer; align-self: flex-start; }
  .profile-retry-btn:hover { border-color: #3b82f6; color: #60a5fa; }
  .profiles-empty { display: flex; flex-direction: column; gap: 0.3rem; padding: 0.1rem 0; }
  .profiles-load-btn {
    background: #081428; border: 1px solid #1e3a5f; color: #7dd3fc;
    font-size: var(--fs-xs); padding: 0.2rem 0.5rem; border-radius: 0.25rem;
    cursor: pointer; transition: all 0.12s; align-self: flex-start;
  }
  .profiles-load-btn:hover:not(:disabled) { border-color: #38bdf8; color: #e0f2fe; background: #0c2240; }
  .profiles-load-btn:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
