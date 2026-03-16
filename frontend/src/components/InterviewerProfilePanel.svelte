<script lang="ts">
  const { interviewers } = $props<{
    interviewers: Array<{ name: string; role: string; background: string; tenure: string; rapport_tip: string }>;
  }>();
</script>

{#if interviewers.length > 0}
  <div class="profiles">
    {#each interviewers as iv}
      <div class="profile-card">
        <div class="profile-header">
          <span class="profile-name">{iv.name}</span>
          {#if iv.role}<span class="profile-role">{iv.role}</span>{/if}
        </div>
        {#if iv.background}<p class="profile-bg">{iv.background}</p>{/if}
        {#if iv.tenure}<p class="profile-tenure">Tenure: {iv.tenure}</p>{/if}
        {#if iv.rapport_tip}
          <div class="rapport-tip">
            <span class="rapport-label">Rapport tip</span>
            <span class="rapport-text">{iv.rapport_tip}</span>
          </div>
        {/if}
      </div>
    {/each}
  </div>
{/if}

<style>
  .profiles { display: flex; flex-direction: column; gap: 0.5rem; }
  .profile-card { background: #060e1a; border: 1px solid #1a2d4a; border-radius: 0.4rem; padding: 0.6rem 0.75rem; display: flex; flex-direction: column; gap: 0.3rem; }
  .profile-header { display: flex; align-items: baseline; gap: 0.5rem; flex-wrap: wrap; }
  .profile-name { font-size: var(--fs-base); font-weight: 700; color: #e2e8f0; }
  .profile-role { font-size: var(--fs-sm); color: #60a5fa; }
  .profile-bg { font-size: var(--fs-sm); color: #94a3b8; margin: 0; line-height: 1.4; }
  .profile-tenure { font-size: var(--fs-sm); color: #475569; margin: 0; }
  .rapport-tip { display: flex; flex-direction: column; gap: 0.1rem; padding: 0.35rem 0.5rem; background: #071a0f; border-left: 2px solid #14532d; border-radius: 0.25rem; margin-top: 0.1rem; }
  .rapport-label { font-size: var(--fs-xs); font-weight: 700; text-transform: uppercase; letter-spacing: 0.07em; color: #4ade80; }
  .rapport-text { font-size: var(--fs-sm); color: #94a3b8; line-height: 1.4; }
</style>
