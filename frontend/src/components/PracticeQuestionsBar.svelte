<script lang="ts">
  import { authFetch } from '../lib/api';

  const { questions, capturing = false, onPredict, loadingPredict = false } = $props<{
    questions: string[];
    capturing?: boolean;
    onPredict?: () => void;
    loadingPredict?: boolean;
  }>();

  let open = $state(false);
  let sending = $state(false);

  $effect(() => {
    if (capturing) open = false;
  });

  async function send(q: string) {
    if (sending) return;
    sending = true;
    try {
      await authFetch('/api/simulate-question', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ question: q }),
      });
    } finally {
      sending = false;
    }
  }

  function shortQ(q: string, max = 48): string {
    return q.length > max ? q.slice(0, max) + '…' : q;
  }
</script>

<div class="pqb">
  <div class="pqb-header">
    <button class="pqb-toggle" onclick={() => open = !open}>
      {open ? '▾' : '▸'} Practice Questions
      {#if questions.length > 0}<span class="pqb-count">{questions.length}</span>{/if}
    </button>
  </div>

  {#if open}
    {#if questions.length === 0}
      <div class="pqb-empty-row">
        {#if onPredict}
          <button class="pqb-predict-btn" onclick={onPredict} disabled={loadingPredict}>
            {loadingPredict ? 'Predicting…' : '⟳ Predict questions'}
          </button>
        {/if}
        <p class="pqb-empty">{loadingPredict ? 'Analyzing your setup…' : 'No predicted questions yet.'}</p>
      </div>
    {:else}
      <div class="pqb-chips">
        {#each questions as q, i}
          <button
            class="pqb-chip"
            class:sending
            onclick={() => send(q)}
            disabled={sending}
          >
            {shortQ(q)}
          </button>
        {/each}
      </div>
    {/if}
  {/if}
</div>


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

  .pqb-empty-row {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }

  .pqb-predict-btn {
    background: #081428;
    border: 1px solid #1e3a5f;
    color: #7dd3fc;
    font-size: var(--fs-xs);
    padding: 0.15rem 0.5rem;
    border-radius: 0.25rem;
    cursor: pointer;
    transition: all 0.12s;
    align-self: flex-start;
  }
  .pqb-predict-btn:hover:not(:disabled) { border-color: #38bdf8; color: #e0f2fe; background: #0c2240; }
  .pqb-predict-btn:disabled { opacity: 0.5; cursor: not-allowed; }

  .pqb-empty {
    font-size: var(--fs-xs);
    color: #334155;
    font-style: italic;
    margin: 0;
    padding: 0.1rem 0;
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
</style>
