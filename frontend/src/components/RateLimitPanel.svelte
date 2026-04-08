<script lang="ts">
  interface RateLimitEntry {
    remaining: number;
    limit: number;
    history: Array<{ r: number; t: number }>;
  }

  interface ProviderEntry { name: string; local: boolean; }

  const { rateLimits, callCounts = {}, providerStatus = {} } = $props<{
    rateLimits: Record<string, RateLimitEntry>;
    callCounts?: Record<string, number>;
    providerStatus?: Record<string, ProviderEntry>;
  }>();

  const SERVICE_LABELS: Record<string, string> = {
    transcription: 'Transcription',
    suggestions: 'Suggestions',
    sentiment: 'Sentiment',
  };

  const serviceEntries = $derived(
    Object.entries(providerStatus).map(([svc, p]) => ({ svc, label: SERVICE_LABELS[svc] ?? svc, ...p }))
  );

  function pct(entry: RateLimitEntry) {
    return entry.limit > 0 ? (entry.remaining / entry.limit) * 100 : 0;
  }

  function barColor(p: number) {
    if (p > 50) return '#22c55e';
    if (p > 20) return '#f59e0b';
    return '#ef4444';
  }

  function fillColor(provider: string, p: number): string {
    if (provider === 'Claude' || provider === 'Anthropic') return '#475569';
    return barColor(p);
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

  const PURPOSE: Record<string, string> = {
    'Whisper (local)':    'transcription · local',
    'Groq Whisper':       'transcription',
    'Groq Whisper #2':    'transcription',
    'Gemini Transcription': 'transcription',
    'Claude':             'suggestions · sentiment',
    'Groq':               'suggestions · setup',
    'Groq #2':            'suggestions · setup',
    'Gemini':             'suggestions · setup',
    'Ollama':             'suggestions · local',
    'Gemini Vision':      'sentiment',
    'Anthropic':          'suggestions · sentiment',
  };

  function purpose(name: string): string {
    if (PURPOSE[name]) return PURPOSE[name];
    if (name.startsWith('Ollama')) return 'suggestions · local';
    if (name.includes('Whisper')) return 'transcription';
    return '';
  }

  function minsLeft(remaining: number, rate: number | null): number {
    if (!rate || rate <= 0) return Infinity;
    return remaining / rate;
  }

  function isNearLimit(entry: RateLimitEntry): boolean {
    const p = pct(entry);
    if (p < 20) return true;
    const rate = computeRate(entry.history);
    const mins = minsLeft(entry.remaining, rate);
    return mins < 5;
  }
</script>

<div class="rate-panel">
  <div class="panel-inner">
    {#if serviceEntries.length > 0}
      <div class="service-summary">
        {#each serviceEntries as s}
          <div class="service-row">
            <span class="service-label">{s.label}</span>
            <span class="service-provider" class:service-local={s.local} title={s.local ? 'Local' : 'API'}>{s.name}{s.local ? ' ·local' : ''}</span>
          </div>
        {/each}
      </div>
    {/if}
    {#if entries.length === 0}
      <p class="empty">API usage will appear here once the interview starts.</p>
    {:else}
      {#each entries as [provider, entry]}
        {@const p = pct(entry)}
        {@const rate = computeRate(entry.history)}
        {@const warn = isNearLimit(entry)}
        <div class="provider-row" class:provider-warn={warn}>
          <div class="provider-header">
            <div class="provider-name-block">
              <span class="provider-name">{provider}</span>
              {#if purpose(provider)}<span class="provider-purpose">{purpose(provider)}</span>{/if}
            </div>
            <span class="counts">{entry.remaining.toLocaleString()} / {entry.limit.toLocaleString()}</span>
          </div>

          <div class="bar-track">
            <div class="bar-fill" style="width: {p}%; background: {fillColor(provider, p)}"></div>
          </div>

          <div class="provider-meta">
            <span class="pct" style="color: {fillColor(provider, p)}">{p.toFixed(1)}% remaining</span>
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
          {#if warn}
            <div class="warn-badge">⚠ Rate limit low — responses may slow down</div>
          {/if}
        </div>
      {/each}
      {#each Object.entries(callCounts).filter(([p]) => !rateLimits[p]) as [provider, count]}
        <div class="provider-row provider-row-counts-only">
          <div class="provider-header">
            <div class="provider-name-block">
              <span class="provider-name">{provider}</span>
              {#if purpose(provider)}<span class="provider-purpose">{purpose(provider)}</span>{/if}
            </div>
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
  .service-summary {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    padding: 0.5rem 0.75rem;
    background: #0c1929;
    border-radius: 0.4rem;
    border: 1px solid #1e3a5f;
  }
  .service-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.5rem;
  }
  .service-label {
    font-size: var(--fs-xs);
    color: #64748b;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .service-provider {
    font-size: var(--fs-xs);
    font-weight: 600;
    color: #94a3b8;
  }
  .service-local {
    color: #4ade80;
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
  .provider-name-block { display: flex; flex-direction: column; gap: 0.05rem; }
  .provider-name {
    font-weight: 500;
    font-size: var(--fs-sm);
    color: #94a3b8;
  }
  .provider-purpose { font-size: var(--fs-xs); color: #475569; }
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
  .provider-warn { border-color: #92400e; }
  .warn-badge { font-size: var(--fs-xs); color: #f59e0b; font-weight: 600; letter-spacing: 0.02em; }
</style>
