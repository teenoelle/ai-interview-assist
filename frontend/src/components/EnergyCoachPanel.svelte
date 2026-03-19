<script lang="ts">
  const { wpm, status, tip, energySignal } = $props<{
    wpm: number;
    status: 'good' | 'fast' | 'slow' | 'idle';
    tip: string;
    energySignal: string | null;
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
</script>

<div class="energy-panel">
  <div class="pace-row">
    <span class="pace-label">Pace</span>
    {#if status !== 'idle'}
      <span class="pace-wpm" style="color: {color}">{wpm} wpm</span>
      {#if tip && status !== 'good'}
        <span class="pace-tip" style="color: {color}">{tip}</span>
      {/if}
    {:else}
      <span class="pace-idle">—</span>
    {/if}
  </div>

  <!-- Pace bar: gradient track + moving marker -->
  <div class="pace-bar-wrap">
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
  </div>

  {#if energySignal}
    <div class="energy-signal">{energySignal}</div>
  {/if}
</div>

<style>
  .energy-panel { display: flex; flex-direction: column; gap: 0.3rem; padding: 0.25rem 0; }
  .pace-row { display: flex; align-items: center; gap: 0.5rem; flex-wrap: wrap; }
  .pace-label { font-size: var(--fs-xs); font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em; color: #475569; min-width: 2.5rem; }
  .pace-wpm { font-size: var(--fs-sm); font-weight: 700; font-variant-numeric: tabular-nums; }
  .pace-tip { font-size: var(--fs-sm); font-style: italic; }
  .pace-idle { font-size: var(--fs-sm); color: #1e293b; }

  .pace-bar-wrap { display: flex; flex-direction: column; gap: 0.15rem; }

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

  .energy-signal {
    font-size: var(--fs-sm); color: #f59e0b; font-style: italic;
    padding: 0.2rem 0.4rem; background: #1a0f00; border-left: 2px solid #92400e;
    border-radius: 0.2rem;
  }
</style>
