<script lang="ts">
  import type { SuggestionEntry } from '../lib/types';
  import { TAG_CONFIG } from '../lib/questionTagger';
  import { parseSuggestion, parseCues } from '../lib/parseSuggestion';
  import PanelHeader from './PanelHeader.svelte';

  // Expand-cue state: cue text → { sentence, loading }
  let expandedCues = $state<Record<string, { sentence: string; loading: boolean }>>({});
  // Open/collapsed state for cue bullet blocks
  let openCues = $state<Record<string, boolean>>({});

  function toggleCueOpen(key: string) {
    openCues = { ...openCues, [key]: !openCues[key] };
  }

  async function expandCue(question: string, cue: string) {
    if (expandedCues[cue]?.sentence) return; // already fetched — keep cached result
    if (expandedCues[cue]?.loading) return;  // fetch already in flight
    expandedCues = { ...expandedCues, [cue]: { sentence: '', loading: true } };
    try {
      const r = await fetch('/api/expand-cue', {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ question, cue }),
      });
      const d = r.ok ? await r.json() : null;
      const sentence = d?.sentence?.trim() ?? '';
      expandedCues = { ...expandedCues, [cue]: { sentence: sentence || cue, loading: false } };
    } catch { expandedCues = { ...expandedCues, [cue]: { sentence: cue, loading: false } }; }
  }

  const { suggestions, onClear, teleprompter = false, jumpSignal = null, cueExpandSignal = null, onPinnedChange } = $props<{
    suggestions: SuggestionEntry[];
    onClear: () => void;
    teleprompter?: boolean;
    jumpSignal?: { idx: number; key: number } | null;
    cueExpandSignal?: { cueIdx: number; key: number } | null;
    onPinnedChange?: (pinned: boolean) => void;
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

  $effect(() => {
    onPinnedChange?.(historyIndex === -1);
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

  // Pre-fetch expanded sentences as soon as a suggestion finishes streaming
  // so they're ready before the user clicks, regardless of capture state
  $effect(() => {
    if (!current || current.streaming) return;
    const parsed = parseSuggestion(current.suggestion);
    const cues = parseCues(parsed.body);
    for (const cue of cues) {
      expandCue(current.question, cue.text);
    }
    for (const ask of parsed.asks) {
      expandCue(current.question, '[Ask] ' + ask.topic);
    }
  });

  function wordCount(text: string): number {
    return text.trim().split(/\s+/).filter(Boolean).length;
  }

  function estimateSecs(parsed: ReturnType<typeof parseSuggestion>): number {
    const words = wordCount([parsed.acknowledge, parsed.bridge, parsed.solve, parsed.tell, parsed.close].filter(Boolean).join(' '));
    return Math.round((words / 130) * 60);
  }

  // Collapsed state per question entry (collapsed = body hidden)
  let collapsed = $state<boolean[]>([]);
  $effect(() => {
    if (collapsed.length < suggestions.length) {
      collapsed = [...collapsed, ...new Array(suggestions.length - collapsed.length).fill(false)];
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
            {#key current.confidenceScore}
              <span class="tp-conf-badge"
                class:conf-good-pulse={current.confidenceScore >= 75}
                style="color: {current.confidenceScore >= 70 ? '#4ade80' : current.confidenceScore >= 40 ? '#f59e0b' : '#f87171'}">
                {current.confidenceScore}%
              </span>
            {/key}
          {/if}
        </div>
        <span class="tp-active-q-text">"{current.question}"</span>
      </div>

      {@const parsed = parseSuggestion(current.suggestion)}
      <div class="tp-card">
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
              <span class="cue-badge cue-ack">Acknowledge</span>
              <span class="tp-ack-text">{parsed.acknowledge}</span>
            </div>
          {/if}

          <!-- SOLVE section -->
          {#if parsed.solve}
            <div class="tp-sec tp-sec-solve">
              <span class="cue-badge cue-solve">Solve</span>
              <span class="tp-solve-text">{parsed.solve}</span>
            </div>
          {/if}

          <!-- BRIDGE section -->
          {#if parsed.bridge}
            <div class="tp-sec tp-sec-bridge">
              <span class="cue-badge cue-bridge">Bridge</span>
              <span class="tp-bridge-text">{parsed.bridge}</span>
            </div>
          {/if}

          <!-- ANSWER section (includes cue points) -->
          <div class="tp-sec tp-sec-say">
            <div class="tp-sec-header">
              <span class="cue-badge">{parsed.cue}</span>
              {#if parsed.tell && !current.streaming}
                {@const secs = estimateSecs(parsed)}
                <span class="tp-time-est">~{secs < 60 ? secs + 's' : Math.floor(secs/60) + 'm ' + (secs%60) + 's'}</span>
              {/if}
            </div>
            <span class="tp-tell">{parsed.tell}{#if current.streaming && !parsed.body}<span class="cursor">|</span>{/if}</span>
            {#if parsed.body}
              {@const cues = parseCues(parsed.body)}
              {#if cues.length > 0}
                <div class="tp-cues">
                  {#each cues as cue}
                    {@const isOpen = !!openCues[cue.text]}
                    {@const _km = cue.text.match(/^\[([^\]]+)\]\s*(.*)/s)}
                    <div class="tp-cue-block" class:tp-cue-open={isOpen}>
                      <button class="tp-cue-toggle" onclick={() => { const opening = !isOpen; toggleCueOpen(cue.text); if (opening) expandCue(current.question, cue.text); }}>
                        <span class="cue-label-sm" class:cue-label-example={cue.typeTag === 'Example' || cue.typeTag === 'Story' || cue.label === 'Example' || cue.label === 'Story'}>{cue.typeTag || (cue.label === 'General' ? 'Point' : cue.label)}</span>
                        <span class="tp-cue-preview">{#if _km}<span class="tp-cue-kw">{_km[1]}</span> {_km[2]}{:else}{cue.text}{/if}</span>
                        <span class="tp-cue-chevron">{isOpen ? '▾' : '▸'}</span>
                      </button>
                      {#if isOpen}
                        <div class="tp-cue-body">
                          {#if expandedCues[cue.text]?.loading}
                            <div class="cue-sentence cue-loading">…</div>
                          {:else if expandedCues[cue.text]?.sentence}
                            <div class="cue-sentence">{#each expandedCues[cue.text].sentence.split(/(?<=[.!?])\s+/) as s}{s.trim()}<br/>{/each}</div>
                          {:else}
                            <div class="cue-sentence cue-loading">Loading…</div>
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

          <!-- CLOSE section -->
          {#if parsed.close}
            <div class="tp-sec tp-sec-close">
              <span class="cue-badge cue-close">Close</span>
              <span class="tp-close-text">{parsed.close}</span>
            </div>
          {/if}

          <!-- ASK section — always show expanded question text -->
          {#if parsed.asks.length > 0}
            <div class="tp-sec tp-sec-ask">
              <span class="cue-badge cue-ask">Ask</span>
              <div class="tp-ask-list">
                {#each parsed.asks as ask, i}
                  {@const askKey = '[Ask] ' + ask.topic}
                  <div class="tp-ask-item">
                    <span class="cue-label-sm cue-label-ask">Q{i + 1}</span>
                    <div class="tp-ask-content">
                      <span class="tp-ask-topic">{ask.topic}</span>
                      {#if expandedCues[askKey]?.sentence}
                        <span class="tp-ask-question">{expandedCues[askKey].sentence}</span>
                      {:else if expandedCues[askKey]?.loading}
                        <span class="tp-ask-question cue-loading">…</span>
                      {/if}
                    </div>
                  </div>
                {/each}
              </div>
            </div>
          {/if}

        {/if}
      </div>
    {:else}
      <div class="tp-empty">Waiting for a question...</div>
    {/if}

    <span class="tp-hint">Acknowledge → Bridge → Answer → Close · Ask = follow-up question</span>
  </div>

{:else}
  <!-- Standard panel mode -->
  <div class="suggestion-panel">
    <PanelHeader
      title="AI Suggestions"
      hint="Say = speak it · Ask = follow-up questions"
      actionLabel={suggestions.length > 0 ? 'Clear' : undefined}
      onAction={onClear}
    />

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
              <button class="entry-collapse-btn" onclick={() => { collapsed[i] = !collapsed[i]; collapsed = [...collapsed]; }} title={collapsed[i] ? 'Expand' : 'Collapse'}>{collapsed[i] ? '▸' : '▾'}</button>
            </div>
            {#if !collapsed[i]}
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
                  <span class="cue-badge cue-ack">Acknowledge</span>
                  <span class="affirm-text">{parsed.acknowledge}</span>
                </div>
              {/if}
              <!-- SOLVE -->
              {#if parsed.solve}
                <div class="e-sec e-sec-solve">
                  <span class="cue-badge cue-solve">Solve</span>
                  <span class="affirm-text">{parsed.solve}</span>
                </div>
              {/if}
              <!-- BRIDGE -->
              {#if parsed.bridge}
                <div class="e-sec e-sec-bridge">
                  <span class="cue-badge cue-bridge">Bridge</span>
                  <span class="affirm-text">{parsed.bridge}</span>
                </div>
              {/if}
              <!-- ANSWER -->
              <div class="e-sec e-sec-say">
                <div class="e-sec-header">
                  <span class="cue-badge" class:cue-ask={parsed.cue === 'Ask'}>{parsed.cue}</span>
                  {#if parsed.tell && !entry.streaming}
                    {@const secs = estimateSecs(parsed)}
                    <span class="tp-time-est">~{secs < 60 ? secs + 's' : Math.floor(secs/60) + 'm ' + (secs%60) + 's'}</span>
                  {/if}
                </div>
                <span class="tell-text">{parsed.tell}{#if entry.streaming && !parsed.body}<span class="cursor">|</span>{/if}</span>
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
              <!-- CLOSE -->
              {#if parsed.close}
                <div class="e-sec e-sec-close">
                  <span class="cue-badge cue-close">Close</span>
                  <span class="affirm-text">{parsed.close}</span>
                </div>
              {/if}
              <!-- ASK — always visible -->
              {#if parsed.asks.length > 0}
                <div class="e-sec e-sec-ask">
                  <span class="cue-badge cue-ask">Ask</span>
                  <div class="tp-ask-list">
                    {#each parsed.asks as ask, ai}
                      {@const askKey = '[Ask] ' + ask.topic}
                      <div class="tp-ask-item">
                        <span class="cue-label-sm cue-label-ask">Q{ai + 1}</span>
                        <div class="tp-ask-content">
                          <span class="tp-ask-topic">{ask.topic}</span>
                          {#if expandedCues[askKey]?.sentence}
                            <span class="tp-ask-question">{expandedCues[askKey].sentence}</span>
                          {:else if expandedCues[askKey]?.loading}
                            <span class="tp-ask-question cue-loading">…</span>
                          {/if}
                        </div>
                      </div>
                    {/each}
                  </div>
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
  .tp-sec-ack    { background: var(--bg-ack);    border-left-color: var(--border-ack); }
  .tp-sec-solve  { background: #071a1a;          border-left-color: #0e7490; }
  .tp-sec-bridge { background: #0d0d07;          border-left-color: #78716c; }
  .tp-sec-say    { background: var(--bg-say);    border-left-color: var(--border-say); }
  .tp-sec-close  { background: #080d1a;          border-left-color: #3b82f6; }
  .tp-sec-ask    { background: var(--bg-ask);    border-left-color: var(--border-ask); }

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
  .cue-badge.cue-ask    { background: #422006; color: #fbbf24; }
  .cue-badge.cue-ack    { background: #2e1065; color: #c084fc; }
  .cue-badge.cue-solve  { background: #164e63; color: #67e8f9; }
  .cue-badge.cue-bridge { background: #292524; color: #a8a29e; }
  .cue-badge.cue-close  { background: #1e3a5f; color: #93c5fd; }

  /* Acknowledge text (purple section) */
  .tp-ack-text {
    color: #ffffff;
    font-size: var(--fs-lg);
    line-height: 1.4;
    overflow-wrap: break-word;
    flex: 1;
  }

  /* Solve text (teal section) */
  .tp-solve-text {
    color: #ffffff;
    font-size: var(--fs-lg);
    line-height: 1.4;
    overflow-wrap: break-word;
    flex: 1;
  }

  /* Bridge text (stone section) */
  .tp-bridge-text {
    color: #d6d3d1;
    font-size: var(--fs-lg);
    line-height: 1.4;
    overflow-wrap: break-word;
    flex: 1;
    font-style: italic;
  }

  /* Close text (blue section) */
  .tp-close-text {
    color: #ffffff;
    font-size: var(--fs-lg);
    line-height: 1.4;
    overflow-wrap: break-word;
    flex: 1;
  }

  /* Answer header row with time estimate */
  .tp-sec-header { display: flex; align-items: center; gap: 0.5rem; }
  .e-sec-header  { display: flex; align-items: center; gap: 0.5rem; }
  .tp-time-est {
    font-size: var(--fs-xs); color: #334155;
    font-variant-numeric: tabular-nums; font-style: italic;
  }

  /* Ask inline list */
  .tp-ask-list { display: flex; flex-direction: column; gap: 0.35rem; }
  .tp-ask-item { display: flex; align-items: flex-start; gap: 0.4rem; }
  .tp-ask-content { display: flex; flex-direction: column; gap: 0.15rem; flex: 1; }
  .tp-ask-topic { font-size: var(--fs-sm); color: #7c4a1a; font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; }
  .tp-ask-question { font-size: var(--fs-lg); color: #fde68a; line-height: 1.4; overflow-wrap: break-word; }

  /* Say text */
  .tp-tell {
    color: #ffffff;
    font-size: var(--fs-lg);
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
  .tp-cue-block-ask { border-color: #2d1200; background: #060300; }
  .tp-cue-block-ask.tp-cue-open { border-color: #78350f; }
  .tp-cue-toggle {
    display: flex; align-items: center; gap: 0.4rem;
    width: 100%; padding: 0.28rem 0.5rem;
    background: none; border: none; cursor: pointer; text-align: left;
  }
  .tp-cue-toggle:hover { background: #071a0f; }
  .cue-label-sm {
    font-size: var(--fs-sm); font-weight: 800; text-transform: uppercase;
    letter-spacing: 0.07em; color: #4ade80; flex-shrink: 0;
  }
  .tp-cue-preview {
    flex: 1; font-size: var(--fs-lg); color: #3d8c52;
    white-space: normal; overflow-wrap: break-word; word-break: break-word;
  }
  .tp-cue-kw {
    font-weight: 700; color: #86efac;
  }
  .tp-cue-chevron { font-size: var(--fs-sm); color: #2d6e40; flex-shrink: 0; }
  .tp-cue-body {
    display: flex; flex-direction: column; gap: 0.1rem;
    padding: 0.1rem 0.5rem 0.35rem;
    border-top: 1px solid #0d2010;
  }
  .cue-sentence {
    padding: 0.3rem 0.4rem;
    background: #061209; border-left: 2px solid #22c55e;
    border-radius: 0 0.25rem 0.25rem 0; color: #f1f5f9;
    font-size: var(--fs-lg); line-height: 1.5; font-weight: 400;
    overflow-wrap: break-word;
  }
  .cue-label-example { }
  .cue-loading { color: #334155; }

  /* Ask cue-block theming (amber) */
  .cue-label-ask { color: #fbbf24 !important; }
  .tp-ask-preview {
    color: #7c4a1a !important;
    white-space: normal !important;
    overflow: visible !important;
    text-overflow: unset !important;
  }
  .ask-sentence {
    padding: 0.3rem 0.4rem; background: #060300; border-left: 2px solid #92400e;
    border-radius: 0 0.25rem 0.25rem 0; color: #f1f5f9;
    font-size: var(--fs-lg); line-height: 1.5; font-weight: 400; overflow-wrap: break-word;
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
  .conf-good-pulse { animation: conf-glow 1.5s ease-out forwards; }
  @keyframes conf-glow {
    0%   { text-shadow: 0 0 10px #4ade80, 0 0 20px #4ade8055; transform: scale(1.2); }
    100% { text-shadow: none; transform: scale(1); }
  }
  .tp-conf-badge {
    font-size: var(--fs-xs); font-weight: 800; flex-shrink: 0;
    margin-left: auto; padding-top: 0.05rem; font-variant-numeric: tabular-nums;
  }


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
  .e-sec-ack    { background: var(--bg-ack);    border-left-color: var(--border-ack); }
  .e-sec-solve  { background: #071a1a;          border-left-color: #0e7490; }
  .e-sec-bridge { background: #0d0d07;          border-left-color: #78716c; }
  .e-sec-say    { background: var(--bg-say);    border-left-color: var(--border-say); }
  .e-sec-close  { background: #080d1a;          border-left-color: #3b82f6; }
  .e-sec-ask    { background: var(--bg-ask);    border-left-color: var(--border-ask); }
  .affirm-text, .tell-text {
    color: #f1f5f9; font-size: var(--fs-lg); font-weight: 400;
    line-height: 1.5; overflow-wrap: break-word; word-break: break-word;
    margin-top: 0.15rem;
  }

  .entry-collapse-btn {
    margin-left: auto; background: none; border: none;
    color: #334155; font-size: var(--fs-sm); cursor: pointer; padding: 0 0.2rem;
    flex-shrink: 0; line-height: 1;
  }
  .entry-collapse-btn:hover { color: #64748b; }

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

  .loading { color: #60a5fa; font-style: italic; font-size: var(--fs-base); }
  .empty {
    color: #475569; font-style: italic; font-size: var(--fs-base);
    text-align: center; padding: 2rem 1rem;
  }
</style>
