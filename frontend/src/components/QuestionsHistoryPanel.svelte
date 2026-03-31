<script lang="ts">
  import type { SuggestionEntry } from '../lib/types';
  import { TAG_CONFIG } from '../lib/questionTagger';
  import { parseSuggestion, getAnswerType } from '../lib/parseSuggestion';

  const { suggestions, currentIndex, onJump, scrollToLatestKey = 0 } = $props<{
    suggestions: SuggestionEntry[];
    currentIndex: number;
    onJump: (i: number) => void;
    scrollToLatestKey?: number;
  }>();

  let listEl: HTMLElement | undefined = $state();

  $effect(() => {
    if (scrollToLatestKey && listEl) {
      listEl.scrollTop = listEl.scrollHeight;
    }
  });

</script>

<div class="qhist">
  {#if suggestions.length === 0}
    <p class="qhist-empty">Questions will appear here as they are detected.</p>
  {:else}
    <div class="qhist-list" bind:this={listEl}>
      {#each suggestions as entry, i (i)}
        <button
          class="qhist-item"
          class:active={i === currentIndex}
          onclick={() => onJump(i)}
        >
          <div class="qhist-top">
            {#if entry.tag}
              {@const tc = TAG_CONFIG[entry.tag]}
              <span class="qhist-tag">{tc.label}</span>
            {/if}
            {#if entry.secondaryTag}
              {@const tc2 = TAG_CONFIG[entry.secondaryTag]}
              <span class="qhist-tag qhist-tag-secondary" title="Also: {tc2.label}">+{tc2.label}</span>
            {/if}
            {#if entry.suggestion && !entry.streaming}
              {@const at = getAnswerType(parseSuggestion(entry.suggestion), entry.tag)}
              {#if at.framework}<span class="qhist-ans-type">{at.framework}</span>{/if}
            {/if}
          </div>
          <div class="qhist-q">{entry.question}</div>
          {#if entry.redFlag}
            <div class="qhist-redflag">⚠ {entry.redFlag.category}</div>
          {/if}
          {#if entry.answerFeedback || (entry.answered === true && entry.vocalFeedback && entry.vocalFeedback.confidence_score >= 70)}
            <div class="qhist-feedback">
              {#if entry.answerFeedback}
                {#if entry.answerFeedback.missed_followup}<span class="fb-chip">No follow-up</span>{/if}
              {/if}
              {#if entry.answered === true && entry.vocalFeedback && entry.vocalFeedback.confidence_score >= 70}
                <span class="fb-chip-good">✓ Strong</span>
              {/if}
            </div>
          {/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .qhist {
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .qhist-empty {
    color: #334155;
    font-style: italic;
    font-size: var(--fs-sm);
    text-align: center;
    padding: 2rem 0.75rem;
    line-height: 1.5;
    margin: 0;
  }

  .qhist-list {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    padding: 0.25rem 0;
    scrollbar-width: thin;
    scrollbar-color: #1e293b transparent;
  }

  .qhist-item {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    padding: 0.5rem 0.6rem;
    background: #080d18;
    border: 1px solid #0f172a;
    border-radius: 0.4rem;
    cursor: pointer;
    text-align: left;
    transition: all 0.12s;
    width: 100%;
  }
  .qhist-item:hover {
    background: #0d1525;
    border-color: #1e2d45;
  }
  .qhist-item.active {
    background: #0a1f12;
    border-color: #166534;
    border-left: 4px solid #4ade80;
    box-shadow: 0 0 0 1px #14532d;
  }

  .qhist-top {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    flex-wrap: nowrap;
  }

.qhist-tag {
    font-size: var(--fs-xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.03em;
    padding: 0.05em 0.35em;
    border-radius: 0.2em;
    flex-shrink: 0;
    color: #475569;
    background: #080d18;
    border: 1px solid #1a2540;
  }
  .qhist-tag-secondary {
    opacity: 0.7;
  }

  .qhist-ans-type {
    font-size: var(--fs-xs);
    font-weight: 600;
    color: #475569;
    letter-spacing: 0.03em;
    padding: 0.05em 0.3em;
    background: #080d18;
    border: 1px solid #1a2540;
    border-radius: 0.2em;
    flex-shrink: 0;
  }

  .qhist-status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
    margin-left: auto;
    background: #1e293b;
  }
  .qhist-status-dot.dot-answered { background: #22c55e; }
  .qhist-status-dot.dot-unanswered { background: #f59e0b; }
  .qhist-status-dot.dot-active {
    background: #60a5fa;
    animation: pulse 1.5s ease-in-out infinite;
  }
  @keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.4; } }

  .qhist-q {
    font-size: var(--fs-sm);
    color: #c87070;
    line-height: 1.35;
    overflow-wrap: break-word;
    word-break: break-word;
  }
  .qhist-item.active .qhist-q { color: #fca5a5; font-weight: 600; }

  .qhist-redflag {
    font-size: var(--fs-xs);
    color: #b45309;
    font-weight: 600;
  }

  .qhist-feedback {
    display: flex;
    gap: 0.25rem;
    flex-wrap: wrap;
    margin-top: 0.1rem;
  }
  .fb-chip-good {
    font-size: var(--fs-xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #4ade80;
    background: #071a0f;
    border-radius: 0.2em;
    padding: 0.05em 0.35em;
  }
  .fb-chip {
    font-size: var(--fs-xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #fb923c;
    background: #431407;
    border-radius: 0.2em;
    padding: 0.05em 0.35em;
  }
</style>
