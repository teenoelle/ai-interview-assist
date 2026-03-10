<script lang="ts">
  import type { SuggestionEntry } from '../lib/types';

  const { suggestions, onClear, teleprompter = false } = $props<{
    suggestions: SuggestionEntry[];
    onClear: () => void;
    teleprompter?: boolean;
  }>();

  let container: HTMLElement;

  $effect(() => {
    if (!teleprompter && suggestions.length && container) {
      container.scrollTop = container.scrollHeight;
    }
  });

  function renderBold(text: string): string {
    return text.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>');
  }

  const latest = $derived(suggestions.length > 0 ? suggestions[suggestions.length - 1] : null);
  const previous = $derived(suggestions.length > 1 ? suggestions.slice(0, -1) : []);
</script>

{#if teleprompter}
  <!-- Teleprompter mode: latest suggestion large + centered under webcam -->
  <div class="teleprompter">
    <div class="tp-header">
      <span class="tp-hint">glance at bold — don't read aloud</span>
      {#if suggestions.length > 0}
        <button class="tp-clear" onclick={onClear}>Clear</button>
      {/if}
    </div>

    {#if latest}
      <div class="tp-card">
        <div class="tp-question">"{latest.question}"</div>
        <div class="tp-suggestion">
          {#if latest.suggestion}
            {@html renderBold(latest.suggestion)}{#if latest.streaming}<span class="cursor">|</span>{/if}
          {:else if latest.streaming}
            <span class="tp-loading">Generating<span class="dots">...</span></span>
          {/if}
        </div>
      </div>
    {:else}
      <div class="tp-empty">Waiting for a question...</div>
    {/if}

    {#if previous.length > 0}
      <div class="tp-history">
        {#each previous as entry, i (i)}
          <div class="tp-history-item" title={entry.question}>
            Q{i + 1}: {entry.question}
          </div>
        {/each}
      </div>
    {/if}
  </div>

{:else}
  <!-- Standard panel mode -->
  <div class="suggestion-panel">
    <div class="panel-header">
      <div class="header-left">
        <h3>AI Suggestions</h3>
        <span class="glance-hint">glance at bold keywords — don't read aloud</span>
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
          <div class="entry" class:latest={i === suggestions.length - 1}>
            <div class="question-row">
              <span class="badge">Interviewer</span>
              <p class="question-text">"{entry.question}"</p>
            </div>
            <div class="suggestion-text">
              {#if entry.suggestion}
                {@html renderBold(entry.suggestion)}{#if entry.streaming}<span class="cursor">|</span>{/if}
              {:else if entry.streaming}
                <span class="loading">Generating<span class="dots">...</span></span>
              {/if}
            </div>
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
    gap: 0.6rem;
  }

  .tp-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-shrink: 0;
  }

  .tp-hint {
    font-size: 0.62rem;
    color: #334155;
    font-style: italic;
  }

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
    gap: 1rem;
    padding: 1.25rem 1.5rem;
    background: #07101e;
    border-radius: 0.75rem;
    border: 1px solid #1a2d4a;
    overflow-y: auto;
    min-height: 0;
  }

  .tp-question {
    color: #4d94d4;
    font-style: italic;
    font-size: 0.88rem;
    line-height: 1.5;
    padding-bottom: 0.75rem;
    border-bottom: 1px solid #0f1e33;
    flex-shrink: 0;
  }

  .tp-suggestion {
    color: #b8cce4;
    line-height: 2.3;
    white-space: pre-wrap;
    font-size: 1rem;
  }

  :global(.tp-suggestion strong) {
    color: #f0f6ff;
    font-size: 1.25rem;
    font-weight: 800;
    letter-spacing: 0.01em;
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

  .tp-history {
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    max-height: 4.5rem;
    overflow-y: auto;
  }

  .tp-history-item {
    font-size: 0.65rem;
    color: #1e3a5f;
    padding: 0.12rem 0.5rem;
    background: #06101a;
    border-radius: 0.2rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
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
    font-size: 0.65rem;
    color: #475569;
    font-style: italic;
    text-transform: none;
    letter-spacing: 0;
  }
  h3 {
    font-size: 1rem;
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
    padding: 0.75rem;
    background: #0f1e33;
    border-radius: 0.5rem;
    border-left: 3px solid #334155;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
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
    padding: 0.15rem 0.5rem;
    background: #1d4ed8;
    color: white;
    border-radius: 9999px;
    font-size: 0.65rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin-top: 0.1rem;
  }
  .question-text {
    color: #93c5fd;
    font-style: italic;
    font-size: 0.85rem;
    margin: 0;
    line-height: 1.4;
  }

  .suggestion-text {
    color: #cbd5e1;
    line-height: 1.9;
    white-space: pre-wrap;
    font-size: 0.9rem;
  }
  :global(.suggestion-text strong) {
    color: #f1f5f9;
    font-size: 1rem;
    font-weight: 700;
    letter-spacing: 0.01em;
  }

  .cursor {
    animation: blink 1s step-end infinite;
  }
  @keyframes blink { 50% { opacity: 0; } }

  .loading {
    color: #60a5fa;
    font-style: italic;
    font-size: 0.85rem;
  }

  .empty {
    color: #475569;
    font-style: italic;
    font-size: 0.85rem;
    text-align: center;
    padding: 2rem 1rem;
  }
</style>
