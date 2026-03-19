<script lang="ts">
  const { title, hint, actionLabel, onAction } = $props<{
    title: string;
    hint?: string;
    actionLabel?: string;
    onAction?: () => void;
  }>();
</script>

<div class="panel-header">
  <div class="header-left">
    <h3>{title}</h3>
    {#if hint}<span class="hint">{hint}</span>{/if}
  </div>
  {#if actionLabel && onAction}
    <button class="action-btn" onclick={onAction}>{actionLabel}</button>
  {/if}
</div>

<style>
  .panel-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    margin-bottom: 0.75rem;
    gap: 0.5rem;
  }
  .header-left { display: flex; flex-direction: column; gap: 0.15rem; }
  h3 {
    font-size: var(--fs-base);
    font-weight: 600;
    color: var(--clr-text-secondary, #94a3b8);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin: 0;
  }
  .hint {
    font-size: var(--fs-xs);
    color: var(--clr-text-muted, #475569);
    font-style: italic;
  }
  .action-btn {
    padding: 0.15rem 0.6rem;
    background: transparent;
    border: 1px solid var(--border-muted, #334155);
    border-radius: var(--radius-sm, 0.25rem);
    color: var(--clr-text-muted, #64748b);
    font-size: var(--fs-sm);
    cursor: pointer;
  }
  .action-btn:hover {
    border-color: var(--clr-text-secondary, #94a3b8);
    color: var(--clr-text-secondary, #94a3b8);
  }
</style>
