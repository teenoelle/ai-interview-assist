<script lang="ts">
  import type { SuggestionEntry } from '../lib/types';
  import { TAG_CONFIG } from '../lib/questionTagger';

  const { suggestions, onClear, teleprompter = false, jumpSignal = null } = $props<{
    suggestions: SuggestionEntry[];
    onClear: () => void;
    teleprompter?: boolean;
    jumpSignal?: { idx: number; key: number } | null;
  }>();

  // -1 = pinned to latest; >= 0 = viewing specific entry
  let historyIndex = $state(-1);
  let lastSeenCount = $state(0);

  // Track when new questions arrive while user is viewing an older one
  const hasNewQuestion = $derived(
    historyIndex !== -1 && suggestions.length > lastSeenCount
  );

  $effect(() => {
    // When user is pinned to latest, keep lastSeenCount in sync
    if (historyIndex === -1) lastSeenCount = suggestions.length;
  });

  // Handle external jump signal from parent (QuestionsHistoryPanel)
  $effect(() => {
    if (jumpSignal != null) {
      jumpTo(jumpSignal.idx);
    }
  });

  // Expanded state per entry
  let expanded = $state<boolean[]>([]);
  $effect(() => {
    if (expanded.length < suggestions.length) {
      expanded = [...expanded, ...new Array(suggestions.length - expanded.length).fill(false)];
    }
  });

  // Auto-scroll nav strip to show active tab
  let navStrip: HTMLElement | undefined = $state();
  $effect(() => {
    if (!navStrip) return;
    const active = navStrip.querySelector<HTMLElement>('.q-tab.active');
    if (active) active.scrollIntoView({ inline: 'nearest', block: 'nearest' });
  });

  let container: HTMLElement | undefined = $state();
  $effect(() => {
    if (!teleprompter && suggestions.length && container) {
      container.scrollTop = container.scrollHeight;
    }
  });

  function renderBold(text: string): string {
    return text.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>');
  }

  function parseSuggestion(text: string): { affirm: string; cue: string; tell: string; body: string; asks: string[] } {
    const lines = text.split('\n');
    let affirm = '';
    let tell = '';
    let cue = 'Say';
    let body = '';
    const asks: string[] = [];
    const bodyLines: string[] = [];
    let pastSeparator = false;

    for (const line of lines) {
      const trimmed = line.trim();
      if (!pastSeparator) {
        if (trimmed.match(/^Affirm:/i)) {
          affirm = trimmed.replace(/^Affirm:\s*/i, '').trim();
        } else if (trimmed.match(/^(Tell|Say):/i)) {
          cue = 'Say';
          tell = trimmed.replace(/^(Tell|Say):\s*/i, '').trim();
        } else if (trimmed.match(/^Ask:/i) && !tell) {
          cue = 'Ask';
          tell = trimmed.replace(/^Ask:\s*/i, '').trim();
        } else if (trimmed === '---') {
          pastSeparator = true;
        }
      } else {
        if (trimmed.match(/^Ask:/i)) {
          const a = trimmed.replace(/^Ask:\s*/i, '').trim();
          if (a) asks.push(a);
        } else {
          bodyLines.push(line);
        }
      }
    }

    body = bodyLines.join('\n').trim();

    if (!tell && text) {
      const firstSentence = text.split(/[.\n]/)[0]?.trim() ?? '';
      tell = firstSentence.length > 80 ? firstSentence.slice(0, 80) + '…' : firstSentence;
      body = text;
    }

    return { affirm, cue, tell, body, asks };
  }

  const totalCount = $derived(suggestions.length);
  const currentIndex = $derived(historyIndex === -1 ? totalCount - 1 : historyIndex);
  const current = $derived(currentIndex >= 0 && currentIndex < totalCount ? suggestions[currentIndex] : null);

  function jumpTo(i: number) {
    if (i === totalCount - 1) {
      historyIndex = -1;
      lastSeenCount = totalCount;
    } else {
      historyIndex = i;
    }
  }

  function jumpToLatest() {
    historyIndex = -1;
    lastSeenCount = totalCount;
  }

  function toggleExpand(i: number) {
    expanded = expanded.map((v, j) => j === i ? !v : v);
  }

  function shortQ(q: string, max = 28): string {
    return q.length > max ? q.slice(0, max) + '…' : q;
  }

  const unansweredCount = $derived(
    suggestions.filter((s, i) => s.answered === false && i !== currentIndex).length
  );
