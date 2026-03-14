<script lang="ts">
  import type { SuggestionEntry } from '../lib/types';

  const { suggestions, onClear, teleprompter = false } = $props<{
    suggestions: SuggestionEntry[];
    onClear: () => void;
    teleprompter?: boolean;
  }>();

  // History navigation: index of which suggestion to show (-1 = latest)
  let historyIndex = $state(-1);
  $effect(() => {
    // When new suggestion arrives, jump back to latest
    void suggestions.length;
    historyIndex = -1;
  });

  // Expanded state per entry (indexed by position)
  let expanded = $state<boolean[]>([]);
  $effect(() => {
    // Grow array as suggestions arrive
    if (expanded.length < suggestions.length) {
      expanded = [...expanded, ...new Array(suggestions.length - expanded.length).fill(false)];
    }
  });

  let container: HTMLElement;

  $effect(() => {
    if (!teleprompter && suggestions.length && container) {
      container.scrollTop = container.scrollHeight;
    }
  });

  function renderBold(text: string): string {
    return text.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>');
  }

  /** Split a suggestion into { cue, tell, body, ask } */
  function parseSuggestion(text: string): { cue: string; tell: string; body: string; ask: string } {
    const lines = text.split('\n');
    let tell = '';
    let cue = 'Tell';
    let body = '';
    let ask = '';
    let bodyLines: string[] = [];
    let pastSeparator = false;

    for (const line of lines) {
      const trimmed = line.trim();
      if (!pastSeparator) {
        if (trimmed.match(/^Tell:/i)) {
          cue = 'Tell';
          tell = trimmed.replace(/^Tell:\s*/i, '').trim();
        } else if (trimmed.match(/^Ask:/i) && !tell) {
          cue = 'Ask';
          tell = trimmed.replace(/^Ask:\s*/i, '').trim();
        } else if (trimmed === '---') {
          pastSeparator = true;
        }
      } else {
        if (trimmed.match(/^Ask:/i)) {
          ask = trimmed.replace(/^Ask:\s*/i, '').trim();
        } else {
          bodyLines.push(line);
        }
      }
    }

    body = bodyLines.join('\n').trim();

    // Fallback: if no Tell: prefix found, use first sentence as tell
    if (!tell && text) {
      const firstSentence = text.split(/[.\n]/)[0]?.trim() ?? '';
      tell = firstSentence.length > 80 ? firstSentence.slice(0, 80) + '…' : firstSentence;
      body = text;
    }

    return { cue, tell, body, ask };
  }

  const totalCount = $derived(suggestions.length);
  const currentIndex = $derived(historyIndex === -1 ? totalCount - 1 : historyIndex);
  const current = $derived(currentIndex >= 0 && currentIndex < totalCount ? suggestions[currentIndex] : null);
  const previous = $derived(suggestions.length > 1 ? suggestions.slice(0, -1) : []);

  function goBack() {
    const idx = historyIndex === -1 ? totalCount - 1 : historyIndex;
    if (idx > 0) historyIndex = idx - 1;
  }
  function goForward() {
    const idx = historyIndex === -1 ? totalCount - 1 : historyIndex;
    if (idx < totalCount - 1) historyIndex = idx + 1;
    else historyIndex = -1;
  }
  function toggleExpand(i: number) {
    expanded = expanded.map((v, j) => j === i ? !v : v);
  }
</script>

