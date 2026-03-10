<script lang="ts">
  import type { FillerCount } from '../lib/filler';

  const { answerMs, youPct, interviewerPct, fillerTotal, fillerCounts, wsStatus, wsAttempt } = $props<{
    answerMs: number;
    youPct: number;
    interviewerPct: number;
    fillerTotal: number;
    fillerCounts: FillerCount[];
    wsStatus: string;
    wsAttempt: number;
  }>();

  function fmtTime(ms: number): string {
    const s = Math.floor(ms / 1000);
    const m = Math.floor(s / 60);
    return `${m}:${String(s % 60).padStart(2, '0')}`;
  }

  const timerColor = $derived(
    answerMs === 0 ? '#475569' :
    answerMs < 90000 ? '#22c55e' :
    answerMs < 150000 ? '#f59e0b' : '#ef4444'
  );

  const ratioColor = $derived(
    youPct === 0 ? '#475569' :
    youPct < 65 ? '#22c55e' : '#f59e0b'
  );

  let showFillers = $state(false);
</script>

<div class="stats-bar">
  <!-- Answer timer -->
  <div class="stat" title="Time since you started your current answer">
    <span class="stat-label">Answer</span>
    <span class="stat-value" style="color: {timerColor}">
      {answerMs > 0 ? fmtTime(answerMs) : '—'}
    </span>
  </div>

  <div class="sep"></div>

  <!-- Talk ratio -->
  <div class="stat" title="Your share of speaking time vs interviewer">
    <span class="stat-label">You / Them</span>
    <span class="stat-value" style="color: {ratioColor}">
      {youPct > 0 ? `${youPct}% / ${interviewerPct}%` : '—'}
    </span>
  </div>

  <div class="sep"></div>

  <!-- Filler words -->
  <div class="stat filler-stat" title="Filler word count from your speech">
    <span class="stat-label">Fillers</span>
    <button
      class="filler-btn"
      class:has-fillers={fillerTotal > 0}
      onclick={() => showFillers = !showFillers}
    >
      {fillerTotal > 0 ? fillerTotal : '—'}
    </button>
    {#if showFillers && fillerCounts.length > 0}
      <div class="filler-popup">
        {#each fillerCounts as f}
          <span class="filler-item">"{f.word}" ×{f.count}</span>
        {/each}
      </div>
    {/if}
  </div>

  <div class="sep"></div>

  <!-- WS status -->
  <div class="stat" title="WebSocket connection status">
    <span class="stat-label">WS</span>
    <span class="stat-value ws-status"
      class:connected={wsStatus === 'connected'}
      class:reconnecting={wsStatus === 'reconnecting'}
    >
      {wsStatus === 'connected' ? '●' : wsStatus === 'reconnecting' ? `↻ #${wsAttempt}` : '○'}
    </span>
  </div>
</div>

<style>
  .stats-bar {
    display: flex;
    align-items: center;
    gap: 0;
    padding: 0.25rem 1rem;
    background: #0a0f1a;
    border-bottom: 1px solid #1e293b;
    flex-shrink: 0;
    flex-wrap: wrap;
  }
  .stat {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.1rem 0.75rem;
    position: relative;
  }
  .stat-label {
    font-size: 0.65rem;
    color: #475569;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    font-weight: 600;
  }
  .stat-value {
    font-size: 0.75rem;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }
  .sep { width: 1px; height: 16px; background: #1e293b; }
  .filler-stat { position: relative; }
  .filler-btn {
    background: none; border: none; cursor: pointer;
    font-size: 0.75rem; font-weight: 700; color: #475569;
    padding: 0;
  }
  .filler-btn.has-fillers { color: #f59e0b; }
  .filler-popup {
    position: absolute; top: 100%; left: 0; z-index: 50;
    background: #1e293b; border: 1px solid #334155;
    border-radius: 0.375rem; padding: 0.5rem;
    display: flex; flex-direction: column; gap: 0.2rem;
    white-space: nowrap; min-width: 120px;
  }
  .filler-item { font-size: 0.72rem; color: #f59e0b; }
  .ws-status { font-size: 0.8rem; }
  .ws-status.connected { color: #22c55e; }
  .ws-status.reconnecting { color: #f59e0b; }
</style>
