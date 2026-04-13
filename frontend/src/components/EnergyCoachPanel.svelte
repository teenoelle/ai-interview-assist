<script lang="ts">
  const { wpm, status, tip, hideLabel = false } = $props<{
    wpm: number;
    status: 'good' | 'fast' | 'slow' | 'idle';
    tip: string;
    hideLabel?: boolean;
  }>();

  const GOOD_MIN = 90;
  const GOOD_MAX = 180;
  const BAR_MAX  = 220;

  function paceColor(status: string): string {
    if (status === 'idle') return '#334155';
    if (status === 'good') return '#22c55e';
    return '#f59e0b';
  }

  const color      = $derived(paceColor(status));
  const markerPct  = $derived(status !== 'idle' ? Math.min(100, (wpm / BAR_MAX) * 100) : null);
  const hasTip     = $derived(!!(tip && status !== 'good' && status !== 'idle'));
  let tipExpanded  = $state(false);
</script>

<div class="energy-panel">
  {#if !hideLabel}
  <div class="pace-row">
    <span class="pace-label">Pace</span>
    {#if status !== 'idle'}
      <span class="pace-wpm" style="color: {color}">{wpm} wpm</span>
    {:else}
      <span class="pace-idle">—</span>
    {/if}
  </div>
  {:else if status !== 'idle'}
  <div class="pace-row pace-row-compact">
    <span class="pace-wpm" style="color: {color}">{wpm} wpm</span>
  </div>
  {/if}

  <!-- Pace bar: gradient track + moving marker; click to reveal tip -->
  <button class="pace-bar-wrap" class:pace-bar-clickable={hasTip} onclick={() => { if (hasTip) tipExpanded = !tipExpanded; }}>
    <div class="pace-track">
      {#if markerPct !== null}
        <div class="pace-marker" style="left: {markerPct}%; border-color: {color}"></div>
      {/if}
    </div>
    <div class="pace-bar-labels">
      <span>slow</span>
      <span>ideal</span>
      <span>fast</span>
    </div>
  </button>
  {#if tipExpanded && hasTip}
    <div class="pace-tip-expanded" style="color: {color}">{tip}</div>
  {/if}

</div>

<style>
  .energy-panel { display: flex; flex-direction: column; gap: 0.3rem; padding: 0.25rem 0; width: 100%; }
  .pace-row { display: flex; align-items: center; gap: 0.5rem; flex-wrap: wrap; }
  .pace-row-compact { justify-content: flex-end; }
  .pace-label { font-size: var(--fs-xs); font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em; color: #475569; min-width: 2.5rem; }
  .pace-wpm { font-size: var(--fs-sm); font-weight: 700; font-variant-numeric: tabular-nums; margin-left: auto; }
  .pace-idle { font-size: var(--fs-sm); color: #1e293b; margin-left: auto; }

  .pace-bar-wrap { display: flex; flex-direction: column; gap: 0.15rem; background: none; border: none; padding: 0; width: 100%; text-align: left; }
  .pace-bar-clickable { cursor: pointer; }
  .pace-tip-expanded { font-size: var(--fs-sm); font-style: italic; padding: 0.2rem 0; line-height: 1.35; }

  .pace-track {
    position: relative;
    height: 5px;
    border-radius: var(--radius-pill, 9999px);
    /* gradient zones: red → amber → green (90/220=41%) → green (180/220=82%) → amber → red */
    background: linear-gradient(to right,
      #f59e0b 0%,
      #22c55e 41%,
      #22c55e 82%,
      #f59e0b 100%
    );
  }

  .pace-marker {
    position: absolute;
    top: 50%;
    transform: translate(-50%, -50%);
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: #fff;
    border: 2px solid;  /* color set inline */
    box-shadow: 0 1px 4px rgba(0,0,0,0.6);
    transition: left 0.5s ease;
    pointer-events: none;
  }

  .pace-bar-labels {
    display: flex;
    justify-content: space-between;
    font-size: var(--fs-xs);
    color: #334155;
  }

</style>
