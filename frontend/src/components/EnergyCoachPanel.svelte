<script lang="ts">
  const { wpm, status, tip, energySignal } = $props<{
    wpm: number;
    status: 'good' | 'fast' | 'slow' | 'idle';
    tip: string;
    energySignal: string | null;
  }>();

  const statusColor = $derived(
    status === 'good' ? '#22c55e' :
    status === 'fast' ? '#f59e0b' :
    status === 'slow' ? '#60a5fa' : '#334155'
  );
</script>

<div class="energy-panel">
  <div class="pace-row">
    <span class="pace-label">Pace</span>
    {#if status !== 'idle'}
      <span class="pace-wpm" style="color: {statusColor}">{wpm} wpm</span>
      {#if tip && status !== 'good'}
        <span class="pace-tip" style="color: {statusColor}">{tip}</span>
      {/if}
    {:else}
      <span class="pace-idle">—</span>
    {/if}
  </div>
  {#if energySignal}
    <div class="energy-signal">{energySignal}</div>
  {/if}
</div>

<style>
  .energy-panel { display: flex; flex-direction: column; gap: 0.2rem; padding: 0.25rem 0; }
  .pace-row { display: flex; align-items: center; gap: 0.5rem; flex-wrap: wrap; }
  .pace-label { font-size: 0.62rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em; color: #475569; min-width: 2.5rem; }
  .pace-wpm { font-size: 0.75rem; font-weight: 700; font-variant-numeric: tabular-nums; }
  .pace-tip { font-size: 0.68rem; font-style: italic; }
  .pace-idle { font-size: 0.75rem; color: #1e293b; }
  .energy-signal {
    font-size: 0.7rem; color: #f59e0b; font-style: italic;
    padding: 0.2rem 0.4rem; background: #1a0f00; border-left: 2px solid #92400e;
    border-radius: 0.2rem;
  }
</style>