</script>

{#if teleprompter}
  <div class="teleprompter">

    <!-- Question tabs navigation strip -->
    {#if totalCount > 0}
      <div class="q-nav" bind:this={navStrip}>
        {#each suggestions as entry, i (i)}
          <button
            class="q-tab"
            class:active={i === currentIndex}
            class:is-latest={i === totalCount - 1}
            onclick={() => jumpTo(i)}
            title={entry.question}
          >
            <span class="q-num">Q{i + 1}</span>
            <span class="q-snippet">{shortQ(entry.question)}</span>
            {#if i === totalCount - 1 && totalCount > 1}
              <span class="latest-dot" class:new={hasNewQuestion}></span>
            {/if}
          </button>
        {/each}

        <button class="clear-tab" onclick={onClear} title="Clear all suggestions">✕</button>
      </div>

      <!-- New question alert banner -->
      {#if hasNewQuestion}
        <button class="new-q-alert" onclick={jumpToLatest}>
          <span class="alert-pulse"></span>
          New question — Q{totalCount} →
        </button>
      {/if}
    {/if}

    <!-- Unanswered previous questions banner -->
    {#if unansweredCount > 0}
      <div class="unanswered-banner">
        {unansweredCount} previous question{unansweredCount > 1 ? 's' : ''} not yet answered
        — check Questions panel
      </div>
    {/if}

    <!-- Main suggestion card -->
    {#if current}
      {@const parsed = parseSuggestion(current.suggestion)}
      <div class="tp-card">
        <div class="tp-question-row">
          {#if current.tag}
            {@const tc = TAG_CONFIG[current.tag]}
            <span class="tp-tag" style="color: {tc.color}; background: {tc.bg}">{tc.label}</span>
          {/if}
          <span class="tp-question">"{current.question}"</span>
        </div>
        {#if current.redFlag}
          <div class="tp-redflag">
            <span class="redflag-cat">{current.redFlag.category}</span>
            <span class="redflag-note">{current.redFlag.coachingNote}</span>
          </div>
        {/if}

        {#if current.streaming && !current.suggestion}
          <span class="tp-loading">Generating<span class="dots">...</span></span>
        {:else}
          {#if parsed.affirm}
            <div class="tp-affirm-row">
              <span class="cue-badge cue-affirm">Affirm</span>
              <span class="tp-affirm-text">{parsed.affirm}</span>
            </div>
          {/if}
          <div class="tp-cue-row">
            <span class="cue-badge" class:cue-ask={parsed.cue === 'Ask'}>{parsed.cue}</span>
            <span class="tp-tell">{parsed.tell}{#if current.streaming && !parsed.body}<span class="cursor">|</span>{/if}</span>
          </div>

          {#each parsed.asks as ask}
            <div class="tp-ask-row tp-ask-always">
              <span class="cue-badge cue-ask">Ask</span>
              <span class="tp-ask-text">{ask}</span>
            </div>
          {/each}
          {#if current.answerFeedback}
            <div class="tp-feedback">
              {#if current.answerFeedback.missed_followup}
                <span class="fb-badge fb-orange">Forgot follow-up</span>
              {/if}
              {#if current.answerFeedback.missed_metric}
                <span class="fb-badge fb-orange">Add a metric</span>
              {/if}
              <p class="fb-coaching">{current.answerFeedback.coaching}</p>
            </div>
          {/if}

          {#if parsed.body}
            <div class="tp-expand-row">
              <button class="tp-expand-btn" onclick={() => toggleExpand(currentIndex)}>
                {expanded[currentIndex] ? '▴ Less context' : '▾ More context'}
              </button>
            </div>

            {#if expanded[currentIndex]}
              <div class="tp-body">
                {@html renderBold(parsed.body)}
                {#if current.streaming}<span class="cursor">|</span>{/if}
              </div>
            {/if}
          {/if}
        {/if}
      </div>
    {:else}
      <div class="tp-empty">Waiting for a question...</div>
    {/if}

    <span class="tp-hint">Affirm = acknowledge concern · Say = speak · Ask = questions to ask them</span>
  </div>

{:else}
  <!-- Standard panel mode -->
  <div class="suggestion-panel">
    <div class="panel-header">
      <div class="header-left">
        <h3>AI Suggestions</h3>
        <span class="glance-hint">Say = speak it · Ask = optional question to ask</span>
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
              <span class="q-num-badge">Q{i + 1}</span>
              <p class="question-text">"{entry.question}"</p>
              {#if entry.tag}
                {@const tc = TAG_CONFIG[entry.tag]}
                <span class="entry-tag" style="color: {tc.color}; background: {tc.bg}">{tc.label}</span>
              {/if}
            </div>
            {#if entry.redFlag}
              <div class="entry-redflag">
                <span class="redflag-cat">{entry.redFlag.category}</span>
                <span class="redflag-note">{entry.redFlag.coachingNote}</span>
              </div>
            {/if}
            {#if entry.streaming && !entry.suggestion}
              <span class="loading">Generating<span class="dots">...</span></span>
            {:else}
              {#if parsed.affirm}
                <div class="affirm-row">
                  <span class="cue-badge cue-affirm">Affirm</span>
                  <span class="affirm-text">{parsed.affirm}</span>
                </div>
              {/if}
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
                  {#each parsed.asks as ask}
                      <div class="ask-row">
                        <span class="cue-badge cue-ask">Ask</span>
                        <span>{ask}</span>
                      </div>
                    {/each}
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
    gap: 0.4rem;
  }

  /* Question tab navigation strip */
  .q-nav {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    flex-shrink: 0;
    overflow-x: auto;
    scrollbar-width: none;
    padding-bottom: 0.1rem;
  }
  .q-nav::-webkit-scrollbar { display: none; }

  .q-tab {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    padding: 0.3rem 0.6rem;
    background: #0d1a2b;
    border: 1px solid #1a2d4a;
    border-radius: 0.4rem;
    color: #334155;
    font-size: 0.68rem;
    cursor: pointer;
    white-space: nowrap;
    flex-shrink: 0;
    transition: all 0.15s;
    max-width: 180px;
  }
  .q-tab:hover { background: #111e33; border-color: #2a4a6a; color: #64748b; }
  .q-tab.active {
    background: #0a1f10;
    border-color: #166534;
    color: #4ade80;
  }
  .q-tab.active .q-num { color: #4ade80; }

  .q-num {
    font-weight: 800;
    font-size: 0.6rem;
    color: #475569;
    flex-shrink: 0;
    letter-spacing: 0.04em;
  }
  .q-snippet {
    overflow: hidden;
    text-overflow: ellipsis;
    font-style: italic;
  }

  /* Pulsing dot on latest tab when new question arrived */
  .latest-dot {
    width: 5px; height: 5px;
    border-radius: 50%;
    background: #334155;
    flex-shrink: 0;
  }
  .latest-dot.new {
    background: #4ade80;
    animation: dotpulse 1s ease-in-out infinite;
  }
  @keyframes dotpulse { 0%, 100% { opacity: 1; transform: scale(1); } 50% { opacity: 0.4; transform: scale(0.7); } }

  .clear-tab {
    margin-left: auto;
    flex-shrink: 0;
    padding: 0.25rem 0.5rem;
    background: transparent;
    border: 1px solid #0f1e33;
    border-radius: 0.3rem;
    color: #1e293b;
    font-size: 0.65rem;
    cursor: pointer;
    transition: all 0.15s;
  }
  .clear-tab:hover { border-color: #334155; color: #475569; }

  /* New question alert banner */
  .new-q-alert {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    width: 100%;
    padding: 0.35rem 0.75rem;
    background: #0a1f10;
    border: 1px solid #166534;
    border-radius: 0.4rem;
    color: #4ade80;
    font-size: 0.72rem;
    font-weight: 600;
    cursor: pointer;
    flex-shrink: 0;
    transition: background 0.15s;
    animation: alertslide 0.3s ease-out;
  }
  .new-q-alert:hover { background: #0d2a16; }
  @keyframes alertslide { from { opacity: 0; transform: translateY(-4px); } to { opacity: 1; transform: none; } }

  .alert-pulse {
    width: 7px; height: 7px;
    border-radius: 50%;
    background: #4ade80;
    flex-shrink: 0;
    animation: dotpulse 0.8s ease-in-out infinite;
  }

  /* Unanswered banner */
  .unanswered-banner {
    padding: 0.3rem 0.75rem;
    background: #1a0f00;
    border: 1px solid #92400e;
    border-radius: 0.35rem;
    color: #f59e0b;
    font-size: 0.7rem;
    font-weight: 600;
    flex-shrink: 0;
    text-align: center;
  }

  /* Main card */
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
    overflow-wrap: break-word;
    word-break: break-word;
  }

  /* CUE badges */
  .cue-badge {
    display: inline-block;
    padding: 0.1rem 0.45rem;
    background: #14532d;
    color: #4ade80;
    border-radius: 0.25rem;
    font-size: 0.6rem;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    flex-shrink: 0;
  }
  .cue-badge.cue-ask { background: #3b0764; color: #c084fc; }
  .cue-badge.cue-affirm { background: #1a2d4a; color: #60a5fa; }

  /* Teleprompter Affirm row */
  .tp-affirm-row {
    display: flex;
    align-items: flex-start;
    gap: 0.5rem;
    padding: 0.35rem 0.6rem;
    background: #070f1e;
    border-radius: 0.4rem;
    border-left: 2px solid #1e3a5f;
    flex-shrink: 0;
  }
  .tp-affirm-text {
    color: #60a5fa;
    font-size: 0.82rem;
    font-style: italic;
    line-height: 1.4;
    overflow-wrap: break-word;
  }

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
    overflow-wrap: break-word;
    word-break: break-word;
  }

  .tp-expand-row { flex-shrink: 0; }
  .tp-expand-btn {
    background: #0d1525;
    border: 1px solid #1e2d45;
    color: #475569;
    font-size: 0.68rem;
    cursor: pointer;
    padding: 0.2rem 0.6rem;
    border-radius: 0.25rem;
    align-self: flex-start;
    transition: all 0.15s;
  }
  .tp-expand-btn:hover { border-color: #334155; color: #64748b; }

  .tp-body {
    color: #7a9ab8;
    font-size: 0.82rem;
    line-height: 1.8;
    white-space: pre-wrap;
    border-top: 1px solid #0f1e33;
    padding-top: 0.6rem;
    overflow-wrap: break-word;
    word-break: break-word;
  }
  :global(.tp-body strong) { color: #b8cce4; font-weight: 700; }

  .tp-ask-row {
    display: flex;
    align-items: flex-start;
    gap: 0.4rem;
    margin-top: 0.5rem;
  }
  .tp-ask-always {
    padding: 0.4rem 0.6rem;
    background: #160a2a;
    border-radius: 0.4rem;
    border-left: 2px solid #7c3aed;
    margin-top: 0.25rem;
  }
  .tp-ask-text { color: #7a9ab8; font-size: 0.82rem; font-style: italic; overflow-wrap: break-word; }

  /* Answer feedback */
  .tp-feedback {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-start;
    gap: 0.35rem;
    padding: 0.5rem 0.7rem;
    background: #0d0d1a;
    border: 1px solid #2a1a3a;
    border-left: 3px solid #7c3aed;
    border-radius: 0.4rem;
    margin-top: 0.25rem;
  }
  .fb-badge {
    padding: 0.1rem 0.45rem;
    border-radius: 0.25rem;
    font-size: 0.58rem;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    flex-shrink: 0;
  }
  .fb-orange { background: #431407; color: #fb923c; }
  .fb-coaching {
    width: 100%;
    margin: 0;
    font-size: 0.75rem;
    color: #94a3b8;
    line-height: 1.5;
    font-style: italic;
  }

  .tp-hint {
    font-size: 0.58rem;
    color: #1e293b;
    font-style: italic;
    flex-shrink: 0;
    text-align: center;
  }

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

  /* Question tag + red flag */
  .tp-question-row {
    display: flex;
    align-items: flex-start;
    gap: 0.4rem;
    flex-shrink: 0;
    flex-wrap: wrap;
  }
  .tp-tag {
    display: inline-block;
    padding: 0.08rem 0.4rem;
    border-radius: 0.2rem;
    font-size: 0.55rem;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    flex-shrink: 0;
    margin-top: 0.05rem;
  }
  .tp-redflag {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    padding: 0.4rem 0.6rem;
    background: #1a0800;
    border-left: 3px solid #b45309;
    border-radius: 0.3rem;
    flex-shrink: 0;
  }
  .redflag-cat {
    font-size: 0.58rem;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: #f59e0b;
  }
  .redflag-note {
    font-size: 0.72rem;
    color: #94a3b8;
    line-height: 1.4;
    font-style: italic;
  }
  .entry-tag {
    display: inline-block;
    padding: 0.05rem 0.35rem;
    border-radius: 0.2rem;
    font-size: 0.52rem;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    flex-shrink: 0;
    align-self: flex-start;
    margin-top: 0.15rem;
  }
  .entry-redflag {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    padding: 0.3rem 0.5rem;
    background: #1a0800;
    border-left: 2px solid #b45309;
    border-radius: 0.25rem;
    margin-bottom: 0.1rem;
  }

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
    border-left-color: #4ade80;
    opacity: 1;
  }

  .question-row {
    display: flex;
    align-items: flex-start;
    gap: 0.5rem;
    flex-wrap: wrap;
  }
  .q-num-badge {
    flex-shrink: 0;
    padding: 0.12rem 0.45rem;
    background: #14532d;
    color: #4ade80;
    border-radius: 9999px;
    font-size: 0.6rem;
    font-weight: 800;
    text-transform: uppercase;
    margin-top: 0.1rem;
    letter-spacing: 0.04em;
  }
  .question-text {
    color: #93c5fd;
    font-style: italic;
    font-size: 0.82rem;
    margin: 0;
    line-height: 1.4;
    overflow-wrap: break-word;
    word-break: break-word;
  }

  /* Standard panel Affirm row */
  .affirm-row {
    display: flex;
    align-items: flex-start;
    gap: 0.4rem;
    padding: 0.3rem 0.5rem;
    background: #070f1e;
    border-radius: 0.3rem;
    border-left: 2px solid #1e3a5f;
  }
  .affirm-text {
    color: #60a5fa;
    font-size: 0.78rem;
    font-style: italic;
    line-height: 1.4;
    flex: 1;
    overflow-wrap: break-word;
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
    overflow-wrap: break-word;
    word-break: break-word;
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
    overflow-wrap: break-word;
    word-break: break-word;
  }
  :global(.body-text strong) { color: #b8cce4; font-weight: 700; }

  .ask-row {
    display: flex;
    align-items: flex-start;
    gap: 0.4rem;
    margin-top: 0.35rem;
    font-size: 0.78rem;
    color: #7a9ab8;
    font-style: italic;
  }

  .cursor { animation: blink 1s step-end infinite; }
  @keyframes blink { 50% { opacity: 0; } }

  .loading { color: #60a5fa; font-style: italic; font-size: 0.82rem; }

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
