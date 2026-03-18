<script lang="ts">
  const { interviewers } = $props<{
    interviewers: Array<{ name: string; role: string; background: string; tenure: string; rapport_tips: string[] }>;
  }>();

  let expanded = $state(true);
  let collapsedCards = $state(new Set<number>());

  function toggleCard(i: number) {
    const s = new Set(collapsedCards);
    s.has(i) ? s.delete(i) : s.add(i);
    collapsedCards = s;
  }

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
        <span class="profiles-names">{interviewers.map(i => i.name).filter(Boolean).join(' · ')}</span>
      </div>
      <span class="profiles-chevron">{expanded ? '▴' : '▾'}</span>
    </button>
    {#if expanded}
  <div class="profiles">
    {#each interviewers as iv, i}
      {@const cardCollapsed = collapsedCards.has(i)}
      <div class="profile-card">
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="profile-header" onclick={() => toggleCard(i)} role="button" tabindex="0"
          onkeydown={(e) => e.key === 'Enter' && toggleCard(i)}>
          <span class="profile-name">{iv.name}</span>
          {#if iv.role}<span class="profile-role">{iv.role}</span>{/if}
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
  .profiles-chevron { font-size: var(--fs-xs); color: #334155; }
  .profiles { display: flex; flex-direction: column; gap: 0.5rem; padding: 0.5rem 0.75rem 0.75rem; border-top: 1px solid #0f1e33; }
  .profile-card { background: #060e1a; border: 1px solid #1a2d4a; border-radius: 0.4rem; padding: 0.6rem 0.75rem; display: flex; flex-direction: column; gap: 0.5rem; }
  .profile-header { display: flex; align-items: baseline; gap: 0.5rem; cursor: pointer; user-select: none; min-width: 0; }
  .profile-role { font-size: var(--fs-sm); color: #93c5fd; flex-shrink: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .profile-header:hover .profile-name { color: #93c5fd; }
  .profile-name { font-size: var(--fs-base); font-weight: 700; color: #60a5fa; transition: color 0.12s; }
  .card-chevron { font-size: var(--fs-xs); color: #334155; margin-left: auto; flex-shrink: 0; }
  .profile-field { display: flex; flex-direction: column; gap: 0.1rem; }
  .profile-field-label { font-size: var(--fs-xs); font-weight: 700; text-transform: uppercase; letter-spacing: 0.07em; color: #475569; }
  .profile-field-value { font-size: var(--fs-sm); color: #94a3b8; line-height: 1.4; }
  .rapport-section { display: flex; flex-direction: column; gap: 0.3rem; padding: 0.35rem 0.5rem; background: #060e1a; border-left: 2px solid #1e40af; border-radius: 0.25rem; }
  .rapport-label { font-size: var(--fs-xs); font-weight: 700; text-transform: uppercase; letter-spacing: 0.07em; color: #475569; }
  .rapport-tips-list { display: flex; flex-direction: column; gap: 0.25rem; }
  .rapport-tip { display: flex; flex-direction: column; gap: 0.05rem; }
  .rapport-kw { font-size: var(--fs-xs); font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em; color: #93c5fd; }
  .rapport-text { font-size: var(--fs-sm); color: #94a3b8; line-height: 1.4; }
</style>
