<script lang="ts">
  const {
    presenceIssues = [],
  } = $props<{
    presenceIssues?: string[];
  }>();
</script>

<div class="bl-panel">
  {#if presenceIssues.length > 0}
    <div class="presence-section">
      <span class="presence-label">📷 Your camera</span>
      {#each presenceIssues as issue}
        <div class="presence-issue">{issue}</div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .bl-panel {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .presence-section {
    background: #0d1a0d;
    border: 1px solid #1a3a1a;
    border-left: 3px solid #15803d;
    border-radius: 0.4rem;
    padding: 0.35rem 0.6rem;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }
  .presence-label {
    font-size: var(--fs-xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: #4ade80;
  }
  .presence-issue {
    font-size: var(--fs-sm);
    color: #fca5a5;
    line-height: 1.3;
  }
</style>
