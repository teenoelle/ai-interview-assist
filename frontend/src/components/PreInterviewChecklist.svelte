<script lang="ts">
  const { onStart, onSkip } = $props<{ onStart: () => void; onSkip: () => void }>();

  const items = [
    { id: 'camera',  label: 'Camera framing',      detail: 'Head and shoulders visible, centred in frame' },
    { id: 'light',   label: 'Lighting',             detail: 'Face well-lit — light source in front, not behind you' },
    { id: 'bg',      label: 'Background',           detail: 'Clean, professional or virtual background' },
    { id: 'audio',   label: 'Microphone',           detail: 'Tested and working — headset preferred' },
    { id: 'tabs',    label: 'Distractions closed',  detail: 'Close social media, notifications silenced' },
    { id: 'water',   label: 'Water nearby',         detail: 'Prevents dry mouth during long answers' },
    { id: 'notes',   label: 'Notes ready',          detail: 'CV and key talking points visible for reference' },
    { id: 'phone',   label: 'Phone on silent',      detail: 'Including vibrate — it carries through the mic' },
  ];

  let checked = $state<Record<string, boolean>>({});
  const allChecked = $derived(items.every(i => checked[i.id]));

  function toggle(id: string) { checked = { ...checked, [id]: !checked[id] }; }
  function checkAll() { const all: Record<string, boolean> = {}; items.forEach(i => all[i.id] = true); checked = all; }
</script>

<div class="backdrop">
  <div class="modal">
    <div class="modal-header">
      <h2>Pre-Interview Checklist</h2>
      <p class="subtitle">Quick check before you start — takes 30 seconds</p>
    </div>

    <div class="items">
      {#each items as item}
        <button class="item" class:done={checked[item.id]} onclick={() => toggle(item.id)}>
          <span class="check">{checked[item.id] ? '✓' : '○'}</span>
          <div class="item-text">
            <span class="item-label">{item.label}</span>
            <span class="item-detail">{item.detail}</span>
          </div>
        </button>
      {/each}
    </div>

    <div class="actions">
      {#if !allChecked}
        <button class="check-all-btn" onclick={checkAll}>Mark all done</button>
      {/if}
      <button class="start-btn" onclick={onStart} disabled={!allChecked}>
        {allChecked ? 'Start Interview' : `${items.filter(i => checked[i.id]).length}/${items.length} checked`}
      </button>
      <button class="skip-btn" onclick={onSkip}>Skip checklist</button>
    </div>
  </div>
</div>

<style>
  .backdrop { position: fixed; inset: 0; background: rgba(0,0,0,0.85); display: flex; align-items: center; justify-content: center; z-index: 200; }
  .modal { background: #0f172a; border: 1px solid #1e293b; border-radius: 0.75rem; width: min(480px, 95vw); max-height: 90vh; display: flex; flex-direction: column; }
  .modal-header { padding: 1.25rem 1.5rem 0.75rem; border-bottom: 1px solid #1e293b; }
  h2 { font-size: var(--fs-lg); font-weight: 700; color: #f1f5f9; margin: 0 0 0.25rem; }
  .subtitle { font-size: var(--fs-base); color: #475569; margin: 0; }
  .items { overflow-y: auto; padding: 0.75rem; display: flex; flex-direction: column; gap: 0.3rem; }
  .item { display: flex; align-items: flex-start; gap: 0.75rem; padding: 0.6rem 0.75rem; background: #080d18; border: 1px solid #1e293b; border-radius: 0.4rem; cursor: pointer; text-align: left; transition: all 0.15s; }
  .item:hover { border-color: #334155; background: #0d1525; }
  .item.done { background: #0a1f10; border-color: #14532d; }
  .check { font-size: 1rem; color: #334155; flex-shrink: 0; width: 1.2rem; text-align: center; margin-top: 0.1rem; }
  .item.done .check { color: #4ade80; }
  .item-text { display: flex; flex-direction: column; gap: 0.1rem; }
  .item-label { font-size: var(--fs-base); font-weight: 600; color: #94a3b8; }
  .item.done .item-label { color: #4ade80; }
  .item-detail { font-size: var(--fs-sm); color: #334155; line-height: 1.3; }
  .actions { padding: 0.75rem 1rem; border-top: 1px solid #1e293b; display: flex; align-items: center; gap: 0.5rem; flex-wrap: wrap; }
  .check-all-btn { padding: 0.35rem 0.75rem; background: transparent; border: 1px solid #334155; border-radius: 0.3rem; color: #64748b; font-size: var(--fs-sm); cursor: pointer; }
  .check-all-btn:hover { border-color: #64748b; color: #94a3b8; }
  .start-btn { flex: 1; padding: 0.5rem 1rem; background: #4ade80; border: none; border-radius: 0.4rem; color: #0a1f10; font-size: var(--fs-base); font-weight: 700; cursor: pointer; transition: all 0.15s; }
  .start-btn:disabled { background: #1e293b; color: #334155; cursor: default; }
  .start-btn:not(:disabled):hover { background: #86efac; }
  .skip-btn { padding: 0.35rem 0.75rem; background: transparent; border: none; color: #334155; font-size: var(--fs-sm); cursor: pointer; }
  .skip-btn:hover { color: #475569; }
</style>
