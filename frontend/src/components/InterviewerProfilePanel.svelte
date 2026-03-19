<script lang="ts">
  const { interviewers } = $props<{
    interviewers: Array<{ name: string; role: string; background: string; tenure: string; rapport_tips: string[] }>;
  }>();

  let expanded = $state(false);
  let expandedCards = $state(new Set<number>());

  function toggleCard(i: number) {
    const s = new Set(expandedCards);
    s.has(i) ? s.delete(i) : s.add(i);
    expandedCards = s;
  }

  function firstName(name: string) { return name.trim().split(/\s+/)[0] ?? ''; }
  function lastName(name: string) { const p = name.trim().split(/\s+/); return p.length > 1 ? p.slice(1).join(' ') : ''; }

  function parseTip(tip: string): { keyword: string; text: string } {
    const m = tip.match(/^\[([^\]]+)\]\s*(.*)/s);
    return m ? { keyword: m[1], text: m[2].trim() } : { keyword: '', text: tip };
  }
</script>

{#if interviewers.length > 0}
  <div class="profiles-panel">
    <button class="profiles-toggle" onclick={() => expanded = !expanded}>
      <div class="profiles-toggle-inner">
        <span class="profiles-label-header">Interviewers</span>
        {#if !expanded}<span class="profiles-names-preview">{interviewers.map(iv => firstName(iv.name)).join(' · ')}</span>{/if}
      </div>
      <span class="profiles-chevron">{expanded ? '▴' : '▾'}</span>
    </button>
    {#if expanded}
  <div class="profiles">
    {#each interviewers as iv, i}
      {@const cardCollapsed = !expandedCards.has(i)}
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
          {#if iv.background}
            <div class="profile-field">
              <span class="profile-field-label">Background</span>
              <span class="profile-field-value">{iv.background}</span>
            </div>
          {/if}
          {#if iv.tenure}
            <div class="profile-field">
              <span class="profile-field-label">Tenure</span>
              <span class="profile-field-value">{iv.tenure}</span>
            </div>
          {/if}
          {#if iv.rapport_tips?.length > 0}
            <div class="rapport-section">
              <span class="rapport-label">Rapport Tips</span>
              <div class="rapport-tips-list">
                {#each iv.rapport_tips as tip}
                  {@const p = parseTip(tip)}
                  <div class="rapport-tip">
                    {#if p.keyword}<span class="rapport-kw">{p.keyword}</span>{/if}
                    <span class="rapport-text">{p.text}</span>
                  </div>
                {/each}
              </div>
            </div>
          {/if}
        {/if}
      </div>
    {/each}
  </div>
    {/if}
  </div>
{/if}

<style>
  .profiles-panel { background: #060e1a; border: 1px solid #1a2d4a; border-radius: 0.5rem; overflow: hidden; }
  .profiles-toggle { width: 100%; display: flex; align-items: center; justify-content: space-between; padding: 0.5rem 0.75rem; background: transparent; border: none; cursor: pointer; text-align: left; }
  .profiles-toggle:hover { background: #0a1525; }
  .profiles-toggle-inner { display: flex; flex-direction: column; gap: 0.1rem; text-align: left; }
  .profiles-label-header { font-size: var(--fs-xs); font-weight: 700; text-transform: uppercase; letter-spacing: 0.07em; color: #334155; }
  .profiles-names { font-size: var(--fs-base); font-weight: 700; color: #60a5fa; }
  .profiles-names-preview { font-size: var(--fs-sm); color: #60a5fa; font-weight: 600; }
  .profiles-chevron { font-size: var(--fs-xs); color: #334155; }
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
</style>
