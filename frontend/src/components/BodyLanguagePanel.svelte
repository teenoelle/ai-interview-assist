<script lang="ts">
  const {
    speakerMode = 'idle',
    presenceIssues = [],
  } = $props<{
    speakerMode?: 'listening' | 'answering' | 'idle';
    presenceIssues?: string[];
  }>();
</script>

<div class="bl-panel">
  <!-- Mode badge -->
  {#if speakerMode !== 'idle'}
    <div class="mode-badge" class:mode-listening={speakerMode === 'listening'} class:mode-answering={speakerMode === 'answering'}>
      {speakerMode === 'listening' ? '👂 Listening mode' : '🎤 Answering mode'}
    </div>
  {/if}

  <!-- Webcam presence issues -->
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
    border-top: 1px solid #1e293b;
    padding-top: 0.5rem;
  }

  .mode-badge {
    font-size: var(--fs-xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    padding: 0.15rem 0.5rem;
    border-radius: 0.25rem;
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    align-self: flex-start;
  }
  .mode-listening { background: #0d1a2b; color: #60a5fa; border: 1px solid #1e3a5f; }
  .mode-answering { background: #0f1a0f; color: #4ade80; border: 1px solid #14532d; }

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