{#if teleprompter}
  <!-- Teleprompter mode -->
  <div class="teleprompter">
    <div class="tp-header">
      <span class="tp-hint">Tell = say it · Ask = ask them · bold = keywords</span>
      <div class="tp-controls">
        {#if totalCount > 1}
          <button class="nav-btn" onclick={goBack} disabled={currentIndex <= 0} title="Previous suggestion">‹</button>
          <span class="nav-count">{currentIndex + 1}/{totalCount}</span>
          <button class="nav-btn" onclick={goForward} disabled={currentIndex >= totalCount - 1} title="Latest suggestion">›</button>
        {/if}
        {#if totalCount > 0}
          <button class="tp-clear" onclick={onClear}>Clear</button>
        {/if}
      </div>
    </div>

    {#if current}
      {@const parsed = parseSuggestion(current.suggestion)}
      <div class="tp-card">
        <div class="tp-question">"{current.question}"</div>

        {#if current.streaming && !current.suggestion}
          <span class="tp-loading">Generating<span class="dots">...</span></span>
        {:else}
          <!-- CUE line — speak this first -->
          <div class="tp-cue-row">
            <span class="cue-badge" class:cue-ask={parsed.cue === 'Ask'}>{parsed.cue}</span>
            <span class="tp-tell">{parsed.tell}{#if current.streaming && !parsed.body}<span class="cursor">|</span>{/if}</span>
          </div>

          {#if parsed.body}
            <div class="tp-expand-row">
              <button
                class="tp-expand-btn"
                onclick={() => toggleExpand(currentIndex)}
              >
                {expanded[currentIndex] ? '▴ Less' : '▾ Context'}
              </button>
            </div>

            {#if expanded[currentIndex]}
              <div class="tp-body">
                {@html renderBold(parsed.body)}
                {#if current.streaming}<span class="cursor">|</span>{/if}
                {#if parsed.ask}
                  <div class="tp-ask-row">
                    <span class="cue-badge cue-ask">Ask</span>
                    <span class="tp-ask-text">{parsed.ask}</span>
                  </div>
                {/if}
              </div>
            {/if}
          {/if}
        {/if}
      </div>

      <!-- Previous questions bar -->
      {#if previous.length > 0 && historyIndex === -1}
        <div class="tp-history">
          {#each previous as entry, i (i)}
            <button class="tp-history-item" onclick={() => historyIndex = i} title={entry.question}>
              Q{i + 1}: {entry.question}
            </button>
          {/each}
        </div>
      {/if}
    {:else}
      <div class="tp-empty">Waiting for a question...</div>
    {/if}
  </div>

{:else}
  <!-- Standard panel mode -->
  <div class="suggestion-panel">
    <div class="panel-header">
      <div class="header-left">
        <h3>AI Suggestions</h3>
        <span class="glance-hint">Tell = say it · Ask = optional question to ask</span>
      </div>
      {#if suggestions.length > 0}
        <button class="clear-btn" onclick={onClear}>Clear</button>
      {/if}
    </div>

    <div class="entries" bind:this={container}>
      {#if suggestions.length === 0}
        <p class="empty">Suggestions will appear when the interviewer speaks...</p>
      {:else}
        {#each suggestions as entry, i (i)}
          {@const parsed = parseSuggestion(entry.suggestion)}
          {@const isLatest = i === suggestions.length - 1}
          <div class="entry" class:latest={isLatest}>
            <div class="question-row">
              <span class="badge">Interviewer</span>
              <p class="question-text">"{entry.question}"</p>
            </div>
            {#if entry.streaming && !entry.suggestion}
              <span class="loading">Generating<span class="dots">...</span></span>
            {:else}
              <div class="cue-row">
                <span class="cue-badge" class:cue-ask={parsed.cue === 'Ask'}>{parsed.cue}</span>
                <span class="tell-text">{parsed.tell}{#if entry.streaming && !parsed.body}<span class="cursor">|</span>{/if}</span>
              </div>
              {#if parsed.body}
                <button class="expand-btn" onclick={() => toggleExpand(i)}>
                  {expanded[i] ? '▴ Less' : '▾ Full answer'}
                </button>
                {#if expanded[i]}
                  <div class="body-text">
                    {@html renderBold(parsed.body)}
                    {#if entry.streaming}<span class="cursor">|</span>{/if}
                    {#if parsed.ask}
                      <div class="ask-row">
                        <span class="cue-badge cue-ask">Ask</span>
                        <span>{parsed.ask}</span>
                      </div>
                    {/if}
                  </div>
                {/if}
              {/if}
            {/if}
          </div>
        {/each}
      {/if}
    </div>
  </div>
{/if}

<style>
  /* === Teleprompter mode === */
  .teleprompter {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .tp-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-shrink: 0;
    gap: 0.5rem;
  }

  .tp-hint {
    font-size: 0.6rem;
    color: #334155;
    font-style: italic;
    flex: 1;
  }

  .tp-controls {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    flex-shrink: 0;
  }

  .nav-btn {
    padding: 0.05rem 0.4rem;
    background: transparent;
    border: 1px solid #1e293b;
    border-radius: 0.2rem;
    color: #475569;
    font-size: 0.85rem;
    cursor: pointer;
    line-height: 1.4;
  }
  .nav-btn:hover:not(:disabled) { border-color: #475569; color: #94a3b8; }
  .nav-btn:disabled { opacity: 0.3; cursor: default; }
  .nav-count { font-size: 0.65rem; color: #334155; min-width: 28px; text-align: center; }

  .tp-clear {
    padding: 0.1rem 0.5rem;
    background: transparent;
    border: 1px solid #1e293b;
    border-radius: 0.25rem;
    color: #475569;
    font-size: 0.7rem;
    cursor: pointer;
  }
  .tp-clear:hover { border-color: #475569; color: #94a3b8; }

  .tp-card {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    padding: 1rem 1.25rem;
    background: #07101e;
    border-radius: 0.75rem;
    border: 1px solid #1a2d4a;
    overflow-y: auto;
    min-height: 0;
  }

  .tp-question {
    color: #4d94d4;
    font-style: italic;
    font-size: 0.82rem;
    line-height: 1.4;
    padding-bottom: 0.6rem;
    border-bottom: 1px solid #0f1e33;
    flex-shrink: 0;
  }

  /* CUE badges */
  .cue-badge {
    display: inline-block;
    padding: 0.1rem 0.45rem;
    background: #1d4ed8;
    color: white;
    border-radius: 0.25rem;
    font-size: 0.6rem;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    flex-shrink: 0;
  }
  .cue-badge.cue-ask { background: #7c3aed; }

  /* Tell line — the key thing to say */
  .tp-cue-row {
    display: flex;
    align-items: flex-start;
    gap: 0.5rem;
    flex-shrink: 0;
  }
  .tp-tell {
    color: #e2e8f0;
    font-size: 1.05rem;
    font-weight: 600;
    line-height: 1.5;
    flex: 1;
  }

  /* Expand context */
  .tp-expand-row { flex-shrink: 0; }
  .tp-expand-btn {
    background: transparent;
    border: none;
    color: #334155;
    font-size: 0.68rem;
    cursor: pointer;
    padding: 0;
  }
  .tp-expand-btn:hover { color: #64748b; }

  .tp-body {
    color: #7a9ab8;
    font-size: 0.82rem;
    line-height: 1.8;
    white-space: pre-wrap;
    border-top: 1px solid #0f1e33;
    padding-top: 0.6rem;
  }
  :global(.tp-body strong) {
    color: #b8cce4;
    font-weight: 700;
  }

  .tp-ask-row {
    display: flex;
    align-items: flex-start;
    gap: 0.4rem;
    margin-top: 0.5rem;
  }
  .tp-ask-text { color: #7a9ab8; font-size: 0.82rem; font-style: italic; }

  .tp-loading {
    color: #4d94d4;
    font-style: italic;
    font-size: 0.9rem;
  }

  .tp-empty {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #1e293b;
    font-style: italic;
    font-size: 0.85rem;
  }

  .tp-history {
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    max-height: 4rem;
    overflow-y: auto;
  }

  .tp-history-item {
    font-size: 0.62rem;
    color: #1e3a5f;
    padding: 0.1rem 0.5rem;
    background: #06101a;
    border: none;
    border-radius: 0.2rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    cursor: pointer;
    text-align: left;
    width: 100%;
  }
  .tp-history-item:hover { color: #334155; background: #080e1a; }

  /* === Standard panel mode === */
  .suggestion-panel {
    height: 100%;
    display: flex;
    flex-direction: column;
  }
  .panel-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    margin-bottom: 0.75rem;
    gap: 0.5rem;
  }
  .header-left {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }
  .glance-hint {
    font-size: 0.62rem;
    color: #475569;
    font-style: italic;
  }
  h3 {
    font-size: 0.85rem;
    font-weight: 600;
    color: #94a3b8;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin: 0;
  }
  .clear-btn {
    padding: 0.15rem 0.6rem;
    background: transparent;
    border: 1px solid #334155;
    border-radius: 0.25rem;
    color: #64748b;
    font-size: 0.75rem;
    cursor: pointer;
  }
  .clear-btn:hover { border-color: #64748b; color: #94a3b8; }

  .entries {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .entry {
    padding: 0.65rem;
    background: #0f1e33;
    border-radius: 0.5rem;
    border-left: 3px solid #334155;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    opacity: 0.6;
    transition: opacity 0.2s;
  }
  .entry.latest {
    border-left-color: #3b82f6;
    opacity: 1;
  }

  .question-row {
    display: flex;
    align-items: flex-start;
    gap: 0.5rem;
    flex-wrap: wrap;
  }
  .badge {
    flex-shrink: 0;
    padding: 0.12rem 0.45rem;
    background: #1d4ed8;
    color: white;
    border-radius: 9999px;
    font-size: 0.6rem;
    font-weight: 700;
    text-transform: uppercase;
    margin-top: 0.1rem;
  }
  .question-text {
    color: #93c5fd;
    font-style: italic;
    font-size: 0.82rem;
    margin: 0;
    line-height: 1.4;
  }

  .cue-row {
    display: flex;
    align-items: flex-start;
    gap: 0.4rem;
  }
  .tell-text {
    color: #e2e8f0;
    font-size: 0.88rem;
    font-weight: 600;
    line-height: 1.4;
    flex: 1;
  }

  .expand-btn {
    background: transparent;
    border: none;
    color: #334155;
    font-size: 0.65rem;
    cursor: pointer;
    padding: 0;
    align-self: flex-start;
  }
  .expand-btn:hover { color: #64748b; }

  .body-text {
    color: #7a9ab8;
    line-height: 1.7;
    white-space: pre-wrap;
    font-size: 0.8rem;
    border-top: 1px solid #1a2d4a;
    padding-top: 0.4rem;
  }
  :global(.body-text strong) {
    color: #b8cce4;
    font-weight: 700;
  }

  .ask-row {
    display: flex;
    align-items: flex-start;
    gap: 0.4rem;
    margin-top: 0.35rem;
    font-size: 0.78rem;
    color: #7a9ab8;
    font-style: italic;
  }

  .cursor {
    animation: blink 1s step-end infinite;
  }
  @keyframes blink { 50% { opacity: 0; } }

  .loading {
    color: #60a5fa;
    font-style: italic;
    font-size: 0.82rem;
  }

  .empty {
    color: #475569;
    font-style: italic;
    font-size: 0.85rem;
    text-align: center;
    padding: 2rem 1rem;
  }

  .dots { animation: ellipsis 1.5s infinite; }
  @keyframes ellipsis {
    0% { content: '.'; }
    33% { content: '..'; }
    66% { content: '...'; }
  }
</style>
