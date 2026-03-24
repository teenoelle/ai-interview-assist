<script lang="ts">
  const { questions, capturing = false } = $props<{
    questions: string[];
    capturing?: boolean;
  }>();

  let open = $state(!capturing);
  let sending = $state(false);
  let hints = $state<Record<number, string>>({});
  let loading = $state<Set<number>>(new Set());
  let tooltip = $state<{ idx: number; x: number; y: number } | null>(null);
  let tooltipEl: HTMLDivElement | undefined = $state();

  $effect(() => {
    if (capturing) open = false;
    else open = true;
  });

  async function fetchHint(idx: number) {
    if (hints[idx] !== undefined || loading.has(idx)) return;
    loading = new Set([...loading, idx]);
    try {
      const resp = await fetch('/api/practice-question', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ question: questions[idx] }),
      });
      if (resp.ok) {
        const data = await resp.json();
        hints = { ...hints, [idx]: data.suggestion ?? '' };
      }
    } catch { /* ignore */ }
    const next = new Set(loading);
    next.delete(idx);
    loading = next;
  }

  async function send(q: string) {
    if (sending) return;
    sending = true;
    tooltip = null;
    try {
      await fetch('/api/simulate-question', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ question: q }),
      });
    } finally {
      sending = false;
    }
  }

  function onMouseEnter(idx: number, e: MouseEvent) {
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    tooltip = { idx, x: rect.left, y: rect.bottom + 6 };
    fetchHint(idx);
  }

  function onMouseLeave() {
    tooltip = null;
  }

  function shortQ(q: string, max = 48): string {
    return q.length > max ? q.slice(0, max) + '…' : q;
  }

  function hintPreview(hint: string, max = 220): string {
    return hint.length > max ? hint.slice(0, max) + '…' : hint;
  }
</script>

{#if questions.length > 0}
<div class="pqb">
  <div class="pqb-header">
    <button class="pqb-toggle" onclick={() => open = !open}>
      {open ? '▾' : '▸'} Practice Questions
      <span class="pqb-count">{questions.length}</span>
    </button>
  </div>

  {#if open}
    <div class="pqb-chips">
      {#each questions as q, i}
        <button
          class="pqb-chip"
          class:sending
          onclick={() => send(q)}
          onmouseenter={(e) => onMouseEnter(i, e)}
          onmouseleave={onMouseLeave}
          disabled={sending}
          title={q}
        >
          {shortQ(q)}
        </button>
      {/each}
    </div>
  {/if}
</div>
{/if}

{#if tooltip !== null}
  {@const idx = tooltip.idx}
  <div
    class="pqb-tooltip"
    bind:this={tooltipEl}
    style="left: {tooltip.x}px; top: {tooltip.y}px"
    onmouseenter={() => { /* keep open */ }}
  >
    <div class="pqb-tooltip-q">{questions[idx]}</div>
    {#if loading.has(idx)}
      <div class="pqb-tooltip-loading">Loading hint…</div>
    {:else if hints[idx]}
      <div class="pqb-tooltip-hint">{hintPreview(hints[idx])}</div>
    {:else}
      <div class="pqb-tooltip-loading">Hover to load hint</div>
    {/if}
    <div class="pqb-tooltip-cta">Click chip to send as test question →</div>
  </div>
{/if}

<style>
  .pqb {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    padding: 0.4rem 0.5rem;
    background: #060e1c;
    border-radius: 0.5rem;
    border: 1px solid #1a2d4a;
    flex-shrink: 0;
  }

  .pqb-header {
    display: flex;
    align-items: center;
  }

  .pqb-toggle {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    background: none;
    border: none;
    color: #475569;
    font-size: var(--fs-xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    cursor: pointer;
    padding: 0;
    transition: color 0.12s;
  }
  .pqb-toggle:hover { color: #64748b; }

  .pqb-count {
    background: #0d1f35;
    border: 1px solid #1e3a5f;
    color: #475569;
    font-size: var(--fs-xs);
    font-weight: 700;
    border-radius: 0.9em;
    padding: 0 0.4em;
    line-height: 1.5;
  }

  .pqb-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
  }

  .pqb-chip {
    background: #081428;
    border: 1px solid #1e3a5f;
    color: #7dd3fc;
    font-size: var(--fs-xs);
    padding: 0.15rem 0.45rem;
    border-radius: 0.25rem;
    cursor: pointer;
    white-space: nowrap;
    transition: all 0.12s;
    line-height: 1.4;
    max-width: 22ch;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .pqb-chip:hover:not(:disabled) {
    border-color: #38bdf8;
    color: #e0f2fe;
    background: #0c2240;
  }
  .pqb-chip:disabled { opacity: 0.4; cursor: not-allowed; }

  /* tooltip */
  .pqb-tooltip {
    position: fixed;
    z-index: 9999;
    max-width: 300px;
    background: #040d1a;
    border: 1px solid #1e3a5f;
    border-radius: 0.4rem;
    padding: 0.5rem 0.65rem;
    box-shadow: 0 4px 20px rgba(0,0,0,0.6);
    pointer-events: none;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .pqb-tooltip-q {
    font-size: var(--fs-xs);
    font-weight: 700;
    color: #7dd3fc;
    line-height: 1.35;
  }

  .pqb-tooltip-hint {
    font-size: var(--fs-xs);
    color: #94a3b8;
    line-height: 1.45;
    white-space: pre-wrap;
  }

  .pqb-tooltip-loading {
    font-size: var(--fs-xs);
    color: #334155;
    font-style: italic;
  }

  .pqb-tooltip-cta {
    font-size: var(--fs-xs);
    color: #1e3a5f;
    border-top: 1px solid #0f1e33;
    padding-top: 0.25rem;
    margin-top: 0.1rem;
  }
</style>
