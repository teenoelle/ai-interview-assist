<script lang="ts">
  import type { SuggestionEntry } from '../lib/types';
  import { TAG_CONFIG } from '../lib/questionTagger';
  import { parseSuggestion, parseCues, getAnswerType } from '../lib/parseSuggestion';
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
    // Auto-open all strategies
    parsed.strategies.forEach((_, si) => {
      openCues[`strat-${currentIndex}-${si}`] = true;
    });
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
    const words = wordCount([
      parsed.acknowledge, parsed.solve, parsed.bridge, parsed.tell, parsed.close,
      parsed.present, parsed.past, parsed.future,
      parsed.company, parsed.role, parsed.self,
      parsed.direction, parsed.alignment, parsed.contribution,
    ].filter(Boolean).join(' '));
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

  const totalCount = $derived(suggestions.length);
  const currentIndex = $derived(historyIndex === -1 ? totalCount - 1 : historyIndex);
  const current = $derived(currentIndex >= 0 && currentIndex < totalCount ? suggestions[currentIndex] : null);

  function jumpTo(i: number) {
    historyIndex = i;
    lastSeenCount = suggestions.length;
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

  // Active mode per entry index (compound | primary | secondary)
  // Defaults to 'compound' for compound questions, 'primary' otherwise
  let activeModes = $state<Record<number, 'compound' | 'primary' | 'secondary'>>({});

  function getActiveMode(i: number, entry: import('../lib/types').SuggestionEntry): 'compound' | 'primary' | 'secondary' {
    if (!entry.secondaryTag) return 'primary';
    return activeModes[i] ?? 'compound';
  }

  function setActiveMode(i: number, mode: 'compound' | 'primary' | 'secondary') {
    activeModes = { ...activeModes, [i]: mode };
  }

  function getModeContent(mode: 'compound' | 'primary' | 'secondary', entry: import('../lib/types').SuggestionEntry): string {
    if (mode === 'compound') return entry.compoundSuggestion ?? '';
    if (mode === 'secondary') return entry.secondarySuggestion ?? '';
    return entry.suggestion;
  }

  function getModeStreaming(mode: 'compound' | 'primary' | 'secondary', entry: import('../lib/types').SuggestionEntry): boolean {
    if (mode === 'compound') return entry.compoundStreaming ?? false;
    if (mode === 'secondary') return entry.secondaryStreaming ?? false;
    return entry.streaming;
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
          {#if current.provider && !current.streaming}
            <span class="tp-provider-badge" class:tp-provider-local={current.providerLocal} title={current.providerLocal ? 'Local model' : 'API'}>{current.provider}</span>
          {/if}
        </div>
        <span class="tp-active-q-text">"{current.question}"</span>
      </div>

      {@const parsed = parseSuggestion(tpSuggestion)}
      {@const bodyCues = parsed.body ? parseCues(parsed.body) : []}
      {@const ansType = getAnswerType(parsed, tpMode === 'compound' ? undefined : current.tag)}
      {#if ansType.framework && !tpStreaming}
        <div class="tp-breadcrumb">
          {#if current.tag}{@const tc = TAG_CONFIG[current.tag]}<span class="tp-bc-q" style="color:{tc.color}">{tc.label}</span><span class="tp-bc-sep">·</span>{/if}
          <span class="tp-bc-a">{ansType.label}</span>
        </div>
      {/if}
      {#if current.secondaryTag}
        {@const tc1 = TAG_CONFIG[current.tag ?? 'general']}
        {@const tc2 = TAG_CONFIG[current.secondaryTag]}
        {@const activeMode = getActiveMode(currentIndex, current)}
        <div class="mode-tabs">
          <button class="mode-tab" class:mode-tab-active={activeMode === 'compound'}
                  onclick={() => setActiveMode(currentIndex, 'compound')}>
            <span class="mode-tab-dot"
              class:mode-tab-streaming={current.compoundStreaming}
              class:mode-tab-ready={!current.compoundStreaming && !!current.compoundSuggestion}
            ></span>
            Compound
          </button>
          <button class="mode-tab" class:mode-tab-active={activeMode === 'primary'}
                  onclick={() => setActiveMode(currentIndex, 'primary')}>
            <span class="mode-tab-dot"
              class:mode-tab-streaming={current.streaming}
              class:mode-tab-ready={!current.streaming && !!current.suggestion}
            ></span>
            <span style="color:{tc1.color}">{tc1.label}</span>
          </button>
          <button class="mode-tab" class:mode-tab-active={activeMode === 'secondary'}
                  onclick={() => setActiveMode(currentIndex, 'secondary')}>
            <span class="mode-tab-dot"
              class:mode-tab-streaming={current.secondaryStreaming}
              class:mode-tab-ready={!current.secondaryStreaming && !!current.secondarySuggestion}
            ></span>
            <span style="color:{tc2.color}">{tc2.label}</span>
          </button>
        </div>
      {/if}

      {@const tpMode = current.secondaryTag ? getActiveMode(currentIndex, current) : 'primary'}
      {@const tpSuggestion = getModeContent(tpMode, current)}
      {@const tpStreaming = getModeStreaming(tpMode, current)}

      <div class="tp-card">
        {#if current.redFlag}
          <div class="tp-redflag">
            <span class="redflag-cat">{current.redFlag.category}</span>
            <span class="redflag-note">{current.redFlag.coachingNote}</span>
          </div>
        {/if}

        {#if tpStreaming && !tpSuggestion}
          <span class="tp-loading">Generating<span class="dots">...</span></span>
        {:else}
          {@const isIntro    = !!(parsed.present || parsed.thread || parsed.past || parsed.future)}
          {@const isMotiv    = !!(parsed.company || parsed.role || parsed.self)}
          {@const isFutureTy = !!(parsed.direction || parsed.alignment || parsed.contribution)}
          {@const isClosing  = parsed.asks.length >= 3 && !parsed.acknowledge && !parsed.present && !parsed.company && !parsed.direction && !parsed.tell}

          {#if isIntro}
            <!-- INTRODUCTION: Summary → Thread → Story → Next → Close -->
            {#if parsed.present}
              <div class="tp-sec tp-sec-present">
                <span class="cue-badge cue-present">Summary</span>
                <span class="tp-narrative-text">{parsed.present}{#if tpStreaming && !parsed.thread}<span class="cursor">|</span>{/if}</span>
              </div>
            {/if}
            {#if parsed.thread}
              <div class="tp-sec tp-sec-thread">
                <span class="cue-badge cue-thread">Thread</span>
                <span class="tp-narrative-text">{parsed.thread}{#if tpStreaming && !parsed.transition1 && !parsed.past}<span class="cursor">|</span>{/if}</span>
              </div>
            {/if}
            {#if parsed.transition1}<div class="tp-transition">{parsed.transition1}</div>{/if}
            {#if parsed.past}
              <div class="tp-sec tp-sec-past">
                <span class="cue-badge cue-past">Story</span>
                <span class="tp-narrative-text">{parsed.past}{#if tpStreaming && !parsed.transition2 && !parsed.future}<span class="cursor">|</span>{/if}</span>
              </div>
            {/if}
            {#if parsed.transition2}<div class="tp-transition">{parsed.transition2}</div>{/if}
            {#if parsed.future}
              <div class="tp-sec tp-sec-future">
                <span class="cue-badge cue-future">Next</span>
                <span class="tp-narrative-text">{parsed.future}{#if tpStreaming && !parsed.transition3 && !parsed.close}<span class="cursor">|</span>{/if}</span>
              </div>
            {/if}
            {#if parsed.transition3}<div class="tp-transition">{parsed.transition3}</div>{/if}
            {#if parsed.close}<div class="tp-sec tp-sec-close"><span class="cue-badge cue-close">Close</span><span class="tp-close-text">{parsed.close}</span></div>{/if}
            {#if parsed.asks.length > 0}<div class="tp-sec tp-sec-ask"><span class="cue-badge cue-ask">Ask</span><div class="tp-ask-list">{#each parsed.asks as ask}<div class="tp-ask-item"><div class="tp-ask-content"><span class="tp-ask-topic">{ask.topic}</span><span class="tp-ask-question">{ask.question}</span>{#if ask.followUp}<span class="tp-ask-followup">↳ {ask.followUp}</span>{/if}</div></div>{/each}</div></div>{/if}

          {:else if isMotiv}
            <!-- MOTIVATION: Company → Role → Self -->
            {#if parsed.company}
              <div class="tp-sec tp-sec-company">
                <span class="cue-badge cue-company">Company</span>
                <span class="tp-narrative-text">{parsed.company}{#if current.streaming && !parsed.transition1 && !parsed.role}<span class="cursor">|</span>{/if}</span>
              </div>
            {/if}
            {#if parsed.transition1}<div class="tp-transition">{parsed.transition1}</div>{/if}
            {#if parsed.role}
              <div class="tp-sec tp-sec-role">
                <span class="cue-badge cue-role">Role</span>
                <span class="tp-narrative-text">{parsed.role}{#if current.streaming && !parsed.transition2 && !parsed.self}<span class="cursor">|</span>{/if}</span>
              </div>
            {/if}
            {#if parsed.transition2}<div class="tp-transition">{parsed.transition2}</div>{/if}
            {#if parsed.self}
              <div class="tp-sec tp-sec-self">
                <span class="cue-badge cue-self">Self</span>
                <span class="tp-narrative-text">{parsed.self}{#if tpStreaming && !parsed.transition3 && !parsed.close}<span class="cursor">|</span>{/if}</span>
              </div>
            {/if}
            {#if parsed.transition3}<div class="tp-transition">{parsed.transition3}</div>{/if}
            {#if parsed.close}<div class="tp-sec tp-sec-close"><span class="cue-badge cue-close">Close</span><span class="tp-close-text">{parsed.close}</span></div>{/if}
            {#if parsed.asks.length > 0}<div class="tp-sec tp-sec-ask"><span class="cue-badge cue-ask">Ask</span><div class="tp-ask-list">{#each parsed.asks as ask}<div class="tp-ask-item"><div class="tp-ask-content"><span class="tp-ask-topic">{ask.topic}</span><span class="tp-ask-question">{ask.question}</span>{#if ask.followUp}<span class="tp-ask-followup">↳ {ask.followUp}</span>{/if}</div></div>{/each}</div></div>{/if}

          {:else if isFutureTy}
            <!-- FUTURE: Direction → Alignment → Contribution -->
            {#if parsed.direction}
              <div class="tp-sec tp-sec-direction">
                <span class="cue-badge cue-direction">Direction</span>
                <span class="tp-narrative-text">{parsed.direction}{#if current.streaming && !parsed.transition1 && !parsed.alignment}<span class="cursor">|</span>{/if}</span>
              </div>
            {/if}
            {#if parsed.transition1}<div class="tp-transition">{parsed.transition1}</div>{/if}
            {#if parsed.alignment}
              <div class="tp-sec tp-sec-alignment">
                <span class="cue-badge cue-alignment">Alignment</span>
                <span class="tp-narrative-text">{parsed.alignment}{#if current.streaming && !parsed.transition2 && !parsed.contribution}<span class="cursor">|</span>{/if}</span>
              </div>
            {/if}
            {#if parsed.transition2}<div class="tp-transition">{parsed.transition2}</div>{/if}
            {#if parsed.contribution}
              <div class="tp-sec tp-sec-contribution">
                <span class="cue-badge cue-contribution">Contribution</span>
                <span class="tp-narrative-text">{parsed.contribution}{#if tpStreaming && !parsed.transition3 && !parsed.close}<span class="cursor">|</span>{/if}</span>
              </div>
            {/if}
            {#if parsed.transition3}<div class="tp-transition">{parsed.transition3}</div>{/if}
            {#if parsed.close}<div class="tp-sec tp-sec-close"><span class="cue-badge cue-close">Close</span><span class="tp-close-text">{parsed.close}</span></div>{/if}
            {#if parsed.asks.length > 0}<div class="tp-sec tp-sec-ask"><span class="cue-badge cue-ask">Ask</span><div class="tp-ask-list">{#each parsed.asks as ask}<div class="tp-ask-item"><div class="tp-ask-content"><span class="tp-ask-topic">{ask.topic}</span><span class="tp-ask-question">{ask.question}</span>{#if ask.followUp}<span class="tp-ask-followup">↳ {ask.followUp}</span>{/if}</div></div>{/each}</div></div>{/if}

          {:else if isClosing}
            <!-- CLOSING: Featured question cards -->
            <div class="tp-closing-wrap">
              <div class="tp-closing-header">Questions to Ask</div>
              {#each parsed.asks as ask, ai}
                <div class="tp-closing-card">
                  <span class="tp-closing-num">{ai + 1}</span>
                  <div class="tp-closing-content">
                    <span class="tp-closing-topic">{ask.topic}</span>
                    <span class="tp-closing-question">{ask.question}</span>
                    {#if ask.followUp}<span class="tp-ask-followup">↳ {ask.followUp}</span>{/if}
                  </div>
                </div>
              {/each}
            </div>

          {:else}
            <!-- BEHAVIORAL / COMPETENCY / STRENGTHS: existing layout -->
            {#if parsed.acknowledge}
              <div class="tp-sec tp-sec-ack">
                <span class="cue-badge cue-ack">Acknowledge</span>
                <span class="tp-ack-text">{parsed.acknowledge}</span>
              </div>
            {/if}
            {#if parsed.solve}
              <div class="tp-sec tp-sec-solve">
                <span class="cue-badge cue-solve">Solve</span>
                <span class="tp-solve-text">{parsed.solve}</span>
              </div>
            {/if}
            {#if parsed.bridge}
              <div class="tp-sec tp-sec-bridge">
                <span class="cue-badge cue-bridge">Bridge</span>
                <span class="tp-bridge-text">{parsed.bridge}</span>
              </div>
            {/if}

          <!-- ANSWER section (includes cue points) -->
          {@const _sayPi = bodyCues.some(c => c.label === 'Pivot' || c.typeTag === 'Pivot')}
          {@const _sayEx = bodyCues.some(c => c.label === 'Example' || c.typeTag === 'Example')}
          <div class="tp-sec tp-sec-say" class:tp-sec-say-example={_sayEx} class:tp-sec-say-pivot={_sayPi}>
            <div class="tp-sec-header">
              <span class="cue-badge">{parsed.cue}</span>
              {#if parsed.tell && !tpStreaming}
                {@const secs = estimateSecs(parsed)}
                <span class="tp-time-est">~{secs < 60 ? secs + 's' : Math.floor(secs/60) + 'm ' + (secs%60) + 's'}</span>
                {#if _sayEx}<span class="cue-type-tag cue-type-example">Example</span>{/if}
                {#if _sayPi}<span class="cue-type-tag cue-type-pivot">Pivot</span>{/if}
              {/if}
            </div>
            <div class="tp-tell">
              {#if tpStreaming && !parsed.body}
                {parsed.tell}<span class="cursor">|</span>
              {:else if parsed.strategies.length > 1 || parsed.strategies[0]?.keyword}
                {#each parsed.strategies as strategy, si}
                  {@const stratKey = `strat-${currentIndex}-${si}`}
                  {@const isStratOpen = !!openCues[stratKey]}
                  <div class="tp-strat-block" class:tp-strat-open={isStratOpen}>
                    <button class="tp-strat-toggle" onclick={() => toggleCueOpen(stratKey)}>
                      {#if strategy.keyword}<span class="tp-strat-kw">{strategy.keyword}</span>{/if}
                      <span class="tp-strat-preview">{strategy.text.split(/(?<=[.!?])\s+/)[0] ?? strategy.text}</span>
                      <span class="tp-strat-chevron">{isStratOpen ? '▾' : '▸'}</span>
                    </button>
                    {#if isStratOpen}
                      <div class="tp-strat-body">
                        {#each strategy.text.split(/(?<=[.!?])\s+/).filter(Boolean) as s}
                          <span class="tp-strat-sent">{s.trim()}</span>
                        {/each}
                      </div>
                    {/if}
                  </div>
                {/each}
              {:else}
                {#each parsed.tell.split(/(?<=[.!?])\s+/).filter(Boolean) as s}
                  <span class="tp-tell-sent" class:tp-strategy-gap={/^(I also|Beyond that|On top of that)/i.test(s.trim())}>{s.trim()}</span>
                {/each}
              {/if}
            </div>
            {#if bodyCues.length > 0}
                <div class="tp-cues">
                  {#each bodyCues as cue}
                    {@const isFlat = cue.label === 'Pivot' || cue.typeTag === 'Pivot'}
                    {@const isOpen = !isFlat && !!openCues[cue.text]}
                    {#if isFlat}
                      <div class="tp-cue-flat">
                        <span class="cue-label-sm" class:cue-label-transfer={cue.typeTag === 'Pivot' || cue.label === 'Pivot'}>{cue.typeTag || (cue.label === 'General' ? 'Point' : cue.label)}</span>
                        <span class="tp-cue-flat-text">{cue.text}</span>
                      </div>
                    {:else}
                      <div class="tp-cue-block" class:tp-cue-open={isOpen}>
                        <button class="tp-cue-toggle" onclick={() => { const opening = !isOpen; toggleCueOpen(cue.text); if (opening) expandCue(current.question, cue.text); }}>
                          <span class="cue-label-sm" class:cue-label-example={cue.label === 'Example' || cue.label === 'Story'}>{cue.label}</span>
                          {#if cue.typeTag}<span class="tp-cue-keyword">{cue.typeTag}</span>{/if}
                          <span class="tp-cue-preview">{cue.title || (cue.text.split(/(?<=[.!?])\s+/)[0] ?? cue.text)}</span>
                          <span class="tp-cue-chevron">{isOpen ? '▾' : '▸'}</span>
                        </button>
                        {#if isOpen}
                          <div class="tp-cue-body">
                            {#if expandedCues[cue.text]?.loading}
                              <div class="cue-sentence cue-loading">…</div>
                            {:else if expandedCues[cue.text]?.sentence}
                              <div class="cue-sentence">{#each expandedCues[cue.text].sentence.split(/(?<=[.!?])\s+/) as s}{s.trim()}<br/>{/each}</div>
                            {:else}
                              <div class="cue-sentence">{#each cue.text.split(/(?<=[.!?])\s+/).filter(Boolean) as s}{s.trim()}<br/>{/each}</div>
                            {/if}
                          </div>
                        {/if}
                      </div>
                    {/if}
                  {/each}
                </div>
            {:else if parsed.body}
              <div class="tp-expand-row">
                <button class="tp-expand-btn" onclick={() => toggleExpand(currentIndex)}>
                  {expanded[currentIndex] ? '▴ Less context' : '▾ More context'}
                </button>
              </div>
              {#if expanded[currentIndex]}
                <div class="tp-body">
                  {@html renderBold(parsed.body)}
                  {#if tpStreaming}<span class="cursor">|</span>{/if}
                </div>
              {/if}
            {/if}
          </div>

          <!-- CLOSE section (behavioral/competency) -->
          {#if parsed.close}
            <div class="tp-sec tp-sec-close">
              <span class="cue-badge cue-close">Close</span>
              <span class="tp-close-text">{parsed.close}</span>
            </div>
          {/if}

          <!-- ASK section (behavioral/competency) -->
          {#if parsed.asks.length > 0}
            <div class="tp-sec tp-sec-ask">
              <span class="cue-badge cue-ask">Ask</span>
              <div class="tp-ask-list">
                {#each parsed.asks as ask}
                  {@const askKey = '[Ask] ' + ask.topic}
                  <div class="tp-ask-item">
                    <div class="tp-ask-content">
                      <span class="tp-ask-topic">{ask.topic}</span>
                      <span class="tp-ask-question">{expandedCues[askKey]?.sentence || ask.question}</span>
                      {#if ask.followUp}<span class="tp-ask-followup">↳ {ask.followUp}</span>{/if}
                    </div>
                  </div>
                {/each}
              </div>
            </div>
          {/if}
          {/if}<!-- end behavioral/competency else -->

        {/if}<!-- end streaming check -->

        <!-- Peek sections: other modes available inline -->
        {#if current.secondaryTag}
          {@const activeMode2 = getActiveMode(currentIndex, current)}
          {#each (['compound', 'primary', 'secondary'] as const).filter(m => m !== activeMode2) as peekMode}
            {@const peekContent = getModeContent(peekMode, current)}
            {@const peekStreaming2 = getModeStreaming(peekMode, current)}
            {@const peekLabel = peekMode === 'compound' ? 'Compound' : peekMode === 'primary' ? (TAG_CONFIG[current.tag ?? 'general']?.label ?? 'Primary') : (TAG_CONFIG[current.secondaryTag]?.label ?? 'Secondary')}
            <details class="tp-peek">
              <summary class="tp-peek-summary">
                <span class="tp-peek-label">{peekLabel}</span>
                {#if peekStreaming2}<span class="tp-peek-status streaming">●</span>
                {:else if peekContent}<span class="tp-peek-status ready">▸ View</span>
                {:else}<span class="tp-peek-status queued">○ Queued</span>{/if}
              </summary>
              {#if peekContent && !peekStreaming2}
                {@const pp = parseSuggestion(peekContent)}
                <div class="tp-peek-body">
                  {#if pp.acknowledge}<p class="tp-peek-line"><span class="tp-peek-cue">Ack</span>{pp.acknowledge}</p>{/if}
                  {#if pp.tell}{@const sentences = pp.tell.split(/(?<=[.!?])\s+/).filter(Boolean)}<p class="tp-peek-line"><span class="tp-peek-cue">Say</span>{sentences.slice(0,2).join(' ')}{sentences.length > 2 ? '…' : ''}</p>{/if}
                  {#if pp.present}<p class="tp-peek-line"><span class="tp-peek-cue">Summary</span>{pp.present}</p>{/if}
                  {#if pp.company}<p class="tp-peek-line"><span class="tp-peek-cue">Company</span>{pp.company}</p>{/if}
                  {#if pp.direction}<p class="tp-peek-line"><span class="tp-peek-cue">Direction</span>{pp.direction}</p>{/if}
                  <button class="tp-peek-switch" onclick={() => setActiveMode(currentIndex, peekMode)}>Switch to {peekLabel} →</button>
                </div>
              {:else if peekStreaming2}
                <div class="tp-peek-loading">Generating<span class="dots">...</span></div>
              {/if}
            </details>
          {/each}
        {/if}
      </div>
      <span class="tp-hint">
        {#if parsed.present || parsed.thread || parsed.past || parsed.future}Summary → Thread → Story → Next → Close
        {:else if parsed.company || parsed.role || parsed.self}Company → Role → Self → Close
        {:else if parsed.direction || parsed.alignment || parsed.contribution}Direction → Alignment → Contribution → Close
        {:else if parsed.asks.length >= 3 && !parsed.acknowledge && !parsed.tell}Questions to ask the interviewer
        {:else}Acknowledge → Answer → Close · Ask = follow-up{/if}
      </span>
    {:else}
      <div class="tp-empty">Waiting for a question...</div>
    {/if}
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
          {@const eMode = getActiveMode(i, entry)}
          {@const eModeSuggestion = getModeContent(eMode, entry)}
          {@const eModeStreaming = getModeStreaming(eMode, entry)}
          {@const parsed = parseSuggestion(eModeSuggestion)}
          {@const bodyCues = parsed.body ? parseCues(parsed.body) : []}
          {@const isLatest = i === suggestions.length - 1}
          {@const eAnsType = getAnswerType(parsed, eMode === 'compound' ? undefined : entry.tag)}
          <div class="entry" class:latest={isLatest}>
            <div class="question-row">
              <span class="q-num-badge">Q{i + 1}</span>
              <p class="question-text">"{entry.question}"</p>
              {#if entry.tag}
                {@const tc = TAG_CONFIG[entry.tag]}
                <span class="entry-tag" style="color: {tc.color}; background: {tc.bg}">{tc.label}</span>
              {/if}
              {#if eAnsType.framework && !entry.streaming}
                <span class="entry-ans-type">{eAnsType.label}</span>
              {/if}
              <button class="entry-collapse-btn" onclick={() => { collapsed[i] = !collapsed[i]; collapsed = [...collapsed]; }} title={collapsed[i] ? 'Expand' : 'Collapse'}>{collapsed[i] ? '▸' : '▾'}</button>
            </div>
            {#if entry.secondaryTag}
              {@const etc1 = TAG_CONFIG[entry.tag ?? 'general']}
              {@const etc2 = TAG_CONFIG[entry.secondaryTag]}
              <div class="mode-tabs mode-tabs-entry">
                <button class="mode-tab" class:mode-tab-active={eMode === 'compound'}
                        onclick={() => setActiveMode(i, 'compound')}>
                  <span class="mode-tab-dot" class:mode-tab-streaming={entry.compoundStreaming} class:mode-tab-ready={!entry.compoundStreaming && !!entry.compoundSuggestion}></span>
                  Compound
                </button>
                <button class="mode-tab" class:mode-tab-active={eMode === 'primary'}
                        onclick={() => setActiveMode(i, 'primary')}>
                  <span class="mode-tab-dot" class:mode-tab-streaming={entry.streaming} class:mode-tab-ready={!entry.streaming && !!entry.suggestion}></span>
                  <span style="color:{etc1.color}">{etc1.label}</span>
                </button>
                <button class="mode-tab" class:mode-tab-active={eMode === 'secondary'}
                        onclick={() => setActiveMode(i, 'secondary')}>
                  <span class="mode-tab-dot" class:mode-tab-streaming={entry.secondaryStreaming} class:mode-tab-ready={!entry.secondaryStreaming && !!entry.secondarySuggestion}></span>
                  <span style="color:{etc2.color}">{etc2.label}</span>
                </button>
              </div>
            {/if}
            {#if !collapsed[i]}
            {#if entry.redFlag}
              <div class="entry-redflag">
                <span class="redflag-cat">{entry.redFlag.category}</span>
                <span class="redflag-note">{entry.redFlag.coachingNote}</span>
              </div>
            {/if}
            {#if eModeStreaming && !eModeSuggestion}
              <span class="loading">Generating<span class="dots">...</span></span>
            {:else}
              {@const eIsIntro    = !!(parsed.present || parsed.thread || parsed.past || parsed.future)}
              {@const eIsMotiv    = !!(parsed.company || parsed.role || parsed.self)}
              {@const eIsFutureTy = !!(parsed.direction || parsed.alignment || parsed.contribution)}
              {@const eIsClosing  = parsed.asks.length >= 3 && !parsed.acknowledge && !parsed.present && !parsed.company && !parsed.direction && !parsed.tell}

              {#if eIsIntro}
                {#if parsed.present}<div class="e-sec e-sec-present"><span class="cue-badge cue-present">Summary</span><span class="affirm-text">{parsed.present}</span></div>{/if}
                {#if parsed.thread}<div class="e-sec e-sec-thread"><span class="cue-badge cue-thread">Thread</span><span class="affirm-text">{parsed.thread}</span></div>{/if}
                {#if parsed.transition1}<div class="e-transition">{parsed.transition1}</div>{/if}
                {#if parsed.past}<div class="e-sec e-sec-past"><span class="cue-badge cue-past">Story</span><span class="affirm-text">{parsed.past}</span></div>{/if}
                {#if parsed.transition2}<div class="e-transition">{parsed.transition2}</div>{/if}
                {#if parsed.future}<div class="e-sec e-sec-future"><span class="cue-badge cue-future">Next</span><span class="affirm-text">{parsed.future}</span></div>{/if}
                {#if parsed.transition3}<div class="e-transition">{parsed.transition3}</div>{/if}
                {#if parsed.close}<div class="e-sec e-sec-close"><span class="cue-badge cue-close">Close</span><span class="affirm-text">{parsed.close}</span></div>{/if}
                {#if parsed.asks.length > 0}<div class="e-sec e-sec-ask"><span class="cue-badge cue-ask">Ask</span><div class="tp-ask-list">{#each parsed.asks as ask}<div class="tp-ask-item"><div class="tp-ask-content"><span class="tp-ask-topic">{ask.topic}</span><span class="tp-ask-question">{ask.question}</span>{#if ask.followUp}<span class="tp-ask-followup">↳ {ask.followUp}</span>{/if}</div></div>{/each}</div></div>{/if}

              {:else if eIsMotiv}
                {#if parsed.company}<div class="e-sec e-sec-company"><span class="cue-badge cue-company">Company</span><span class="affirm-text">{parsed.company}</span></div>{/if}
                {#if parsed.transition1}<div class="e-transition">{parsed.transition1}</div>{/if}
                {#if parsed.role}<div class="e-sec e-sec-role"><span class="cue-badge cue-role">Role</span><span class="affirm-text">{parsed.role}</span></div>{/if}
                {#if parsed.transition2}<div class="e-transition">{parsed.transition2}</div>{/if}
                {#if parsed.self}<div class="e-sec e-sec-self"><span class="cue-badge cue-self">Self</span><span class="affirm-text">{parsed.self}</span></div>{/if}
                {#if parsed.transition3}<div class="e-transition">{parsed.transition3}</div>{/if}
                {#if parsed.close}<div class="e-sec e-sec-close"><span class="cue-badge cue-close">Close</span><span class="affirm-text">{parsed.close}</span></div>{/if}
                {#if parsed.asks.length > 0}<div class="e-sec e-sec-ask"><span class="cue-badge cue-ask">Ask</span><div class="tp-ask-list">{#each parsed.asks as ask}<div class="tp-ask-item"><div class="tp-ask-content"><span class="tp-ask-topic">{ask.topic}</span><span class="tp-ask-question">{ask.question}</span>{#if ask.followUp}<span class="tp-ask-followup">↳ {ask.followUp}</span>{/if}</div></div>{/each}</div></div>{/if}

              {:else if eIsFutureTy}
                {#if parsed.direction}<div class="e-sec e-sec-direction"><span class="cue-badge cue-direction">Direction</span><span class="affirm-text">{parsed.direction}</span></div>{/if}
                {#if parsed.transition1}<div class="e-transition">{parsed.transition1}</div>{/if}
                {#if parsed.alignment}<div class="e-sec e-sec-alignment"><span class="cue-badge cue-alignment">Alignment</span><span class="affirm-text">{parsed.alignment}</span></div>{/if}
                {#if parsed.transition2}<div class="e-transition">{parsed.transition2}</div>{/if}
                {#if parsed.contribution}<div class="e-sec e-sec-contribution"><span class="cue-badge cue-contribution">Contribution</span><span class="affirm-text">{parsed.contribution}</span></div>{/if}
                {#if parsed.transition3}<div class="e-transition">{parsed.transition3}</div>{/if}
                {#if parsed.close}<div class="e-sec e-sec-close"><span class="cue-badge cue-close">Close</span><span class="affirm-text">{parsed.close}</span></div>{/if}
                {#if parsed.asks.length > 0}<div class="e-sec e-sec-ask"><span class="cue-badge cue-ask">Ask</span><div class="tp-ask-list">{#each parsed.asks as ask}<div class="tp-ask-item"><div class="tp-ask-content"><span class="tp-ask-topic">{ask.topic}</span><span class="tp-ask-question">{ask.question}</span>{#if ask.followUp}<span class="tp-ask-followup">↳ {ask.followUp}</span>{/if}</div></div>{/each}</div></div>{/if}

              {:else if eIsClosing}
                <div class="e-closing-wrap">
                  <div class="tp-closing-header">Questions to Ask</div>
                  {#each parsed.asks as ask, ai}
                    <div class="tp-closing-card">
                      <span class="tp-closing-num">{ai + 1}</span>
                      <div class="tp-closing-content">
                        <span class="tp-closing-topic">{ask.topic}</span>
                        <span class="tp-closing-question">{ask.question}</span>
                      </div>
                    </div>
                  {/each}
                </div>

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
              {@const _eSayPi = bodyCues.some(c => c.label === 'Pivot' || c.typeTag === 'Pivot')}
              {@const _eSayEx = bodyCues.some(c => c.label === 'Example' || c.typeTag === 'Example')}
              <div class="e-sec e-sec-say" class:e-sec-say-example={_eSayEx} class:e-sec-say-pivot={_eSayPi}>
                <div class="e-sec-header">
                  <span class="cue-badge" class:cue-ask={parsed.cue === 'Ask'}>{parsed.cue}</span>
                  {#if parsed.tell && !eModeStreaming}
                    {@const secs = estimateSecs(parsed)}
                    <span class="tp-time-est">~{secs < 60 ? secs + 's' : Math.floor(secs/60) + 'm ' + (secs%60) + 's'}</span>
                    {#if _eSayEx}<span class="cue-type-tag cue-type-example">Example</span>{/if}
                    {#if _eSayPi}<span class="cue-type-tag cue-type-pivot">Pivot</span>{/if}
                  {/if}
                </div>
                <div class="tell-text">
                  {#if eModeStreaming && !parsed.body}
                    {parsed.tell}<span class="cursor">|</span>
                  {:else if parsed.strategies.length > 1 || parsed.strategies[0]?.keyword}
                    {#each parsed.strategies as strategy, si}
                      {@const stratKey = `strat-entry-${i}-${si}`}
                      {@const isStratOpen = !!openCues[stratKey]}
                      <div class="tp-strat-block" class:tp-strat-open={isStratOpen}>
                        <button class="tp-strat-toggle" onclick={() => toggleCueOpen(stratKey)}>
                          {#if strategy.keyword}<span class="tp-strat-kw">{strategy.keyword}</span>{/if}
                          <span class="tp-strat-preview">{strategy.text.split(/(?<=[.!?])\s+/)[0] ?? strategy.text}</span>
                          <span class="tp-strat-chevron">{isStratOpen ? '▾' : '▸'}</span>
                        </button>
                        {#if isStratOpen}
                          <div class="tp-strat-body">
                            {#each strategy.text.split(/(?<=[.!?])\s+/).filter(Boolean) as s}
                              <span class="tp-strat-sent">{s.trim()}</span>
                            {/each}
                          </div>
                        {/if}
                      </div>
                    {/each}
                  {:else}
                    {#each parsed.tell.split(/(?<=[.!?])\s+/).filter(Boolean) as s}
                      <span class="tp-tell-sent" class:tp-strategy-gap={/^(I also|Beyond that|On top of that)/i.test(s.trim())}>{s.trim()}</span>
                    {/each}
                  {/if}
                </div>
                {#if parsed.body}
                  <button class="expand-btn" onclick={() => toggleExpand(i)}>
                    {expanded[i] ? '▴ Less' : '▾ More context'}
                  </button>
                  {#if expanded[i]}
                    <div class="body-text">
                      {@html renderBold(parsed.body)}
                      {#if eModeStreaming}<span class="cursor">|</span>{/if}
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
                    {#each parsed.asks as ask}
                      {@const askKey = '[Ask] ' + ask.topic}
                      <div class="tp-ask-item">
                        <div class="tp-ask-content">
                          <span class="tp-ask-topic">{ask.topic}</span>
                          {#if expandedCues[askKey]?.sentence}
                            <span class="tp-ask-question">{expandedCues[askKey].sentence}</span>
                          {:else if expandedCues[askKey]?.loading}
                            <span class="tp-ask-question cue-loading">…</span>
                          {:else}
                            <span class="tp-ask-question">{ask.question}</span>
                          {/if}
                        </div>
                      </div>
                    {/each}
                  </div>
                </div>
              {/if}
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
  .tp-sec-say.tp-sec-say-example { border-left-color: #0891b2; }
  .tp-sec-say.tp-sec-say-pivot   { border-left-color: #ea580c; }
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
  .tp-ask-topic { font-size: var(--fs-sm); color: #f59e0b; font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; }
  .tp-ask-question { font-size: var(--fs-lg); color: #fde68a; line-height: 1.4; overflow-wrap: break-word; }
  .tp-ask-followup { font-size: var(--fs-sm); color: #78716c; line-height: 1.4; overflow-wrap: break-word; font-style: italic; }

  /* Breadcrumb: Q type · A framework */
  .tp-breadcrumb {
    display: flex; align-items: center; gap: 0.35rem;
    padding: 0.2rem 0 0.35rem;
    font-size: var(--fs-xs); font-weight: 700;
    text-transform: uppercase; letter-spacing: 0.05em;
  }
  .tp-bc-q { /* colored per tag, set inline */ }
  .tp-bc-sep { color: #334155; }
  .tp-bc-a { color: #475569; }

  /* Transition connector lines */
  .tp-transition {
    font-size: var(--fs-sm); color: #475569; font-style: italic;
    padding: 0.05rem 0 0.05rem 1.1rem; line-height: 1.4;
    overflow-wrap: break-word;
  }
  .e-transition {
    font-size: var(--fs-sm); color: #475569; font-style: italic;
    padding: 0.1rem 0 0.1rem 0.75rem; line-height: 1.4;
    overflow-wrap: break-word;
  }

  /* Answer type label on standard entries */
  .entry-ans-type {
    font-size: var(--fs-xs); font-weight: 600; color: #475569;
    letter-spacing: 0.03em; white-space: nowrap;
    padding: 0.05em 0.3em; background: #0d1525;
    border: 1px solid #1e2d45; border-radius: 0.2em;
    flex-shrink: 0;
  }

  /* Say text */
  .tp-tell {
    color: #ffffff;
    font-size: var(--fs-lg);
    line-height: 1.5;
    flex: 1;
    overflow-wrap: break-word;
    word-break: break-word;
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
  }
  .tp-tell-sent { display: block; line-height: 1.5; }
  .tp-strategy-gap { margin-top: 0.55rem; }

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
  .tp-cue-keyword {
    font-size: var(--fs-xs); font-weight: 700; text-transform: uppercase;
    letter-spacing: 0.06em; color: #86efac; background: #14532d;
    padding: 0.05rem 0.35rem; border-radius: 0.2rem; flex-shrink: 0;
  }
  .tp-cue-preview {
    flex: 1; font-size: var(--fs-lg); color: #3d8c52;
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
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
  .tp-cue-flat { display: flex; align-items: center; gap: 0.4rem; padding: 0.15rem 0.4rem; }
  .tp-cue-flat-text { flex: 1; font-size: var(--fs-base); color: #3d8c52; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .tp-sec > .cue-badge, .e-sec > .cue-badge { align-self: flex-start; }
  .cue-type-tag { display: inline-block; font-size: var(--fs-xs); font-weight: 800; padding: 0.1rem 0.4rem; border-radius: 0.25rem; background: #14532d; color: #4ade80; text-transform: uppercase; letter-spacing: 0.06em; flex-shrink: 0; }
  .cue-type-example { background: #1a3a1a; color: #86efac; }
  .cue-type-pivot { background: #3b1506; color: #fb923c; }
  .cue-label-example { }
  .cue-label-transfer { color: #fb923c !important; }
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
  .tp-provider-badge {
    font-size: 0.6rem; font-weight: 600; letter-spacing: 0.03em;
    padding: 0.07rem 0.3rem; border-radius: 9999px;
    background: #1e3a5f; color: #7eb8f7; border: 1px solid #2a4a7f;
    white-space: nowrap; flex-shrink: 0;
  }
  .tp-provider-local { background: #14302a; color: #4ade80; border-color: #166534; }


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
  .e-sec-say.e-sec-say-example { border-left-color: #0891b2; }
  .e-sec-say.e-sec-say-pivot   { border-left-color: #ea580c; }
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

  /* === New question type sections === */

  /* Introduction */
  .tp-sec-present      { background: #071020; border-left-color: #2563eb; }
  .tp-sec-thread       { background: #0d0a20; border-left-color: #7c3aed; }
  .e-sec-thread        { background: #0d0a20; border-left-color: #7c3aed; }
  .tp-sec-past         { background: #120a00; border-left-color: #b45309; }
  .tp-sec-future       { background: #071a0d; border-left-color: #059669; }

  /* Motivation */
  .tp-sec-company      { background: #0d0720; border-left-color: #7c3aed; }
  .tp-sec-role         { background: #071a1a; border-left-color: #0e7490; }
  .tp-sec-self         { background: #071622; border-left-color: #0ea5e9; }

  /* Future growth */
  .tp-sec-direction    { background: #071a0d; border-left-color: #059669; }
  .tp-sec-alignment    { background: #071a1a; border-left-color: #0e7490; }
  .tp-sec-contribution { background: #120a00; border-left-color: #b45309; }

  /* Narrative text (used in all new type sections) */
  .tp-narrative-text {
    color: #f1f5f9;
    font-size: var(--fs-lg);
    line-height: 1.45;
    overflow-wrap: break-word;
    flex: 1;
  }

  /* CUE badge colors for new types */
  .cue-badge.cue-present      { background: #1e3a8a; color: #93c5fd; }
  .cue-badge.cue-thread       { background: #3b0764; color: #d8b4fe; }
  .cue-badge.cue-past         { background: #451a03; color: #fcd34d; }
  .cue-badge.cue-future       { background: #14532d; color: #4ade80; }
  .cue-badge.cue-company      { background: #3b0764; color: #d8b4fe; }
  .cue-badge.cue-role         { background: #164e63; color: #67e8f9; }
  .cue-badge.cue-self         { background: #0c4a6e; color: #7dd3fc; }
  .cue-badge.cue-direction    { background: #14532d; color: #86efac; }
  .cue-badge.cue-alignment    { background: #134e4a; color: #5eead4; }
  .cue-badge.cue-contribution { background: #451a03; color: #fca5a5; }

  /* Closing question cards (teleprompter) */
  .tp-closing-wrap {
    display: flex; flex-direction: column; gap: 0.5rem; flex: 1;
  }
  .tp-closing-header {
    font-size: var(--fs-sm); font-weight: 800; text-transform: uppercase;
    letter-spacing: 0.08em; color: #475569; padding: 0 0.25rem;
  }
  .tp-closing-card {
    display: flex; align-items: flex-start; gap: 0.6rem;
    padding: 0.55rem 0.75rem; background: #080d1a;
    border-radius: 0.45rem; border-left: 3px solid #3b82f6; flex-shrink: 0;
  }
  .tp-closing-num {
    font-size: var(--fs-lg); font-weight: 800; color: #3b82f6;
    flex-shrink: 0; min-width: 1.2rem; text-align: right;
  }
  .tp-closing-content { display: flex; flex-direction: column; gap: 0.2rem; flex: 1; }
  .tp-closing-topic {
    font-size: var(--fs-sm); font-weight: 700; text-transform: uppercase;
    letter-spacing: 0.06em; color: #6495e4;
  }
  .tp-closing-question {
    font-size: var(--fs-lg); color: #bfdbfe; line-height: 1.4; overflow-wrap: break-word;
  }

  /* Standard panel equivalents for new types */
  .e-sec-present      { background: #071020; border-left-color: #2563eb; }
  .e-sec-past         { background: #120a00; border-left-color: #b45309; }
  .e-sec-future       { background: #071a0d; border-left-color: #059669; }
  .e-sec-company      { background: #0d0720; border-left-color: #7c3aed; }
  .e-sec-role         { background: #071a1a; border-left-color: #0e7490; }
  .e-sec-self         { background: #071622; border-left-color: #0ea5e9; }
  .e-sec-direction    { background: #071a0d; border-left-color: #059669; }
  .e-sec-alignment    { background: #071a1a; border-left-color: #0e7490; }
  .e-sec-contribution { background: #120a00; border-left-color: #b45309; }
  .e-closing-wrap     { display: flex; flex-direction: column; gap: 0.4rem; }

  /* Strategy collapsible rows (inside tp-tell and tell-text) */
  .tp-strat-block {
    border-radius: 0.3rem; border: 1px solid #0d2010;
    overflow: hidden; background: #040b06; flex-shrink: 0;
  }
  .tp-strat-block.tp-strat-open { border-color: #1e4a2a; }
  .tp-strat-toggle {
    display: flex; align-items: center; gap: 0.4rem;
    width: 100%; padding: 0.3rem 0.5rem;
    background: none; border: none; cursor: pointer; text-align: left;
  }
  .tp-strat-toggle:hover { background: #071a0f; }
  .tp-strat-kw {
    font-size: var(--fs-xs); font-weight: 800; text-transform: uppercase;
    letter-spacing: 0.06em; color: #4ade80; background: #14532d;
    padding: 0.05rem 0.35rem; border-radius: 0.2rem; flex-shrink: 0;
  }
  .tp-strat-preview {
    flex: 1; font-size: var(--fs-lg); color: #94a3b8;
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
  }
  .tp-strat-chevron { font-size: var(--fs-sm); color: #2d6e40; flex-shrink: 0; }
  .tp-strat-body {
    display: flex; flex-direction: column; gap: 0.35rem;
    padding: 0.45rem 0.6rem; border-top: 1px solid #0d2010;
  }
  .tp-strat-sent {
    color: #f1f5f9; font-size: var(--fs-lg); line-height: 1.5; display: block;
  }

  /* === Mode tabs (compound / primary / secondary) === */
  .mode-tabs {
    display: flex;
    gap: 0.25rem;
    flex-shrink: 0;
    padding: 0.2rem 0;
  }
  .mode-tabs-entry {
    padding: 0.3rem 0.6rem;
    border-bottom: 1px solid #0f172a;
  }
  .mode-tab {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    padding: 0.25rem 0.6rem;
    background: #050d1a;
    border: 1px solid #1a2540;
    border-radius: 0.3rem;
    font-size: var(--fs-xs);
    font-weight: 700;
    color: #475569;
    cursor: pointer;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    transition: all 0.1s;
  }
  .mode-tab:hover { background: #0d1a2e; border-color: #2d4060; color: #94a3b8; }
  .mode-tab.mode-tab-active {
    background: #071a0d;
    border-color: #166534;
    color: #4ade80;
  }
  .mode-tab-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #1e293b;
    flex-shrink: 0;
  }
  .mode-tab-dot.mode-tab-streaming {
    background: #60a5fa;
    animation: pulse 1.5s ease-in-out infinite;
  }
  .mode-tab-dot.mode-tab-ready { background: #22c55e; }

  /* === Peek sections === */
  .tp-peek {
    background: #040c18;
    border: 1px solid #0f1e30;
    border-radius: 0.4rem;
    overflow: hidden;
    flex-shrink: 0;
  }
  .tp-peek[open] { border-color: #1a2d45; }
  .tp-peek-summary {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.35rem 0.6rem;
    cursor: pointer;
    list-style: none;
    user-select: none;
  }
  .tp-peek-summary::-webkit-details-marker { display: none; }
  .tp-peek-label {
    font-size: var(--fs-xs);
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: #334155;
  }
  .tp-peek-status {
    font-size: var(--fs-xs);
    font-weight: 600;
    margin-left: auto;
  }
  .tp-peek-status.streaming { color: #60a5fa; animation: pulse 1.5s ease-in-out infinite; }
  .tp-peek-status.ready { color: #4ade80; }
  .tp-peek-status.queued { color: #334155; }
  .tp-peek-body {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    padding: 0.4rem 0.6rem 0.5rem;
    border-top: 1px solid #0f1e30;
  }
  .tp-peek-line {
    display: flex;
    gap: 0.4rem;
    align-items: baseline;
    margin: 0;
  }
  .tp-peek-cue {
    font-size: var(--fs-xs);
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #334155;
    flex-shrink: 0;
    min-width: 4rem;
  }
  .tp-peek-line > :last-child {
    color: #64748b;
    font-size: var(--fs-sm);
    line-height: 1.4;
  }
  .tp-peek-switch {
    align-self: flex-start;
    margin-top: 0.2rem;
    padding: 0.15rem 0.5rem;
    background: none;
    border: 1px solid #1a2540;
    border-radius: 0.25rem;
    font-size: var(--fs-xs);
    color: #4ade80;
    cursor: pointer;
    font-weight: 700;
  }
  .tp-peek-switch:hover { background: #071a0d; }
  .tp-peek-loading {
    padding: 0.3rem 0.6rem;
    color: #334155;
    font-style: italic;
    font-size: var(--fs-xs);
    border-top: 1px solid #0f1e30;
  }
</style>
