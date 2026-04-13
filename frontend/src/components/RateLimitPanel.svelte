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

  const PURPOSE: Record<string, string> = {
    'Whisper (local)':      'transcription · local',
    'Groq Whisper':         'transcription',
    'Groq Whisper #2':      'transcription',
    'Gemini Transcription': 'transcription',
    'Deepgram':             'transcription',
    'Bonsai':               'suggestions · local',
    'Claude CLI':           'suggestions · local',
    'Claude API':           'suggestions · sentiment',
    'Groq':                 'suggestions',
    'Groq #2':              'suggestions',
    'Cerebras':             'suggestions',
    'OpenRouter':           'suggestions',
    'Qwen':                 'suggestions',
    'Mistral':              'suggestions',
    'DeepSeek':             'suggestions',
    'Ollama':               'suggestions · local',
    'Gemini':               'suggestions',
    'Gemma':                'suggestions',
    'Gemini Vision':        'sentiment',
    'Anthropic':            'suggestions · sentiment',
  };

  function purpose(name: string): string {
    if (PURPOSE[name]) return PURPOSE[name];
    if (name.startsWith('Ollama')) return 'suggestions · local';
    if (name.includes('Whisper')) return 'transcription';
    return '';
  }

  function serviceOrder(name: string): number {
    const p = purpose(name);
    if (p.startsWith('transcription')) return 0;
    if (p.startsWith('suggestions')) return 1;
    if (p.startsWith('sentiment')) return 2;
    return 3;
  }

  type RowEntry =
    | { kind: 'rate'; provider: string; entry: RateLimitEntry }
    | { kind: 'count'; provider: string; count: number };

  const allRows = $derived(
    [
      ...Object.entries(rateLimits).map(([provider, entry]): RowEntry => ({ kind: 'rate', provider, entry })),
      ...Object.entries(callCounts)
        .filter(([p]) => !rateLimits[p])
        .map(([provider, count]): RowEntry => ({ kind: 'count', provider, count })),
    ].sort((a, b) => serviceOrder(a.provider) - serviceOrder(b.provider))
  );

  function fillColor(_provider: string, _p: number): string {
    return '#475569';
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
    {#if allRows.length === 0}
      <p class="empty">API usage will appear here once the interview starts.</p>
    {:else}
      {#each allRows as row}
        {#if row.kind === 'rate'}
          {@const p = pct(row.entry)}
          {@const rate = computeRate(row.entry.history)}
          {@const warn = isNearLimit(row.entry)}
          <div class="provider-row" class:provider-warn={warn}>
            <div class="provider-header">
              <div class="provider-name-block">
                <span class="provider-name">{row.provider}</span>
                {#if purpose(row.provider)}<span class="provider-purpose">{purpose(row.provider)}</span>{/if}
              </div>
              <span class="counts">{row.entry.remaining.toLocaleString()} / {row.entry.limit.toLocaleString()}</span>
            </div>
            <div class="bar-track">
              <div class="bar-fill" style="width: {p}%; background: {fillColor(row.provider, p)}"></div>
            </div>
            <div class="provider-meta">
              <span class="pct" style="color: {fillColor(row.provider, p)}">{p.toFixed(1)}% remaining</span>
              <span class="rate">
                {#if rate !== null}
                  {rate.toFixed(1)} req/min · {timeLeft(row.entry.remaining, rate)}
                {:else}
                  usage rate: tracking...
                {/if}
              </span>
            </div>
            {#if callCounts[row.provider] != null}
              <div class="call-count">{callCounts[row.provider]} call{callCounts[row.provider] !== 1 ? 's' : ''} this session</div>
            {/if}
            {#if warn}
              <div class="warn-badge">⚠ Rate limit low — responses may slow down</div>
            {/if}
          </div>
        {:else}
          <div class="provider-row provider-row-counts-only">
            <div class="provider-header">
              <div class="provider-name-block">
                <span class="provider-name">{row.provider}</span>
                {#if purpose(row.provider)}<span class="provider-purpose">{purpose(row.provider)}</span>{/if}
              </div>
              <span class="counts">{row.count} call{row.count !== 1 ? 's' : ''}</span>
            </div>
          </div>
        {/if}
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
    font-size: var(--fs-sm);
    color: #64748b;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .service-provider {
    font-size: var(--fs-sm);
    font-weight: 600;
    color: #94a3b8;
  }
  .service-local {
    color: #94a3b8;
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
  .provider-purpose { font-size: var(--fs-sm); color: #475569; }
  .counts {
    font-size: var(--fs-sm);
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
  .call-count { font-size: var(--fs-sm); color: #475569; }
  .provider-row-counts-only { border-color: #1e293b; opacity: 0.7; }
  .provider-warn { border-color: #334155; }
  .warn-badge { font-size: var(--fs-sm); color: #94a3b8; font-weight: 600; letter-spacing: 0.02em; }
</style>
