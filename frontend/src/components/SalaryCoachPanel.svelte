<script lang="ts">
  const { tactics, onClose } = $props<{
    tactics: { early_round: string; reveal: string; direct_ask: string; total_package: string; counter: string } | null;
    onClose: () => void;
  }>();

  const cards = $derived(tactics ? [
    { id: 'early',   label: 'Early Round',   sub: 'Too soon to anchor',          text: tactics.early_round,   style: 'muted'     },
    { id: 'reveal',  label: 'Reveal',        sub: 'Invite their range first',    text: tactics.reveal,        style: 'negotiate' },
    { id: 'direct',  label: 'Direct Ask',    sub: 'They still want your number', text: tactics.direct_ask,    style: 'confident' },
    { id: 'package', label: 'Total Package', sub: 'Redirect to full comp',       text: tactics.total_package, style: 'neutral'   },
    { id: 'counter', label: 'Counter',       sub: 'Offer came in below range',   text: tactics.counter,       style: 'careful'   },
  ] : []);

  // Early Round open by default
  let expanded = $state(new Set<string>(['early']));

  function toggle(id: string) {
    const next = new Set(expanded);
    if (next.has(id)) { next.delete(id); } else { next.add(id); }
    expanded = next;
  }
</script>

{#if tactics}
  <div class="salary-panel">
    <div class="salary-header">
      <span class="salary-title">Salary Alignment</span>
      <button class="close-btn" onclick={onClose}>✕</button>
    </div>
    <div class="cards">
      {#each cards as card (card.id)}
        {@const open = expanded.has(card.id)}
        <div class="card card-{card.style}" class:card-open={open}>
          <button class="card-toggle" onclick={() => toggle(card.id)}>
            <div class="card-top">
              <span class="card-label">{card.label}</span>
              {#if !open}<span class="card-sub">{card.sub}</span>{/if}
            </div>
            <span class="card-chevron">{open ? '▾' : '▸'}</span>
          </button>
          {#if open}
            <p class="card-text">"{card.text}"</p>
          {/if}
        </div>
      {/each}
    </div>
  </div>
{/if}

<style>
  .salary-panel {
    background: #080d18; border: 1px solid #1a2540; border-radius: 0.5rem;
    padding: 0.6rem 0.75rem; display: flex; flex-direction: column; gap: 0.5rem;
    animation: slidein 0.2s ease-out;
  }
  @keyframes slidein { from { opacity: 0; transform: translateY(-6px); } to { opacity: 1; } }

  .salary-header { display: flex; align-items: center; justify-content: space-between; }
  .salary-title { font-size: var(--fs-sm); font-weight: 700; color: #475569; text-transform: uppercase; letter-spacing: 0.06em; }
  .close-btn { background: none; border: none; color: #334155; cursor: pointer; font-size: var(--fs-base); padding: 0; }
  .close-btn:hover { color: #64748b; }

  .cards { display: flex; flex-direction: column; gap: 0.25rem; }

  .card {
    border-radius: 0.35rem;
    border-left: 3px solid transparent;
    overflow: hidden;
  }

  .card-toggle {
    display: flex; align-items: center; justify-content: space-between;
    width: 100%; background: none; border: none; cursor: pointer;
    padding: 0.4rem 0.55rem; text-align: left; gap: 0.5rem;
  }
  .card-toggle:hover { filter: brightness(1.15); }

  .card-top { display: flex; align-items: baseline; gap: 0.45rem; flex: 1; min-width: 0; }

  .card-label {
    font-size: var(--fs-xs); font-weight: 700; text-transform: uppercase;
    letter-spacing: 0.06em; flex-shrink: 0;
  }
  .card-sub {
    font-size: var(--fs-xs); color: #334155; 
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .card-chevron {
    font-size: var(--fs-xs); color: #334155; flex-shrink: 0;
  }

  .card-text {
    font-size: var(--fs-sm); line-height: 1.45; margin: 0;
     padding: 0 0.55rem 0.45rem;
  }

  /* Muted — early round */
  .card-muted { background: #0a0e1a; border-left-color: #334155; }
  .card-muted .card-label { color: #475569; }
  .card-muted .card-text  { color: #94a3b8; }

  /* Negotiate — reveal */
  .card-negotiate { background: #0d1020; border-left-color: #4f46e5; }
  .card-negotiate .card-label { color: #a5b4fc; }
  .card-negotiate .card-text  { color: #c7d2fe; }

  /* Confident — direct ask */
  .card-confident { background: #0a1a0d; border-left-color: #166534; }
  .card-confident .card-label { color: #4ade80; }
  .card-confident .card-text  { color: #bbf7d0; }

  /* Neutral — total package */
  .card-neutral { background: #0d0f1a; border-left-color: #1e3a5f; }
  .card-neutral .card-label { color: #60a5fa; }
  .card-neutral .card-text  { color: #bfdbfe; }

  /* Careful — counter */
  .card-careful { background: #150e00; border-left-color: #78350f; }
  .card-careful .card-label { color: #f59e0b; }
  .card-careful .card-text  { color: #fde68a; }
</style>
