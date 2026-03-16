<script lang="ts">
  const { tactics, onClose } = $props<{
    tactics: { deflect: string; anchor: string; counter_range: string; never_say: string } | null;
    onClose: () => void;
  }>();
</script>

{#if tactics}
  <div class="salary-panel">
    <div class="salary-header">
      <span class="salary-title">💰 Salary Negotiation</span>
      <button class="close-btn" onclick={onClose}>✕</button>
    </div>
    <div class="tactics">
      <div class="tactic">
        <span class="tactic-label">Deflect now</span>
        <span class="tactic-text">"{tactics.deflect}"</span>
      </div>
      <div class="tactic">
        <span class="tactic-label">If pressed — anchor</span>
        <span class="tactic-text">"{tactics.anchor}"</span>
      </div>
      <div class="tactic">
        <span class="tactic-label">Counter-offer</span>
        <span class="tactic-text">{tactics.counter_range}</span>
      </div>
      <div class="tactic never">
        <span class="tactic-label">Never say</span>
        <span class="tactic-text">{tactics.never_say}</span>
      </div>
    </div>
  </div>
{/if}

<style>
  .salary-panel {
    background: #0d1500; border: 1px solid #3d5c00; border-radius: 0.5rem;
    padding: 0.6rem 0.75rem; display: flex; flex-direction: column; gap: 0.5rem;
    animation: slidein 0.2s ease-out;
  }
  @keyframes slidein { from { opacity: 0; transform: translateY(-6px); } to { opacity: 1; } }
  .salary-header { display: flex; align-items: center; justify-content: space-between; }
  .salary-title { font-size: var(--fs-sm); font-weight: 700; color: #a3e635; }
  .close-btn { background: none; border: none; color: #334155; cursor: pointer; font-size: var(--fs-base); padding: 0; }
  .close-btn:hover { color: #64748b; }
  .tactics { display: flex; flex-direction: column; gap: 0.4rem; }
  .tactic { display: flex; flex-direction: column; gap: 0.1rem; }
  .tactic-label { font-size: var(--fs-xs); font-weight: 700; text-transform: uppercase; letter-spacing: 0.07em; color: #65a30d; }
  .tactic-text { font-size: var(--fs-sm); color: #d9f99d; line-height: 1.4; font-style: italic; }
  .tactic.never .tactic-label { color: #ef4444; }
  .tactic.never .tactic-text { color: #fca5a5; font-style: normal; }
</style>
