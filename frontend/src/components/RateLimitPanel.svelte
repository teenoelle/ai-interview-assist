<script lang="ts">
  interface RateLimitEntry {
    remaining: number;
    limit: number;
    history: Array<{ r: number; t: number }>;
  }

  const { rateLimits, callCounts = {} } = $props<{
    rateLimits: Record<string, RateLimitEntry>;
    callCounts?: Record<string, number>;
  }>();

  function pct(entry: RateLimitEntry) {
    return entry.limit > 0 ? (entry.remaining / entry.limit) * 100 : 0;
  }

  function barColor(p: number) {
    if (p > 50) return '#22c55e';
    if (p > 20) return '#f59e0b';
    return '#ef4444';
  }

  function computeRate(history: Array<{ r: number; t: number }>): number | null {
    if (history.length < 2) return null;
    const oldest = history[0];
    const newest = history[history.length - 1];
    const used = oldest.r - newest.r;
    const ms = newest.t - oldest.t;
    if (used <= 0 || ms <= 0) return null;
    return (used / ms) * 60_000;
  }

  function timeLeft(remaining: number, ratePerMin: number | null): string {
    if (!ratePerMin || ratePerMin <= 0) return '—';
    const mins = remaining / ratePerMin;
    if (mins > 120) return `~${Math.round(mins / 60)}h left`;
    if (mins > 1) return `~${Math.round(mins)}m left`;
    return '<1m left';
  }

  const entries = $derived(Object.entries(rateLimits));
</script>

<div class="rate-panel">
  <div class="panel-inner">
    {#if entries.length === 0}
      <p class="empty">API usage will appear here once the interview starts.</p>
    {:else}
      {#each entries as [provider, entry]}
        {@const p = pct(entry)}
        {@const rate = computeRate(entry.history)}
        <div class="provider-row">
          <div class="provider-header">
            <span class="provider-name">{provider}</span>
            <span class="counts">{entry.remaining.toLocaleString()} / {entry.limit.toLocaleString()}</span>
          </div>

          <div class="bar-track">
            <div class="bar-fill" style="width: {p}%; background: {barColor(p)}"></div>
          </div>

          <div class="provider-meta">
            <span class="pct" style="color: {barColor(p)}">{p.toFixed(1)}% remaining</span>
            <span class="rate">
              {#if rate !== null}
                {rate.toFixed(1)} req/min · {timeLeft(entry.remaining, rate)}
              {:else}
                usage rate: tracking...
              {/if}
            </span>
          </div>
          {#if callCounts[provider] != null}
            <div class="call-count">{callCounts[provider]} call{callCounts[provider] !== 1 ? 's' : ''} this session</div>
          {/if}
        </div>
      {/each}
      {#each Object.entries(callCounts).filter(([p]) => !rateLimits[p]) as [provider, count]}
        <div class="provider-row provider-row-counts-only">
          <div class="provider-header">
            <span class="provider-name">{provider}</span>
            <span class="counts">{count} call{count !== 1 ? 's' : ''}</span>
          </div>
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .rate-panel {
    height: 100%;
    overflow-y: auto;
  }
  .panel-inner {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
  .empty {
    color: #475569;
    font-style: italic;
    font-size: var(--fs-base);
    padding: 1rem 0;
  }
  .provider-row {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    padding: 0.75rem;
    background: #0f1e33;
    border-radius: 0.5rem;
    border: 1px solid #1e3a5f;
  }
  .provider-header {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
  }
  .provider-name {
    font-weight: 700;
    font-size: var(--fs-base);
    color: #e2e8f0;
  }
  .counts {
    font-size: var(--fs-base);
    color: #94a3b8;
    font-variant-numeric: tabular-nums;
  }
  .bar-track {
    height: 6px;
    background: #1e293b;
    border-radius: 9999px;
    overflow: hidden;
  }
  .bar-fill {
    height: 100%;
    border-radius: 9999px;
    transition: width 0.4s ease, background 0.4s ease;
  }
  .provider-meta {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    flex-wrap: wrap;
    gap: 0.25rem;
  }
  .pct {
    font-size: var(--fs-sm);
    font-weight: 600;
  }
  .rate {
    font-size: var(--fs-sm);
    color: #64748b;
    font-variant-numeric: tabular-nums;
  }
  .call-count { font-size: var(--fs-xs); color: #475569; }
  .provider-row-counts-only { border-color: #1e293b; opacity: 0.7; }
</style>
