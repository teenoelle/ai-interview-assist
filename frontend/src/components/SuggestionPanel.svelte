<script lang="ts">
  import type { SuggestionEntry, VocalSentiment } from '../lib/types';
  import { untrack } from 'svelte';
  import { TAG_CONFIG } from '../lib/questionTagger';
  import { parseSuggestion, parseCues, getAnswerType, getSectionLabels } from '../lib/parseSuggestion';
  import PanelHeader from './PanelHeader.svelte';
  import { PracticeRecorder } from '../lib/practiceRecorder';
  import { authFetch } from '../lib/api';
  import SalaryCoachPanel from './SalaryCoachPanel.svelte';

  // Expand-cue state: cue text → { sentence, loading }
  let expandedCues = $state<Record<string, { sentence: string; loading: boolean }>>({});
  // Open/collapsed state for cue bullet blocks
  let openCues = $state<Record<string, boolean>>({});

  function toggleCueOpen(key: string) {
    openCues = { ...openCues, [key]: !openCues[key] };
  }

  // For strategy blocks: undefined/true = open, false = closed
  function toggleStratOpen(key: string) {
    openCues = { ...openCues, [key]: openCues[key] !== false ? false : true };
  }

  async function expandCue(question: string, cue: string) {
    if (expandedCues[cue]?.sentence) return; // already fetched — keep cached result
    if (expandedCues[cue]?.loading) return;  // fetch already in flight
    expandedCues = { ...expandedCues, [cue]: { sentence: '', loading: true } };
    try {
      const r = await authFetch('/api/expand-cue', {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ question, cue }),
      });
      const d = r.ok ? await r.json() : null;
      const sentence = d?.sentence?.trim() ?? '';
      expandedCues = { ...expandedCues, [cue]: { sentence: sentence || cue, loading: false } };
    } catch { expandedCues = { ...expandedCues, [cue]: { sentence: cue, loading: false } }; }
  }

  const { suggestions, onClear, teleprompter = false, lockOnNew = false, jumpSignal = null, navSignal = null, cueExpandSignal = null, onPinnedChange, onClosingSectionOpen, salaryTactics = null } = $props<{
    suggestions: SuggestionEntry[];
    onClear: () => void;
    teleprompter?: boolean;
    lockOnNew?: boolean;
    jumpSignal?: { idx: number; key: number } | null;
    navSignal?: { dir: 'prev' | 'next' | 'latest'; key: number } | null;
    cueExpandSignal?: { cueIdx: number; key: number } | null;
    onPinnedChange?: (pinned: boolean) => void;
    onClosingSectionOpen?: (entryIdx: number, key: string) => void;
    salaryTactics?: { early_round: string; reveal: string; direct_ask: string; total_package: string; counter: string } | null;
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
    // Strategies are open by default (no write needed — see isStratOpen below)
    const cues = parseCues(parsed.body);
    for (const cue of cues) {
      expandCue(current.question, cue.text);
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

  function navPrev() {
    const idx = currentIndex;
    if (idx > 0) jumpTo(idx - 1);
  }

  function navNext() {
    const idx = currentIndex;
    if (idx < totalCount - 1) jumpTo(idx + 1);
  }

  // When lockOnNew=true, freeze on the current question when a new one arrives.
  // The user must press Down (navLatest) to advance to the new question.
  let lockCount = $state(0);
  $effect(() => {
    const count = suggestions.length;
    if (lockOnNew) {
      untrack(() => {
        if (historyIndex === -1 && count > lockCount && lockCount > 0) {
          historyIndex = lockCount - 1;
        }
        lockCount = count;
      });
    }
  });

  // React to parent-driven navigation signals (arrow keys from app.svelte)
  $effect(() => {
    if (!navSignal) return;
    untrack(() => {
      if (navSignal.dir === 'prev') navPrev();
      else if (navSignal.dir === 'next') navNext();
      else jumpToLatest();
    });
  });

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

  let collapsedSecs = $state<Record<string, boolean>>({});
  function toggleSec(key: string) { collapsedSecs = { ...collapsedSecs, [key]: !collapsedSecs[key] }; }

  function shortAsk(text: string, words = 6): string {
    const w = text.split(' ');
    return w.length <= words ? text : w.slice(0, words).join(' ') + '…';
  }

  // Active mode per entry index (compound | primary | secondary)
  // Defaults to 'compound' for compound questions, 'primary' otherwise
  let activeModes = $state<Record<number, 'compound' | 'primary' | 'secondary'>>({});

  // Closing section open state: entryIndex → Set of open section keys ('hr'|'hm'|'ceo')
  let openClosingSections = $state<Record<number, Set<string>>>({});

  const CLOSING_SECTIONS = [
    { key: 'hr',  label: 'HR' },
    { key: 'hm',  label: 'Hiring Manager' },
    { key: 'ceo', label: 'CEO' },
  ] as const;

  function toggleClosingSection(entryIdx: number, key: string) {
    const prev = openClosingSections[entryIdx] ?? new Set<string>();
    const next = new Set(prev);
    if (next.has(key)) {
      next.delete(key);
    } else {
      next.add(key);
      onClosingSectionOpen?.(entryIdx, key);
    }
    openClosingSections = { ...openClosingSections, [entryIdx]: next };
  }

  function closingSectionContent(entry: import('../lib/types').SuggestionEntry, key: string): string {
    return (key === 'hr' ? entry.closingHR : key === 'hm' ? entry.closingHM : entry.closingCEO) ?? '';
  }

  function getActiveMode(i: number, entry: import('../lib/types').SuggestionEntry): 'compound' | 'primary' | 'secondary' {
    if (!entry.secondaryTag) return 'primary';
    return activeModes[i] ?? 'compound';
  }

  function setActiveMode(i: number, mode: 'compound' | 'primary' | 'secondary') {
    activeModes = { ...activeModes, [i]: mode };
    // Fetch on demand if not yet generated
    const entry = suggestions[i];
    if (!entry) return;
    const isEmpty = mode === 'primary' ? !entry.suggestion && !entry.streaming
                  : mode === 'secondary' ? !entry.secondarySuggestion && !entry.secondaryStreaming
                  : false;
    if (isEmpty && (mode === 'primary' || mode === 'secondary')) {
      authFetch('/api/suggest-mode', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ question: entry.question, mode }),
      }).catch(() => {});
    }
  }

  function groupAsksBySection(asks: { topic: string; question: string; followUp?: string; section?: string }[]) {
    const groups: { section: string; asks: typeof asks }[] = [];
    let current: (typeof groups)[0] | null = null;
    for (const ask of asks) {
      const s = ask.section ?? '';
      if (!current || current.section !== s) {
        current = { section: s, asks: [] };
        groups.push(current);
      }
      current.asks.push(ask);
    }
    return groups;
  }

  function getModeContent(mode: 'compound' | 'primary' | 'secondary', entry: import('../lib/types').SuggestionEntry): string {
    if (mode === 'compound') return entry.compoundSuggestion ?? '';
    if (mode === 'secondary') return entry.secondarySuggestion ?? '';
    return entry.suggestion ?? '';
  }

  function getModeStreaming(mode: 'compound' | 'primary' | 'secondary', entry: import('../lib/types').SuggestionEntry): boolean {
    if (mode === 'compound') return entry.compoundStreaming ?? false;
    if (mode === 'secondary') return entry.secondaryStreaming ?? false;
    return entry.streaming;
  }

  // ── Practice recording ─────────────────────────────────────────────────────
  const practiceRecorder = new PracticeRecorder();
  let practiceState = $state<Record<number, 'recording' | 'loading'>>({});
  let practiceResults = $state<Record<number, VocalSentiment>>({});
  let practiceLiveText = $state<Record<number, string>>({});
  let practiceRecordingIdx = $state<number | null>(null);

  function startPractice(i: number) {
    if (practiceRecordingIdx !== null) {
      practiceRecorder.abort();
      const prev = practiceRecordingIdx;
      const { [prev]: _, ...rest } = practiceState;
      practiceState = rest;
    }
    practiceRecordingIdx = i;
    practiceState = { ...practiceState, [i]: 'recording' };
    practiceLiveText = { ...practiceLiveText, [i]: '' };
    practiceRecorder.start((text) => {
      practiceLiveText = { ...practiceLiveText, [i]: text };
    });
  }

  async function stopPractice(i: number) {
    practiceRecordingIdx = null;
    const result = practiceRecorder.stop();
    practiceState = { ...practiceState, [i]: 'loading' };
    try {
      const resp = await fetch('/api/vocal-sentiment', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          question: suggestions[i].question,
          transcript: result.transcript,
          duration_seconds: result.duration,
          word_count: result.wordCount,
          filler_count: result.fillerCount,
          filler_detail: result.fillerDetail,
        }),
      });
      if (resp.ok) {
        const data: VocalSentiment = await resp.json();
        practiceResults = { ...practiceResults, [i]: data };
      }
    } catch { /* ignore */ }
    const { [i]: _, ...rest } = practiceState;
    practiceState = rest;
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
            <span class="tp-tag">{tc.label}</span>
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

      {@const tpMode = current.secondaryTag ? getActiveMode(currentIndex, current) : 'primary'}
      {@const tpSuggestion = getModeContent(tpMode, current)}
      {@const tpStreaming = getModeStreaming(tpMode, current)}
      {@const parsedText = typeof tpSuggestion === 'string' ? tpSuggestion : ''}
      {@const parsed = parseSuggestion(parsedText, tpStreaming)}
      {@const bodyCues = parsed.body ? parseCues(parsed.body) : []}
      {@const ansType = getAnswerType(parsed, tpMode === 'compound' ? undefined : current.tag)}
      {#if ansType.framework && !tpStreaming}
        <div class="tp-breadcrumb">
          <span class="tp-bc-a">{ansType.framework}</span>
        </div>
      {/if}
      {#if current.secondaryTag}
        {@const tc1 = TAG_CONFIG[current.tag ?? 'general']}
        {@const tc2 = TAG_CONFIG[current.secondaryTag]}
        {@const activeMode = getActiveMode(currentIndex, current)}
        <div class="mode-tabs">
          <button class="mode-tab" class:mode-tab-active={activeMode === 'compound'}
                  onclick={() => setActiveMode(currentIndex, 'compound')}>
            Compound
          </button>
          <button class="mode-tab" class:mode-tab-active={activeMode === 'primary'}
                  onclick={() => setActiveMode(currentIndex, 'primary')}>
            <span style="color:{tc1.color}">{tc1.label}</span>
          </button>
          <button class="mode-tab" class:mode-tab-active={activeMode === 'secondary'}
                  onclick={() => setActiveMode(currentIndex, 'secondary')}>
            <span style="color:{tc2.color}">{tc2.label}</span>
          </button>
        </div>
      {/if}

      {#if current.tag === 'salary' && salaryTactics}
        <SalaryCoachPanel tactics={salaryTactics} onClose={() => {}} />
      {:else}
      <div class="tp-card">
        {#if current.redFlag}
          <div class="tp-redflag">
            <span class="redflag-cat">{current.redFlag.category}</span>
            <span class="redflag-note">{current.redFlag.coachingNote}</span>
          </div>
        {/if}

        {#if !tpSuggestion && !tpStreaming && (tpMode === 'primary' || tpMode === 'secondary') && current.secondaryTag}
          <span class="tp-loading tp-loading-pending">Click to generate {tpMode} coaching…</span>
        {:else if tpStreaming && !tpSuggestion}
          <span class="tp-loading">Generating<span class="dots">...</span></span>
        {:else}
          {@const isIntro    = !!(parsed.present || parsed.thread || parsed.past || parsed.future)}
          {@const isMotiv    = !!(parsed.company || parsed.role || parsed.self)}
          {@const isFutureTy = !!(parsed.direction || parsed.alignment || parsed.contribution)}
          {@const isClosing  = current?.tag === 'closing'}

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
            {#if parsed.asks.length > 0}<div class="tp-sec tp-sec-ask"><span class="cue-badge cue-ask">Ask</span><div class="tp-ask-list">{#each parsed.asks as ask}<div class="tp-ask-item"><div class="tp-ask-content">{#if ask.topic}<span class="tp-ask-topic">{ask.topic}</span>{/if}<span class="tp-ask-question">{ask.question}</span>{#if ask.followUp}<span class="tp-ask-followup">↳ {ask.followUp}</span>{/if}</div></div>{/each}</div></div>{/if}

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
            {#if parsed.asks.length > 0}<div class="tp-sec tp-sec-ask"><span class="cue-badge cue-ask">Ask</span><div class="tp-ask-list">{#each parsed.asks as ask}<div class="tp-ask-item"><div class="tp-ask-content">{#if ask.topic}<span class="tp-ask-topic">{ask.topic}</span>{/if}<span class="tp-ask-question">{ask.question}</span>{#if ask.followUp}<span class="tp-ask-followup">↳ {ask.followUp}</span>{/if}</div></div>{/each}</div></div>{/if}

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
            {#if parsed.asks.length > 0}<div class="tp-sec tp-sec-ask"><span class="cue-badge cue-ask">Ask</span><div class="tp-ask-list">{#each parsed.asks as ask}<div class="tp-ask-item"><div class="tp-ask-content">{#if ask.topic}<span class="tp-ask-topic">{ask.topic}</span>{/if}<span class="tp-ask-question">{ask.question}</span>{#if ask.followUp}<span class="tp-ask-followup">↳ {ask.followUp}</span>{/if}</div></div>{/each}</div></div>{/if}

          {:else if isClosing}
            <!-- CLOSING: Three collapsible sections, fetched on demand -->
            <div class="tp-closing-wrap">
              {#each CLOSING_SECTIONS as sec}
                {@const isOpen = openClosingSections[currentIndex]?.has(sec.key) ?? false}
                {@const content = closingSectionContent(current, sec.key)}
                {@const fetched = sec.key === 'hr' ? current.closingHRFetched : sec.key === 'hm' ? current.closingHMFetched : current.closingCEOFetched}
                <div class="closing-sec">
                  <!-- svelte-ignore a11y_no_static_element_interactions -->
                  <div class="closing-sec-toggle" onclick={() => toggleClosingSection(currentIndex, sec.key)}>
                    <span class="closing-sec-arrow">{isOpen ? '▾' : '▸'}</span>
                    <span class="closing-sec-label">{sec.label}</span>
                  </div>
                  {#if isOpen}
                    {#if fetched && !content}
                      <div class="closing-loading">···</div>
                    {:else if content}
                      {@const secParsed = parseSuggestion(content)}
                      {#each secParsed.asks as ask}
                        <div class="tp-closing-card">
                          <div class="tp-closing-content">
                            <span class="tp-closing-topic">{ask.topic}</span>
                            <span class="tp-closing-question">{ask.question}</span>
                            {#if ask.followUp}<span class="tp-ask-followup">↳ {ask.followUp}</span>{/if}
                          </div>
                        </div>
                      {/each}
                    {/if}
                  {/if}
                </div>
              {/each}
            </div>

          {:else}
            <!-- BEHAVIORAL / COMPETENCY / STRENGTHS: existing layout -->
            {@const sl = getSectionLabels(tpMode === 'compound' ? undefined : current.tag)}
            {#if parsed.acknowledge}
              <div class="tp-sec tp-sec-ack">
                <button class="e-sec-header e-sec-header-toggle" onclick={() => toggleSec('tp-ack')}>
                  <span class="cue-badge cue-ack">{sl.ack}</span>
                  <span class="e-sec-chevron">{collapsedSecs['tp-ack'] ? '▸' : '▾'}</span>
                </button>
                {#if !collapsedSecs['tp-ack']}<span class="tp-ack-text">{parsed.acknowledge}</span>{/if}
              </div>
            {/if}
            {#if parsed.solve}
              <div class="tp-sec tp-sec-solve">
                <button class="e-sec-header e-sec-header-toggle" onclick={() => toggleSec('tp-solve')}>
                  <span class="cue-badge cue-solve">{sl.solve}</span>
                  <span class="e-sec-chevron">{collapsedSecs['tp-solve'] ? '▸' : '▾'}</span>
                </button>
                {#if !collapsedSecs['tp-solve']}
                  {#if parsed.solveStrategies.length > 1 || parsed.solveStrategies[0]?.keyword}
                    <div class="tp-strats-row">
                      {#each parsed.solveStrategies as strategy, si}
                        {@const stratKey = `solve-strat-${currentIndex}-${si}`}
                        {@const isStratOpen = openCues[stratKey] !== false}
                        <div class="tp-strat-block" class:tp-strat-open={isStratOpen} style={isStratOpen ? 'max-width:100%' : ''}>
                          <button class="tp-strat-toggle" onclick={() => toggleStratOpen(stratKey)}>
                            <span class="tp-strat-kw">{strategy.keyword || '·'}</span>
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
                    </div>
                  {:else}
                    <span class="tp-solve-text">{parsed.solve}</span>
                  {/if}
                {/if}
              </div>
            {/if}
            {#if parsed.bridge}
              <div class="tp-sec tp-sec-bridge">
                <button class="e-sec-header e-sec-header-toggle" onclick={() => toggleSec('tp-bridge')}>
                  <span class="cue-badge cue-bridge">{sl.bridge}</span>
                  <span class="e-sec-chevron">{collapsedSecs['tp-bridge'] ? '▸' : '▾'}</span>
                </button>
                {#if !collapsedSecs['tp-bridge']}<span class="tp-bridge-text">{parsed.bridge}</span>{/if}
              </div>
            {/if}

          <!-- ANSWER section (includes cue points) -->
          {@const _sayPi = bodyCues.some(c => c.label === 'Pivot' || c.typeTag === 'Pivot')}
          {@const _sayEx = bodyCues.some(c => c.label === 'Example' || c.typeTag === 'Example')}
          {#if parsed.tell || parsed.strategies.length > 1 || parsed.strategies[0]?.keyword || bodyCues.length > 0 || tpStreaming}
          <div class="tp-sec tp-sec-say" class:tp-sec-say-example={_sayEx} class:tp-sec-say-pivot={_sayPi}>
            <button class="e-sec-header e-sec-header-toggle" onclick={() => toggleSec('tp-say')}>
              <span class="cue-badge">{sl.answer}</span>
              {#if parsed.tell && !tpStreaming}
                {@const secs = estimateSecs(parsed)}
                <span class="tp-time-est">~{secs < 60 ? secs + 's' : Math.floor(secs/60) + 'm ' + (secs%60) + 's'}</span>
              {/if}
              <span class="e-sec-chevron">{collapsedSecs['tp-say'] ? '▸' : '▾'}</span>
            </button>
            {#if !collapsedSecs['tp-say']}
            <div class="tp-tell">
              {#if tpStreaming && !parsed.body}
                {parsed.tell}<span class="cursor">|</span>
              {:else if parsed.strategies.length > 1 || parsed.strategies[0]?.keyword}
                <div class="tp-strats-row">
                  {#each parsed.strategies as strategy, si}
                    {@const stratKey = `strat-${currentIndex}-${si}`}
                    {@const isStratOpen = openCues[stratKey] !== false}
                    <div class="tp-strat-block" class:tp-strat-open={isStratOpen} style={isStratOpen ? 'max-width:100%' : ''}>
                      <button class="tp-strat-toggle" onclick={() => toggleStratOpen(stratKey)}>
                        <span class="tp-strat-kw">{strategy.keyword || '·'}</span>
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
                </div>
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
                    {#if isFlat}
                      <div class="tp-cue-flat">
                        <span class="cue-label-sm" class:cue-label-transfer={cue.typeTag === 'Pivot' || cue.label === 'Pivot'}>{cue.typeTag || (cue.label === 'General' ? 'Point' : cue.label)}</span>
                        <span class="tp-cue-flat-text">{cue.text}</span>
                      </div>
                    {:else}
                      <div class="tp-cue-block" class:tp-cue-open={!!openCues[cue.text]}>
                        <button class="tp-cue-toggle" onclick={() => { const opening = !openCues[cue.text]; toggleCueOpen(cue.text); if (opening) expandCue(current.question, cue.text); }}>
                          <span class="tp-cue-preview">{cue.title || (cue.text.split(/(?<=[.!?])\s+/)[0] ?? cue.text)}</span>
                          <span class="tp-cue-chevron">{openCues[cue.text] ? '▾' : '▸'}</span>
                        </button>
                        {#if openCues[cue.text]}
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
            {/if}<!-- end tp-say collapsed -->
          </div>
          {/if}<!-- end Answer section guard -->

          <!-- CLOSE section (behavioral/competency) -->
          {#if parsed.close}
            <div class="tp-sec tp-sec-close">
              <span class="cue-badge cue-close">{sl.close}</span>
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
                      {#if ask.topic}<span class="tp-ask-topic">{ask.topic}</span>{/if}
                      <span class="tp-ask-question">{ask.question}</span>
                      {#if ask.followUp}<span class="tp-ask-followup">↳ {ask.followUp}</span>{/if}
                    </div>
                  </div>
                {/each}
              </div>
            </div>
          {/if}
          {/if}<!-- end behavioral/competency else -->

        {/if}<!-- end streaming check -->

        <!-- Raw-text fallback: if no parsed sections rendered but text exists -->
        {#if tpSuggestion && !tpStreaming && !parsed.acknowledge && !parsed.tell && !parsed.present && !parsed.company && !parsed.direction && parsed.asks.length === 0}
          <div class="tp-raw-fallback">
            {#each tpSuggestion.split('\n').filter(l => l.trim()) as line}
              <p class="tp-raw-line">{line}</p>
            {/each}
          </div>
        {/if}

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
      {#if ansType.label}<span class="tp-hint">{ansType.label}</span>{/if}
      {#if !tpStreaming && tpSuggestion && practiceRecorder.supported}
        {@const pState = practiceState[currentIndex]}
        {@const pResult = practiceResults[currentIndex]}
        <div class="practice-bar">
          {#if !pState}
            <button class="practice-btn" onclick={() => startPractice(currentIndex)}>● Practice answer</button>
          {:else if pState === 'recording'}
            <div class="practice-recording-row">
              <span class="practice-rec-dot">●</span>
              <span class="practice-live-text">{practiceLiveText[currentIndex] || 'Listening…'}</span>
              <button class="practice-stop-btn" onclick={() => stopPractice(currentIndex)}>◼ Stop</button>
            </div>
          {:else if pState === 'loading'}
            <span class="practice-scoring">Scoring<span class="dots">...</span></span>
          {/if}
          {#if pResult}
            <div class="practice-result">
              <div class="pr-header">
                <span class="pr-score"
                  class:pr-score-good={pResult.confidence_score >= 70}
                  class:pr-score-mid={pResult.confidence_score >= 50 && pResult.confidence_score < 70}
                >{pResult.confidence_score}</span>
                <span class="pr-tone">{pResult.tone}</span>
                <span class="pr-pace">{pResult.pace}</span>
                <button class="practice-btn practice-retry-btn" onclick={() => startPractice(currentIndex)}>↺ Retry</button>
              </div>
              {#if pResult.fillers_noted}<div class="pr-fillers">{pResult.fillers_noted}</div>{/if}
              <div class="pr-coaching">{pResult.coaching}</div>
            </div>
          {/if}
        </div>
      {/if}
      {/if}
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
          {@const parsed = parseSuggestion(eModeSuggestion, eModeStreaming)}
          {@const bodyCues = parsed.body ? parseCues(parsed.body) : []}
          {@const isLatest = i === suggestions.length - 1}
          {@const eAnsType = getAnswerType(parsed, eMode === 'compound' ? undefined : entry.tag)}
          <div class="entry" class:latest={isLatest}>
            <div class="question-row">
              <span class="q-num-badge">Q{i + 1}</span>
              <p class="question-text">"{entry.question}"</p>
              {#if entry.tag}
                {@const tc = TAG_CONFIG[entry.tag]}
                <span class="entry-tag">{tc.label}</span>
              {/if}
              {#if eAnsType.framework && !entry.streaming}
                <span class="entry-ans-type" title={eAnsType.label}>{eAnsType.framework}</span>
              {/if}
              <button class="entry-collapse-btn" onclick={() => { collapsed[i] = !collapsed[i]; collapsed = [...collapsed]; }} title={collapsed[i] ? 'Expand' : 'Collapse'}>{collapsed[i] ? '▸' : '▾'}</button>
            </div>
            {#if entry.secondaryTag}
              {@const etc1 = TAG_CONFIG[entry.tag ?? 'general']}
              {@const etc2 = TAG_CONFIG[entry.secondaryTag]}
              <div class="mode-tabs mode-tabs-entry">
                <button class="mode-tab" class:mode-tab-active={eMode === 'compound'}
                        onclick={() => setActiveMode(i, 'compound')}>
                  Compound
                </button>
                <button class="mode-tab" class:mode-tab-active={eMode === 'primary'}
                        onclick={() => setActiveMode(i, 'primary')}>
                  <span style="color:{etc1.color}">{etc1.label}</span>
                </button>
                <button class="mode-tab" class:mode-tab-active={eMode === 'secondary'}
                        onclick={() => setActiveMode(i, 'secondary')}>
                  <span style="color:{etc2.color}">{etc2.label}</span>
                </button>
              </div>
            {/if}
            {#if !collapsed[i]}
            <!-- Practice bar — shown above cues so it's always visible without scrolling -->
            {#if !eModeStreaming && eModeSuggestion}
              {@const pState = practiceState[i]}
              {@const pResult = practiceResults[i]}
              <div class="practice-bar">
                {#if practiceRecorder.supported}
                  {#if !pState}
                    <button class="practice-btn" onclick={() => startPractice(i)}>● Practice answer</button>
                  {:else if pState === 'recording'}
                    <div class="practice-recording-row">
                      <span class="practice-rec-dot">●</span>
                      <span class="practice-live-text">{practiceLiveText[i] || 'Listening…'}</span>
                      <button class="practice-stop-btn" onclick={() => stopPractice(i)}>◼ Stop</button>
                    </div>
                  {:else if pState === 'loading'}
                    <span class="practice-scoring">Scoring<span class="dots">...</span></span>
                  {/if}
                {:else}
                  <span class="practice-unsupported">Voice recording requires Chrome or Edge</span>
                {/if}
                {#if pResult}
                  <div class="practice-result">
                    <div class="pr-header">
                      <span class="pr-score"
                        class:pr-score-good={pResult.confidence_score >= 70}
                        class:pr-score-mid={pResult.confidence_score >= 50 && pResult.confidence_score < 70}
                      >{pResult.confidence_score}</span>
                      <span class="pr-tone">{pResult.tone}</span>
                      <span class="pr-pace">{pResult.pace}</span>
                      <button class="practice-btn practice-retry-btn" onclick={() => startPractice(i)}>↺ Retry</button>
                    </div>
                    {#if pResult.fillers_noted}<div class="pr-fillers">{pResult.fillers_noted}</div>{/if}
                    <div class="pr-coaching">{pResult.coaching}</div>
                  </div>
                {/if}
              </div>
            {/if}
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
              {@const eIsClosing  = entry.tag === 'closing'}

              {#if eIsIntro}
                {#if parsed.present}<div class="e-sec e-sec-present"><span class="cue-badge cue-present">Summary</span><span class="affirm-text">{parsed.present}</span></div>{/if}
                {#if parsed.thread}<div class="e-sec e-sec-thread"><span class="cue-badge cue-thread">Thread</span><span class="affirm-text">{parsed.thread}</span></div>{/if}
                {#if parsed.transition1}<div class="e-transition">{parsed.transition1}</div>{/if}
                {#if parsed.past}<div class="e-sec e-sec-past"><span class="cue-badge cue-past">Story</span><span class="affirm-text">{parsed.past}</span></div>{/if}
                {#if parsed.transition2}<div class="e-transition">{parsed.transition2}</div>{/if}
                {#if parsed.future}<div class="e-sec e-sec-future"><span class="cue-badge cue-future">Next</span><span class="affirm-text">{parsed.future}</span></div>{/if}
                {#if parsed.transition3}<div class="e-transition">{parsed.transition3}</div>{/if}
                {#if parsed.close}<div class="e-sec e-sec-close"><span class="cue-badge cue-close">Close</span><span class="affirm-text">{parsed.close}</span></div>{/if}
                {#if parsed.asks.length > 0}<div class="e-sec e-sec-ask"><span class="cue-badge cue-ask">Ask</span><div class="tp-ask-list">{#each parsed.asks as ask}<div class="tp-ask-item"><div class="tp-ask-content">{#if ask.topic}<span class="tp-ask-topic">{ask.topic}</span>{/if}<span class="tp-ask-question">{ask.question}</span>{#if ask.followUp}<span class="tp-ask-followup">↳ {ask.followUp}</span>{/if}</div></div>{/each}</div></div>{/if}

              {:else if eIsMotiv}
                {#if parsed.company}<div class="e-sec e-sec-company"><span class="cue-badge cue-company">Company</span><span class="affirm-text">{parsed.company}</span></div>{/if}
                {#if parsed.transition1}<div class="e-transition">{parsed.transition1}</div>{/if}
                {#if parsed.role}<div class="e-sec e-sec-role"><span class="cue-badge cue-role">Role</span><span class="affirm-text">{parsed.role}</span></div>{/if}
                {#if parsed.transition2}<div class="e-transition">{parsed.transition2}</div>{/if}
                {#if parsed.self}<div class="e-sec e-sec-self"><span class="cue-badge cue-self">Self</span><span class="affirm-text">{parsed.self}</span></div>{/if}
                {#if parsed.transition3}<div class="e-transition">{parsed.transition3}</div>{/if}
                {#if parsed.close}<div class="e-sec e-sec-close"><span class="cue-badge cue-close">Close</span><span class="affirm-text">{parsed.close}</span></div>{/if}
                {#if parsed.asks.length > 0}<div class="e-sec e-sec-ask"><span class="cue-badge cue-ask">Ask</span><div class="tp-ask-list">{#each parsed.asks as ask}<div class="tp-ask-item"><div class="tp-ask-content">{#if ask.topic}<span class="tp-ask-topic">{ask.topic}</span>{/if}<span class="tp-ask-question">{ask.question}</span>{#if ask.followUp}<span class="tp-ask-followup">↳ {ask.followUp}</span>{/if}</div></div>{/each}</div></div>{/if}

              {:else if eIsFutureTy}
                {#if parsed.direction}<div class="e-sec e-sec-direction"><span class="cue-badge cue-direction">Direction</span><span class="affirm-text">{parsed.direction}</span></div>{/if}
                {#if parsed.transition1}<div class="e-transition">{parsed.transition1}</div>{/if}
                {#if parsed.alignment}<div class="e-sec e-sec-alignment"><span class="cue-badge cue-alignment">Alignment</span><span class="affirm-text">{parsed.alignment}</span></div>{/if}
                {#if parsed.transition2}<div class="e-transition">{parsed.transition2}</div>{/if}
                {#if parsed.contribution}<div class="e-sec e-sec-contribution"><span class="cue-badge cue-contribution">Contribution</span><span class="affirm-text">{parsed.contribution}</span></div>{/if}
                {#if parsed.transition3}<div class="e-transition">{parsed.transition3}</div>{/if}
                {#if parsed.close}<div class="e-sec e-sec-close"><span class="cue-badge cue-close">Close</span><span class="affirm-text">{parsed.close}</span></div>{/if}
                {#if parsed.asks.length > 0}<div class="e-sec e-sec-ask"><span class="cue-badge cue-ask">Ask</span><div class="tp-ask-list">{#each parsed.asks as ask}<div class="tp-ask-item"><div class="tp-ask-content">{#if ask.topic}<span class="tp-ask-topic">{ask.topic}</span>{/if}<span class="tp-ask-question">{ask.question}</span>{#if ask.followUp}<span class="tp-ask-followup">↳ {ask.followUp}</span>{/if}</div></div>{/each}</div></div>{/if}

              {:else if eIsClosing}
                <div class="e-closing-wrap">
                  {#each CLOSING_SECTIONS as sec}
                    {@const isOpen = openClosingSections[i]?.has(sec.key) ?? false}
                    {@const content = closingSectionContent(entry, sec.key)}
                    {@const fetched = sec.key === 'hr' ? entry.closingHRFetched : sec.key === 'hm' ? entry.closingHMFetched : entry.closingCEOFetched}
                    <div class="closing-sec">
                      <!-- svelte-ignore a11y_no_static_element_interactions -->
                      <div class="closing-sec-toggle" onclick={() => toggleClosingSection(i, sec.key)}>
                        <span class="closing-sec-arrow">{isOpen ? '▾' : '▸'}</span>
                        <span class="closing-sec-label">{sec.label}</span>
                      </div>
                      {#if isOpen}
                        {#if fetched && !content}
                          <div class="closing-loading">···</div>
                        {:else if content}
                          {@const secParsed = parseSuggestion(content)}
                          {#each secParsed.asks as ask}
                            <div class="tp-closing-card">
                              <div class="tp-closing-content">
                                <span class="tp-closing-topic">{ask.topic}</span>
                                <span class="tp-closing-question">{ask.question}</span>
                                {#if ask.followUp}<span class="tp-ask-followup">↳ {ask.followUp}</span>{/if}
                              </div>
                            </div>
                          {/each}
                        {/if}
                      {/if}
                    </div>
                  {/each}
                </div>

              {:else}
              <!-- ACKNOWLEDGE -->
              {@const esl = getSectionLabels(eMode === 'compound' ? undefined : entry.tag)}
              {#if parsed.acknowledge}
                <div class="e-sec e-sec-ack">
                  <button class="e-sec-header e-sec-header-toggle" onclick={() => toggleSec(`${i}-ack`)}>
                    <span class="cue-badge cue-ack">{esl.ack}</span>
                    <span class="e-sec-chevron">{collapsedSecs[`${i}-ack`] ? '▸' : '▾'}</span>
                  </button>
                  {#if !collapsedSecs[`${i}-ack`]}<span class="affirm-text">{parsed.acknowledge}</span>{/if}
                </div>
              {/if}
              <!-- SOLVE -->
              {#if parsed.solve}
                <div class="e-sec e-sec-solve">
                  <button class="e-sec-header e-sec-header-toggle" onclick={() => toggleSec(`${i}-solve`)}>
                    <span class="cue-badge cue-solve">{esl.solve}</span>
                    <span class="e-sec-chevron">{collapsedSecs[`${i}-solve`] ? '▸' : '▾'}</span>
                  </button>
                  {#if !collapsedSecs[`${i}-solve`]}
                    {#if parsed.solveStrategies.length > 1 || parsed.solveStrategies[0]?.keyword}
                      <div class="tp-strats-row">
                        {#each parsed.solveStrategies as strategy, si}
                          {@const stratKey = `solve-strat-entry-${i}-${si}`}
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
                      </div>
                    {:else}
                      <span class="affirm-text">{parsed.solve}</span>
                    {/if}
                  {/if}
                </div>
              {/if}
              <!-- BRIDGE -->
              {#if parsed.bridge}
                <div class="e-sec e-sec-bridge">
                  <button class="e-sec-header e-sec-header-toggle" onclick={() => toggleSec(`${i}-bridge`)}>
                    <span class="cue-badge cue-bridge">{esl.bridge}</span>
                    <span class="e-sec-chevron">{collapsedSecs[`${i}-bridge`] ? '▸' : '▾'}</span>
                  </button>
                  {#if !collapsedSecs[`${i}-bridge`]}<span class="affirm-text">{parsed.bridge}</span>{/if}
                </div>
              {/if}
              <!-- ANSWER -->
              {@const _eSayPi = bodyCues.some(c => c.label === 'Pivot' || c.typeTag === 'Pivot')}
              {@const _eSayEx = bodyCues.some(c => c.label === 'Example' || c.typeTag === 'Example')}
              {#if parsed.tell || parsed.strategies.length > 1 || parsed.strategies[0]?.keyword || bodyCues.length > 0 || eModeStreaming}
              <div class="e-sec e-sec-say" class:e-sec-say-example={_eSayEx} class:e-sec-say-pivot={_eSayPi}>
                <button class="e-sec-header e-sec-header-toggle" onclick={() => toggleSec(`${i}-say`)}>
                  <span class="cue-badge" class:cue-say={parsed.cue !== 'Ask'} class:cue-ask={parsed.cue === 'Ask'}>{esl.answer}</span>
                  <span class="e-sec-chevron">{collapsedSecs[`${i}-say`] ? '▸' : '▾'}</span>
                  {#if parsed.tell && !eModeStreaming}
                    {@const secs = estimateSecs(parsed)}
                    <span class="tp-time-est">~{secs < 60 ? secs + 's' : Math.floor(secs/60) + 'm ' + (secs%60) + 's'}</span>
                  {/if}
                </button>
                {#if !collapsedSecs[`${i}-say`]}
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
                {#if bodyCues.length > 0}
                  <div class="e-cues">
                    {#each bodyCues as cue}
                      {@const isFlat = cue.label === 'Pivot' || cue.typeTag === 'Pivot'}
                      {#if isFlat}
                        <div class="tp-cue-flat">
                          <span class="cue-label-sm" class:cue-label-transfer={cue.typeTag === 'Pivot' || cue.label === 'Pivot'}>{cue.typeTag || (cue.label === 'General' ? 'Point' : cue.label)}</span>
                          <span class="tp-cue-flat-text">{cue.text}</span>
                        </div>
                      {:else}
                        <div class="tp-cue-block" class:tp-cue-open={!!openCues[cue.text]}>
                          <button class="tp-cue-toggle" onclick={() => { const opening = !openCues[cue.text]; toggleCueOpen(cue.text); if (opening) expandCue(entry.question, cue.text); }}>
                            <span class="tp-cue-preview">{cue.title || (cue.text.split(/(?<=[.!?])\s+/)[0] ?? cue.text)}</span>
                            <span class="tp-cue-chevron">{openCues[cue.text] ? '▾' : '▸'}</span>
                          </button>
                          {#if openCues[cue.text]}
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
                {/if}
              </div>
              {/if}<!-- end Answer section guard -->
              <!-- CLOSE -->
              {#if parsed.close}
                <div class="e-sec e-sec-close">
                  <span class="cue-badge cue-close">{esl.close}</span>
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
                          {#if ask.topic}<span class="tp-ask-topic">{ask.topic}</span>{/if}
                          <span class="tp-ask-question">{ask.question}</span>
                          {#if ask.followUp}<span class="tp-ask-followup">↳ {ask.followUp}</span>{/if}
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
    line-height: 1.2;
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
  .cue-badge.cue-solve  { background: #292524; color: #a8a29e; }
  .cue-badge.cue-bridge { background: #292524; color: #a8a29e; }
  .cue-badge.cue-close  { background: #1e3a5f; color: #93c5fd; }

  /* Acknowledge text (purple section) */
  .tp-ack-text {
    color: #ffffff;
    font-size: var(--fs-lg);
    line-height: 1.2;
    overflow-wrap: break-word;
    flex: 1;
  }

  /* Solve text (teal section) */
  .tp-solve-text {
    color: #ffffff;
    font-size: var(--fs-lg);
    line-height: 1.2;
    overflow-wrap: break-word;
    flex: 1;
  }

  /* Bridge text (stone section) */
  .tp-bridge-text {
    color: #d6d3d1;
    font-size: var(--fs-lg);
    line-height: 1.2;
    overflow-wrap: break-word;
    flex: 1;
  }

  /* Close text (blue section) */
  .tp-close-text {
    color: #ffffff;
    font-size: var(--fs-lg);
    line-height: 1.2;
    overflow-wrap: break-word;
    flex: 1;
  }

  /* Answer header row with time estimate */
  .tp-sec-header { display: flex; align-items: center; gap: 0.5rem; }
  .e-sec-header  { display: flex; align-items: center; gap: 0.5rem; }
  .e-sec-header-toggle { cursor: pointer; border-radius: 0.2rem; padding: 0.1rem 0.2rem; margin: -0.1rem -0.2rem; background: none; border: none; text-align: left; width: 100%; }
  .e-sec-header-toggle:hover { background: rgba(255,255,255,0.04); }
  .e-sec-chevron { font-size: var(--fs-xs); color: #475569; }
  .tp-time-est {
    font-size: var(--fs-xs); color: #334155;
    font-variant-numeric: tabular-nums; 
  }

  /* Ask inline list */
  .tp-ask-list { display: flex; flex-direction: column; gap: 0.35rem; }
  .tp-ask-item { display: flex; align-items: flex-start; gap: 0.4rem; }
  .tp-ask-content { display: flex; flex-direction: column; gap: 0.15rem; flex: 1; }
  .tp-ask-topic { font-size: var(--fs-sm); color: #f59e0b; font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; }
  .tp-ask-question { font-size: var(--fs-lg); color: #e2e8f0; line-height: 1.2; overflow-wrap: break-word; }
  .tp-ask-followup { font-size: var(--fs-sm); color: #94a3b8; line-height: 1.2; overflow-wrap: break-word;  }

  /* Breadcrumb: Q type · A framework */
  .tp-breadcrumb {
    display: flex; align-items: center; gap: 0.35rem;
    padding: 0.2rem 0 0.35rem;
    font-size: var(--fs-xs); font-weight: 700;
    text-transform: uppercase; letter-spacing: 0.05em;
  }
  .tp-bc-q { /* colored per tag, set inline */ }
  .tp-bc-sep { color: #334155; }
  .tp-bc-a { color: #475569; font-weight: 600; }

  /* Transition connector lines */
  .tp-transition {
    font-size: var(--fs-sm); color: #475569; 
    padding: 0.05rem 0 0.05rem 1.1rem; line-height: 1.2;
    overflow-wrap: break-word;
  }
  .e-transition {
    font-size: var(--fs-sm); color: #475569; 
    padding: 0.1rem 0 0.1rem 0.75rem; line-height: 1.2;
    overflow-wrap: break-word;
  }

  /* Answer type label on standard entries */
  .entry-ans-type {
    font-size: var(--fs-xs); font-weight: 600; color: #475569;
    letter-spacing: 0.03em; white-space: nowrap;
    padding: 0.05em 0.4em; background: #080d18;
    border: 1px solid #1a2540; border-radius: 0.2em;
    flex-shrink: 0;
  }

  /* Say text */
  .tp-tell {
    color: #ffffff;
    font-size: var(--fs-lg);
    line-height: 1.2;
    flex: 1;
    overflow-wrap: break-word;
    word-break: break-word;
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
  }
  .tp-tell-sent { display: block; line-height: 1.2; }
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
    color: #cbd5e1;
    font-size: var(--fs-base);
    line-height: 1.2;
    white-space: pre-wrap;
    border-top: 1px solid #0d2010;
    padding-top: 0.5rem;
    overflow-wrap: break-word;
    word-break: break-word;
  }
  :global(.tp-body strong) { color: #b8cce4; font-weight: 700; }

  /* Cue bullets (inside Say section) */
  .tp-cues, .e-cues { display: flex; flex-direction: column; gap: 0.2rem; border-top: 1px solid #0d2010; padding-top: 0.35rem; }
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
    flex: 1; font-size: var(--fs-lg); color: #94a3b8;
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
    font-size: var(--fs-lg); line-height: 1.2; font-weight: 400;
    overflow-wrap: break-word;
  }
  .tp-cue-flat { display: flex; align-items: center; gap: 0.4rem; padding: 0.15rem 0.4rem; }
  .tp-cue-flat-text { flex: 1; font-size: var(--fs-base); color: #94a3b8; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .tp-sec > .cue-badge, .e-sec > .cue-badge { align-self: flex-start; }
  .cue-badge.cue-toggle { appearance: none; border: none; cursor: pointer; font-family: inherit; font-size: inherit; font-weight: inherit; letter-spacing: inherit; user-select: none; }
  .cue-badge.cue-toggle:hover { filter: brightness(1.2); }
  .cue-type-tag { display: inline-block; font-size: var(--fs-xs); font-weight: 800; padding: 0.1rem 0.4rem; border-radius: 0.25rem; background: #14532d; color: #4ade80; text-transform: uppercase; letter-spacing: 0.06em; flex-shrink: 0; }
  .cue-type-example { background: #1a3a1a; color: #86efac; }
  .cue-type-pivot { background: #3b1506; color: #fb923c; }
  .cue-label-example { }
  .cue-label-transfer { color: #fb923c !important; }
  .cue-loading { color: #334155; }

  /* Ask cue-block theming (amber) */
  .cue-label-ask { color: #fbbf24 !important; }
  .tp-ask-preview {
    color: #94a3b8 !important;
    white-space: normal !important;
    overflow: visible !important;
    text-overflow: unset !important;
  }
  .ask-sentence {
    padding: 0.3rem 0.4rem; background: #060300; border-left: 2px solid #92400e;
    border-radius: 0 0.25rem 0.25rem 0; color: #f1f5f9;
    font-size: var(--fs-lg); line-height: 1.2; font-weight: 400; overflow-wrap: break-word;
  }

  .tp-hint {
    font-size: var(--fs-xs);
    color: #1e293b;
    
    flex-shrink: 0;
    text-align: center;
  }
  .tp-loading {
    color: #4d94d4;  font-size: var(--fs-base);
  }
  .tp-raw-fallback { display: flex; flex-direction: column; gap: 0.4rem; padding: 0.25rem 0; }
  .tp-raw-line { font-size: var(--fs-sm); color: #94a3b8; line-height: 1.2; margin: 0; }
  .tp-loading-pending {
    color: #334155;
  }
  .tp-empty {
    flex: 1; display: flex; align-items: center; justify-content: center;
    color: #1e293b;  font-size: var(--fs-base);
  }

  /* Question tag */
  .tp-tag {
    display: inline-block; padding: 0.08rem 0.4rem; border-radius: 0.2rem;
    font-size: var(--fs-xs); font-weight: 600; text-transform: uppercase;
    letter-spacing: 0.03em; flex-shrink: 0; margin-top: 0.05rem;
    color: #475569; background: #080d18; border: 1px solid #1a2540;
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
  .redflag-note { font-size: var(--fs-sm); color: #94a3b8; line-height: 1.2;  }

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
    line-height: 1.2; overflow-wrap: break-word; word-break: break-word;
  }
  .entry-tag {
    display: inline-block; padding: 0.05rem 0.35rem; border-radius: 0.2rem;
    font-size: var(--fs-xs); font-weight: 600; text-transform: uppercase;
    letter-spacing: 0.03em; flex-shrink: 0; align-self: flex-start; margin-top: 0.15rem;
    color: #475569; background: #080d18; border: 1px solid #1a2540;
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
    line-height: 1.2; overflow-wrap: break-word; word-break: break-word;
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
    color: #7a9ab8; line-height: 1.2; white-space: pre-wrap;
    font-size: var(--fs-base); border-top: 1px solid #0d2010; padding-top: 0.4rem;
    overflow-wrap: break-word; word-break: break-word;
  }
  :global(.body-text strong) { color: #b8cce4; font-weight: 700; }

  .loading { color: #60a5fa;  font-size: var(--fs-base); }
  .empty {
    color: #475569;  font-size: var(--fs-base);
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
    line-height: 1.2;
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

  /* Closing question collapsible sections */
  .closing-sec { display: flex; flex-direction: column; }
  .closing-sec-toggle {
    display: flex; align-items: center; gap: 0.35rem;
    padding: 0.3rem 0.25rem; cursor: pointer; user-select: none;
    border-bottom: 1px solid #1e293b;
  }
  .closing-sec-toggle:hover .closing-sec-label { color: #94a3b8; }
  .closing-sec-arrow { font-size: 0.65rem; color: #334155; width: 0.7rem; }
  .closing-sec-label {
    font-size: var(--fs-xs); font-weight: 800; text-transform: uppercase;
    letter-spacing: 0.08em; color: #475569; transition: color 0.12s;
  }
  .closing-loading {
    padding: 0.4rem 0.75rem; color: #334155; font-size: var(--fs-sm);
    letter-spacing: 0.2em;
  }

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
    font-size: var(--fs-lg); color: #e2e8f0; line-height: 1.2; overflow-wrap: break-word;
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
  .tp-strats-row {
    display: flex; flex-wrap: wrap; gap: 0.3rem; align-items: flex-start;
    min-width: 0;
  }
  .tp-strat-block {
    display: inline-flex; flex-direction: column;
    border-radius: 0.3rem; border: 1px solid #0d2010;
    overflow: hidden; background: #040b06;
    min-width: 0;
  }
  .tp-strat-block.tp-strat-open { border-color: #1e4a2a; }
  .tp-strat-toggle {
    display: flex; align-items: flex-start; gap: 0.4rem;
    padding: 0.3rem 0.5rem;
    background: none; border: none; cursor: pointer; text-align: left;
  }
  .tp-strat-toggle:hover { background: #071a0f; }
  .tp-strat-kw {
    font-size: var(--fs-xs); font-weight: 800; text-transform: uppercase;
    letter-spacing: 0.06em; color: #4ade80; background: #14532d;
    padding: 0.05rem 0.35rem; border-radius: 0.2rem;
    white-space: normal; word-break: break-word; flex-shrink: 1; min-width: 0;
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
    color: #f1f5f9; font-size: var(--fs-lg); line-height: 1.2; display: block;
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
    background: #071428;
    border: 1px solid #1e3a5f;
    border-radius: 0.3rem;
    font-size: var(--fs-xs);
    font-weight: 700;
    color: #64748b;
    cursor: pointer;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    transition: all 0.1s;
  }
  .mode-tab:hover { background: #0d1a2e; border-color: #3b5998; color: #94a3b8; }
  .mode-tab.mode-tab-active {
    background: #071a0d;
    border-color: #166534;
    color: #4ade80;
  }

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
    line-height: 1.2;
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
    
    font-size: var(--fs-xs);
    border-top: 1px solid #0f1e30;
  }

  /* ── Practice recording ─────────────────────────────────── */
  .practice-bar {
    margin-top: 0.4rem;
    border-top: 1px solid #0f1e30;
    padding-top: 0.4rem;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .practice-btn {
    align-self: flex-start;
    background: #0a1a2e;
    border: 1px solid #1e3a5f;
    color: #60a5fa;
    font-size: var(--fs-xs);
    font-weight: 700;
    padding: 0.2rem 0.55rem;
    border-radius: 0.25rem;
    cursor: pointer;
    transition: all 0.12s;
  }
  .practice-btn:hover { background: #0f2847; border-color: #3b82f6; color: #93c5fd; }

  .practice-recording-row {
    display: flex;
    align-items: flex-start;
    gap: 0.4rem;
  }

  .practice-rec-dot {
    color: #ef4444;
    font-size: 0.55rem;
    flex-shrink: 0;
    margin-top: 0.25rem;
    animation: pulse 1s ease-in-out infinite;
  }

  .practice-live-text {
    flex: 1;
    font-size: var(--fs-xs);
    color: #64748b;
    
    line-height: 1.2;
    overflow-wrap: break-word;
    word-break: break-word;
  }

  .practice-stop-btn {
    flex-shrink: 0;
    background: #2d0a0a;
    border: 1px solid #7f1d1d;
    color: #f87171;
    font-size: var(--fs-xs);
    font-weight: 700;
    padding: 0.2rem 0.5rem;
    border-radius: 0.25rem;
    cursor: pointer;
    transition: all 0.12s;
  }
  .practice-stop-btn:hover { background: #450a0a; }

  .practice-unsupported { font-size: var(--fs-xs); color: #334155;  }
  .practice-scoring {
    font-size: var(--fs-xs);
    color: #475569;
    
  }

  .practice-result {
    background: #060f1e;
    border: 1px solid #1e2d45;
    border-radius: 0.35rem;
    padding: 0.4rem 0.5rem;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .pr-header {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    flex-wrap: wrap;
  }

  .pr-score {
    font-size: var(--fs-xs);
    font-weight: 800;
    color: #f87171;
    background: #1a0505;
    border-radius: 0.2em;
    padding: 0.05em 0.4em;
    flex-shrink: 0;
  }
  .pr-score.pr-score-good { color: #4ade80; background: #071a0f; }
  .pr-score.pr-score-mid  { color: #fbbf24; background: #1a1200; }

  .pr-tone {
    font-size: var(--fs-xs);
    font-weight: 600;
    color: #94a3b8;
    text-transform: capitalize;
  }

  .pr-pace {
    font-size: var(--fs-xs);
    color: #64748b;
  }

  .practice-retry-btn {
    margin-left: auto;
    padding: 0.1rem 0.4rem;
    font-size: var(--fs-xs);
    font-weight: 600;
  }

  .pr-fillers {
    font-size: var(--fs-xs);
    color: #f59e0b;
  }

  .pr-coaching {
    font-size: var(--fs-xs);
    color: #94a3b8;
    line-height: 1.2;
  }
</style>
