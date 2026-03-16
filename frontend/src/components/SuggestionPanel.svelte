<script lang="ts">
  import type { SuggestionEntry } from '../lib/types';
  import { TAG_CONFIG } from '../lib/questionTagger';
  import { parseSuggestion, parseCues } from '../lib/parseSuggestion';

  // Expand-cue state: cue text → { sentence, loading }
  let expandedCues = $state<Record<string, { sentence: string; loading: boolean }>>({});
  // Open/collapsed state for cue bullet blocks
  let openCues = $state<Record<string, boolean>>({});

  function toggleCueOpen(key: string) {
    openCues = { ...openCues, [key]: !openCues[key] };
  }

  async function expandCue(question: string, cue: string) {
    if (expandedCues[cue]?.sentence) { expandedCues = { ...expandedCues, [cue]: { sentence: '', loading: false } }; return; }
    expandedCues = { ...expandedCues, [cue]: { sentence: '', loading: true } };
    try {
      const r = await fetch('/api/expand-cue', {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ question, cue }),
      });
      const d = r.ok ? await r.json() : null;
      expandedCues = { ...expandedCues, [cue]: { sentence: d?.sentence ?? '', loading: false } };
    } catch { expandedCues = { ...expandedCues, [cue]: { sentence: '', loading: false } }; }
  }

  const { suggestions, onClear, teleprompter = false, jumpSignal = null, cueExpandSignal = null } = $props<{
    suggestions: SuggestionEntry[];
    onClear: () => void;
    teleprompter?: boolean;
    jumpSignal?: { idx: number; key: number } | null;
    cueExpandSignal?: { cueIdx: number; key: number } | null;
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

  // Handle keyboard cue expand signal (1/2/3 keys)
  $effect(() => {
    if (cueExpandSignal == null || !current) return;
    const body = parseSuggestion(current.suggestion).body;
    const cues = parseCues(body);
    const cue = cues[cueExpandSignal.cueIdx];
    if (cue) expandCue(current.question, cue.text);
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

  // Split a long cue phrase into short bullet lines at natural break points
  function splitBullets(text: string, maxLen = 65): string[] {
    if (text.length <= maxLen) return [text];
    // Split at sentence-ending punctuation first
    const bySentence = text.split(/(?<=[.!?])\s+/);
    if (bySentence.length > 1 && bySentence.every(s => s.length <= maxLen))
      return bySentence.map(s => s.trim()).filter(Boolean);
    // Otherwise split at commas
    const parts = text.split(/,\s+/);
    const lines: string[] = [];
    let cur = '';
    for (const p of parts) {
      const next = cur ? cur + ', ' + p : p;
      if (cur && next.length > maxLen) { lines.push(cur); cur = p; }
      else cur = next;
    }
    if (cur) lines.push(cur);
    return lines.length > 1 ? lines.map(s => s.trim()).filter(Boolean) : [text];
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

  let openAsks = $state<Record<string, boolean>>({});
  function toggleAsk(key: string) { openAsks = { ...openAsks, [key]: !openAsks[key] }; }

  function shortAsk(text: string, words = 6): string {
    const w = text.split(' ');
    return w.length <= words ? text : w.slice(0, words).join(' ') + '…';
  }
</script>

{#if teleprompter}
  <div class="teleprompter">
    <!-- Question header -->
    {#if current}
      <div class="tp-active-question">
        <div class="tp-active-q-row">
          {#if current.tag}
            {@const tc = TAG_CONFIG[current.tag]}
            <span class="tp-tag" style="color: {tc.color}; background: {tc.bg}">{tc.label}</span>
          {/if}
          {#if current.confidenceScore != null}
            <span class="tp-conf-badge"
              style="color: {current.confidenceScore >= 70 ? '#4ade80' : current.confidenceScore >= 40 ? '#f59e0b' : '#f87171'}">
              {current.confidenceScore}%
            </span>
          {/if}
        </div>
        <span class="tp-active-q-text">"{current.question}"</span>
      </div>

      {@const parsed = parseSuggestion(current.suggestion)}
      <div class="tp-card">
        {#if current.matchedStories && current.matchedStories.length > 0}
          <div class="tp-stories">
            <span class="tp-stories-label">Stories</span>
            {#each current.matchedStories as s}
              <div class="tp-story-chip" title={s.result}>
                <span class="tp-story-title">{s.title}</span>
                <span class="tp-story-result">{s.result.length > 60 ? s.result.slice(0, 60) + '…' : s.result}</span>
              </div>
            {/each}
          </div>
        {/if}
        {#if current.redFlag}
          <div class="tp-redflag">
            <span class="redflag-cat">{current.redFlag.category}</span>
            <span class="redflag-note">{current.redFlag.coachingNote}</span>
          </div>
        {/if}

        {#if current.streaming && !current.suggestion}
          <span class="tp-loading">Generating<span class="dots">...</span></span>
        {:else}
          <!-- ACKNOWLEDGE section -->
          {#if parsed.acknowledge}
            <div class="tp-sec tp-sec-ack">
              <div class="tp-sec-row">
                <span class="cue-badge cue-ack">Acknowledge</span>
                <span class="tp-ack-text">{parsed.acknowledge}</span>
              </div>
            </div>
          {/if}

          <!-- AFFIRM section -->
          {#if parsed.affirm}
            <div class="tp-sec tp-sec-affirm">
              <div class="tp-sec-row">
                <span class="cue-badge cue-affirm">Affirm</span>
                <span class="tp-affirm-text">{parsed.affirm}</span>
              </div>
            </div>
          {/if}

          <!-- ANSWER section (includes cue points) -->
          <div class="tp-sec tp-sec-say">
            <div class="tp-sec-row">
              <span class="cue-badge">{parsed.cue}</span>
              <span class="tp-tell">{parsed.tell}{#if current.streaming && !parsed.body}<span class="cursor">|</span>{/if}</span>
            </div>
            {#if parsed.body}
              {@const cues = parseCues(parsed.body)}
              {#if cues.length > 0}
                <div class="tp-cues">
                  {#each cues as cue}
                    {@const isOpen = !!openCues[cue.text]}
                    <div class="tp-cue-block" class:tp-cue-open={isOpen}>
                      <button class="tp-cue-toggle" onclick={() => toggleCueOpen(cue.text)}>
                        <span class="cue-label-sm">{cue.label}</span>
                        <span class="tp-cue-preview">{isOpen ? '' : cue.text.slice(0, 36) + (cue.text.length > 36 ? '…' : '')}</span>
                        <span class="tp-cue-chevron">{isOpen ? '▾' : '▸'}</span>
                      </button>
                      {#if isOpen}
                        <div class="tp-cue-body">
                          {#each splitBullets(cue.text) as line}
                            <div class="tp-cue-line">· {line}</div>
                          {/each}
                          <button class="tp-cue-speak-btn" onclick={() => expandCue(current.question, cue.text)}
                            title="Generate a spoken sentence">
                            {expandedCues[cue.text]?.sentence ? '▴ hide sentence' : '▾ speak as sentence'}
                          </button>
                          {#if expandedCues[cue.text]?.loading}
                            <div class="cue-sentence cue-loading">…</div>
                          {:else if expandedCues[cue.text]?.sentence}
                            <div class="cue-sentence">{expandedCues[cue.text].sentence}</div>
                          {/if}
                        </div>
                      {/if}
                    </div>
                  {/each}
                </div>
              {:else}
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

          <!-- ASK section -->
          {#if parsed.asks.length > 0}
            <div class="tp-sec tp-sec-ask">
              <div class="tp-ask-header">
                <span class="cue-badge cue-ask">Ask</span>
                <span class="tp-ask-hint">tap for alternatives</span>
              </div>
              {#each parsed.asks as ask, i}
                <button class="tp-ask-item" onclick={() => toggleAsk(`${i}`)}>
                  <span class="tp-ask-chevron">{openAsks[`${i}`] ? '▾' : '▸'}</span>
                  <span class="tp-ask-main">{openAsks[`${i}`] ? ask.main : shortAsk(ask.main)}</span>
                </button>
                {#if openAsks[`${i}`]}
                  {#if ask.alts.length > 0}
                    <div class="tp-ask-alts">
                      {#each ask.alts as alt}
                        <div class="tp-ask-alt">↳ {alt}</div>
                      {/each}
                    </div>
                  {/if}
                {/if}
              {/each}
            </div>
          {/if}
        {/if}
      </div>
    {:else}
      <div class="tp-empty">Waiting for a question...</div>
    {/if}

    <span class="tp-hint">Acknowledge = their pain point · Affirm = your alignment · Answer = speak · Ask = follow-up</span>
  </div>

{:else}
  <!-- Standard panel mode -->
  <div class="suggestion-panel">
    <div class="panel-header">
      <div class="header-left">
        <h3>AI Suggestions</h3>
        <span class="glance-hint">Say = speak it · Ask = follow-up questions</span>
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
              <!-- ACKNOWLEDGE -->
              {#if parsed.acknowledge}
                <div class="e-sec e-sec-ack">
                  <div class="e-sec-row">
                    <span class="cue-badge cue-ack">Acknowledge</span>
                    <span class="affirm-text">{parsed.acknowledge}</span>
                  </div>
                </div>
              {/if}
              <!-- AFFIRM -->
              {#if parsed.affirm}
                <div class="e-sec e-sec-affirm">
                  <div class="e-sec-row">
                    <span class="cue-badge cue-affirm">Affirm</span>
                    <span class="affirm-text">{parsed.affirm}</span>
                  </div>
                </div>
              {/if}
              <!-- ANSWER -->
              <div class="e-sec e-sec-say">
                <div class="e-sec-row">
                  <span class="cue-badge" class:cue-ask={parsed.cue === 'Ask'}>{parsed.cue}</span>
                  <span class="tell-text">{parsed.tell}{#if entry.streaming && !parsed.body}<span class="cursor">|</span>{/if}</span>
                </div>
                {#if parsed.body}
                  <button class="expand-btn" onclick={() => toggleExpand(i)}>
                    {expanded[i] ? '▴ Less' : '▾ More context'}
                  </button>
                  {#if expanded[i]}
                    <div class="body-text">
                      {@html renderBold(parsed.body)}
                      {#if entry.streaming}<span class="cursor">|</span>{/if}
                    </div>
                  {/if}
                {/if}
              </div>
              <!-- ASK -->
              {#if parsed.asks.length > 0}
                <div class="e-sec e-sec-ask">
                  <div class="e-ask-header">
                    <span class="cue-badge cue-ask">Ask</span>
                  </div>
                  {#each parsed.asks as ask, ai}
                    <button class="e-ask-item" onclick={() => toggleAsk(`${i}-${ai}`)}>
                      <span class="e-ask-chevron">{openAsks[`${i}-${ai}`] ? '▾' : '▸'}</span>
                      <span class="ask-text">{openAsks[`${i}-${ai}`] ? ask.main : shortAsk(ask.main)}</span>
                    </button>
                    {#if openAsks[`${i}-${ai}`]}
                      {#if ask.alts.length > 0}
                        <div class="e-ask-alts">
                          {#each ask.alts as alt}
                            <div class="e-ask-alt">↳ {alt}</div>
                          {/each}
                        </div>
                      {/if}
                    {/if}
                  {/each}
                </div>
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

  /* Active question header */
  .tp-active-question {
    flex-shrink: 0;
    padding: 0.4rem 0.75rem;
    background: #100808;
    border-radius: 0.4rem;
    border-left: 3px solid #dc2626;
  }
  .tp-active-q-row { display: flex; align-items: center; gap: 0.4rem; margin-bottom: 0.2rem; }
  .tp-active-q-text {
    color: #f87171;
    font-size: var(--fs-base);
    line-height: 1.4;
    display: block;
    overflow-wrap: break-word;
    word-break: break-word;
  }

  /* Main card */
  .tp-card {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 0.75rem 1rem;
    background: #07101e;
    border-radius: 0.75rem;
    border: 1px solid #1a2d4a;
    overflow-y: auto;
    min-height: 0;
  }

  /* === Four sections === */
  .tp-sec {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    padding: 0.55rem 0.75rem;
    border-radius: 0.45rem;
    border-left: 3px solid transparent;
    flex-shrink: 0;
  }
  .tp-sec-ack    { background: #110823; border-left-color: #6d28d9; }
  .tp-sec-affirm { background: #071520; border-left-color: #0e7490; }
  .tp-sec-say    { background: #060e0a; border-left-color: #166534; }
  .tp-sec-ask    { background: #0e0700; border-left-color: #92400e; }

  .tp-sec-row {
    display: flex;
    align-items: flex-start;
    gap: 0.5rem;
  }

  /* CUE badges */
  .cue-badge {
    display: inline-block;
    padding: 0.1rem 0.45rem;
    min-width: 6.2rem;
    text-align: center;
    background: #14532d;
    color: #4ade80;
    border-radius: 0.25rem;
    font-size: var(--fs-xs);
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    flex-shrink: 0;
    margin-top: 0.1rem;
  }
  .cue-badge.cue-ask { background: #422006; color: #fbbf24; }
  .cue-badge.cue-affirm { background: #164e63; color: #67e8f9; }
  .cue-badge.cue-ack { background: #2e1065; color: #c084fc; }

  /* Acknowledge text (purple section) */
  .tp-ack-text {
    color: #ffffff;
    font-size: var(--fs-lg);
    line-height: 1.4;
    overflow-wrap: break-word;
    flex: 1;
  }

  /* Affirm text (cyan section) */
  .tp-affirm-text {
    color: #e0f7fa;
    font-size: var(--fs-lg);
    line-height: 1.4;
    overflow-wrap: break-word;
    flex: 1;
  }

  /* Say text */
  .tp-tell {
    color: #ffffff;
    font-size: var(--fs-lg);
    font-weight: 600;
    line-height: 1.5;
    flex: 1;
    overflow-wrap: break-word;
    word-break: break-word;
  }

  /* Expand button (inside Say section for body text) */
  .tp-expand-row { flex-shrink: 0; }
  .tp-expand-btn {
    background: rgba(255,255,255,0.04);
    border: 1px solid #1a3020;
    color: #2d5a3a;
    font-size: var(--fs-sm);
    cursor: pointer;
    padding: 0.2rem 0.6rem;
    border-radius: 0.25rem;
    transition: all 0.15s;
  }
  .tp-expand-btn:hover { border-color: #22543d; color: #4ade80; }

  .tp-body {
    color: #7a9ab8;
    font-size: var(--fs-base);
    line-height: 1.8;
    white-space: pre-wrap;
    border-top: 1px solid #0d2010;
    padding-top: 0.5rem;
    overflow-wrap: break-word;
    word-break: break-word;
  }
  :global(.tp-body strong) { color: #b8cce4; font-weight: 700; }

  /* Cue bullets (inside Say section) */
  .tp-cues { display: flex; flex-direction: column; gap: 0.2rem; border-top: 1px solid #0d2010; padding-top: 0.35rem; }
  .tp-cue-block {
    border-radius: 0.3rem; border: 1px solid #0d2010;
    overflow: hidden; background: #040b06;
  }
  .tp-cue-block.tp-cue-open { border-color: #14532d; }
  .tp-cue-toggle {
    display: flex; align-items: center; gap: 0.4rem;
    width: 100%; padding: 0.28rem 0.5rem;
    background: none; border: none; cursor: pointer; text-align: left;
  }
  .tp-cue-toggle:hover { background: #071a0f; }
  .cue-label-sm {
    font-size: var(--fs-xs); font-weight: 800; text-transform: uppercase;
    letter-spacing: 0.07em; color: #4ade80; flex-shrink: 0;
  }
  .tp-cue-preview {
    flex: 1; font-size: var(--fs-base); color: #3d8c52;
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
  }
  .tp-cue-chevron { font-size: var(--fs-sm); color: #2d6e40; flex-shrink: 0; }
  .tp-cue-body {
    display: flex; flex-direction: column; gap: 0.1rem;
    padding: 0.1rem 0.5rem 0.35rem;
    border-top: 1px solid #0d2010;
  }
  .tp-cue-line {
    /* text is lighter green than the label */
    color: #86efac; font-size: var(--fs-lg); line-height: 1.45;
    padding-left: 0.15rem; overflow-wrap: break-word;
  }
  .tp-cue-speak-btn {
    margin-top: 0.2rem; background: none; border: none;
    color: #166534; font-size: var(--fs-xs); cursor: pointer;
    padding: 0; text-align: left;
  }
  .tp-cue-speak-btn:hover { color: #22c55e; }
  .cue-sentence {
    padding: 0.3rem 0.4rem;
    background: #07101e; border-left: 2px solid #3b82f6;
    border-radius: 0 0.25rem 0.25rem 0; color: #93c5fd;
    font-size: var(--fs-base); line-height: 1.5;
  }
  .cue-loading { color: #334155; }

  /* Ask section items */
  .tp-ask-header {
    display: flex; align-items: center; gap: 0.5rem;
  }
  .tp-ask-hint {
    font-size: var(--fs-xs); color: #4a2500; font-style: italic;
  }
  .tp-ask-item {
    display: flex; align-items: flex-start; gap: 0.4rem;
    width: 100%; background: none; border: none; cursor: pointer;
    text-align: left; padding: 0.2rem 0;
  }
  .tp-ask-item:hover .tp-ask-main { color: #fde68a; }
  .tp-ask-chevron { color: #92400e; font-size: var(--fs-sm); flex-shrink: 0; margin-top: 0.15rem; }
  .tp-ask-main {
    flex: 1; color: #ffffff; font-size: var(--fs-lg);
    overflow-wrap: break-word; line-height: 1.4;
    transition: color 0.15s;
  }
  .tp-ask-alts {
    display: flex; flex-direction: column; gap: 0.2rem;
    padding: 0.2rem 0 0.1rem 1.1rem;
    border-left: 1px solid #3d1a02;
  }
  .tp-ask-alt {
    color: #fbbf24; font-size: var(--fs-base); line-height: 1.4;
    font-style: italic; overflow-wrap: break-word;
  }

  .tp-hint {
    font-size: var(--fs-xs);
    color: #1e293b;
    font-style: italic;
    flex-shrink: 0;
    text-align: center;
  }
  .tp-loading {
    color: #4d94d4; font-style: italic; font-size: var(--fs-base);
  }
  .tp-empty {
    flex: 1; display: flex; align-items: center; justify-content: center;
    color: #1e293b; font-style: italic; font-size: var(--fs-base);
  }

  /* Question tag */
  .tp-tag {
    display: inline-block; padding: 0.08rem 0.4rem; border-radius: 0.2rem;
    font-size: var(--fs-xs); font-weight: 800; text-transform: uppercase;
    letter-spacing: 0.07em; flex-shrink: 0; margin-top: 0.05rem;
  }
  .tp-conf-badge {
    font-size: var(--fs-xs); font-weight: 800; flex-shrink: 0;
    margin-left: auto; padding-top: 0.05rem; font-variant-numeric: tabular-nums;
  }

  /* Story matches */
  .tp-stories {
    display: flex; flex-direction: column; gap: 0.25rem;
    padding: 0.4rem 0.6rem;
    background: #0a0f1a; border: 1px solid #1a2535;
    border-left: 2px solid #7c3aed; border-radius: 0.35rem;
  }
  .tp-stories-label {
    font-size: var(--fs-xs); font-weight: 800; text-transform: uppercase;
    letter-spacing: 0.08em; color: #6d28d9; margin-bottom: 0.1rem;
  }
  .tp-story-chip {
    display: flex; flex-direction: column; gap: 0.1rem;
    padding: 0.25rem 0.4rem; background: #100d1a;
    border-radius: 0.25rem; border: 1px solid #2d1f4a;
  }
  .tp-story-title { font-size: var(--fs-sm); font-weight: 700; color: #c084fc; }
  .tp-story-result { font-size: var(--fs-sm); color: #7a6a8a; font-style: italic; line-height: 1.3; }

  .tp-redflag {
    display: flex; flex-direction: column; gap: 0.15rem;
    padding: 0.4rem 0.6rem; background: #1a0800;
    border-left: 3px solid #b45309; border-radius: 0.3rem; flex-shrink: 0;
  }
  .redflag-cat {
    font-size: var(--fs-xs); font-weight: 800; text-transform: uppercase;
    letter-spacing: 0.07em; color: #f59e0b;
  }
  .redflag-note { font-size: var(--fs-sm); color: #94a3b8; line-height: 1.4; font-style: italic; }

  .cursor { animation: blink 1s step-end infinite; }
  @keyframes blink { 50% { opacity: 0; } }
  .dots { animation: ellipsis 1.5s infinite; }
  @keyframes ellipsis {
    0% { content: '.'; } 33% { content: '..'; } 66% { content: '...'; }
  }

  /* === Standard panel mode === */
  .suggestion-panel { height: 100%; display: flex; flex-direction: column; }
  .panel-header {
    display: flex; align-items: flex-start; justify-content: space-between;
    margin-bottom: 0.75rem; gap: 0.5rem;
  }
  .header-left { display: flex; flex-direction: column; gap: 0.15rem; }
  .glance-hint { font-size: var(--fs-xs); color: #475569; font-style: italic; }
  h3 {
    font-size: var(--fs-base); font-weight: 600; color: #94a3b8;
    text-transform: uppercase; letter-spacing: 0.05em; margin: 0;
  }
  .clear-btn {
    padding: 0.15rem 0.6rem; background: transparent;
    border: 1px solid #334155; border-radius: 0.25rem;
    color: #64748b; font-size: var(--fs-sm); cursor: pointer;
  }
  .clear-btn:hover { border-color: #64748b; color: #94a3b8; }

  .entries { flex: 1; overflow-y: auto; display: flex; flex-direction: column; gap: 0.75rem; }

  .entry {
    padding: 0.65rem; background: #0f1e33; border-radius: 0.5rem;
    border-left: 3px solid #334155; display: flex; flex-direction: column;
    gap: 0.3rem; opacity: 0.6; transition: opacity 0.2s;
  }
  .entry.latest { border-left-color: #4ade80; opacity: 1; }

  .question-row { display: flex; align-items: flex-start; gap: 0.5rem; flex-wrap: wrap; }
  .q-num-badge {
    flex-shrink: 0; padding: 0.12rem 0.45rem; background: #14532d; color: #4ade80;
    border-radius: 9999px; font-size: var(--fs-xs); font-weight: 800;
    text-transform: uppercase; margin-top: 0.1rem; letter-spacing: 0.04em;
  }
  .question-text {
    color: #f87171; font-size: var(--fs-base); margin: 0;
    line-height: 1.4; overflow-wrap: break-word; word-break: break-word;
  }
  .entry-tag {
    display: inline-block; padding: 0.05rem 0.35rem; border-radius: 0.2rem;
    font-size: var(--fs-xs); font-weight: 800; text-transform: uppercase;
    letter-spacing: 0.06em; flex-shrink: 0; align-self: flex-start; margin-top: 0.15rem;
  }
  .entry-redflag {
    display: flex; flex-direction: column; gap: 0.1rem;
    padding: 0.3rem 0.5rem; background: #1a0800;
    border-left: 2px solid #b45309; border-radius: 0.25rem; margin-bottom: 0.1rem;
  }

  /* Standard panel section blocks */
  .e-sec {
    display: flex; flex-direction: column; gap: 0.35rem;
    padding: 0.45rem 0.6rem; border-radius: 0.4rem;
    border-left: 2px solid transparent;
  }
  .e-sec-ack    { background: #110823; border-left-color: #6d28d9; }
  .e-sec-affirm { background: #071520; border-left-color: #0e7490; }
  .e-sec-say    { background: #060e0a; border-left-color: #166534; }
  .e-sec-ask    { background: #0e0700; border-left-color: #92400e; }
  .e-sec-row { display: flex; align-items: flex-start; gap: 0.5rem; }

  .affirm-text {
    color: #ffffff; font-size: var(--fs-lg); line-height: 1.4;
    flex: 1; overflow-wrap: break-word;
  }
  .tell-text {
    color: #ffffff; font-size: var(--fs-lg); font-weight: 600;
    line-height: 1.5; flex: 1; overflow-wrap: break-word; word-break: break-word;
  }

  .expand-btn {
    background: none; border: none; color: #1e4a2a;
    font-size: var(--fs-sm); cursor: pointer; padding: 0; align-self: flex-start;
  }
  .expand-btn:hover { color: #4ade80; }

  .body-text {
    color: #7a9ab8; line-height: 1.7; white-space: pre-wrap;
    font-size: var(--fs-base); border-top: 1px solid #0d2010; padding-top: 0.4rem;
    overflow-wrap: break-word; word-break: break-word;
  }
  :global(.body-text strong) { color: #b8cce4; font-weight: 700; }

  .e-ask-header { display: flex; align-items: center; }
  .e-ask-item {
    display: flex; align-items: flex-start; gap: 0.35rem;
    width: 100%; background: none; border: none; cursor: pointer;
    text-align: left; padding: 0.15rem 0;
  }
  .e-ask-item:hover .ask-text { color: #fde68a; }
  .e-ask-chevron { color: #92400e; font-size: var(--fs-sm); flex-shrink: 0; margin-top: 0.1rem; }
  .ask-text {
    flex: 1; color: #ffffff; font-size: var(--fs-lg);
    overflow-wrap: break-word; line-height: 1.4; transition: color 0.15s;
  }
  .e-ask-alts {
    display: flex; flex-direction: column; gap: 0.15rem;
    padding: 0.15rem 0 0.05rem 1rem; border-left: 1px solid #3d1a02;
  }
  .e-ask-alt {
    color: #fbbf24; font-size: var(--fs-base); line-height: 1.4;
    font-style: italic; overflow-wrap: break-word;
  }

  .loading { color: #60a5fa; font-style: italic; font-size: var(--fs-base); }
  .empty {
    color: #475569; font-style: italic; font-size: var(--fs-base);
    text-align: center; padding: 2rem 1rem;
  }
</style>
