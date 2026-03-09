<script lang="ts">
  import type { SuggestionEntry } from '../lib/types';

  const { suggestions, onClear } = $props<{
    suggestions: SuggestionEntry[];
    onClear: () => void;
  }>();

  let container: HTMLElement;

  $effect(() => {
    if (suggestions.length && container) {
      container.scrollTop = container.scrollHeight;
    }
  });
</script>

<div class="suggestion-panel">
  <div class="panel-header">
    <h3>AI Suggestions</h3>
    {#if suggestions.length > 0}
      <button class="clear-btn" onclick={onClear}>Clear</button>
    {/if}
  </div>

  <div class="entries" bind:this={container}>
    {#if suggestions.length === 0}
      <p class="empty">
        Suggestions will appear when the interviewer speaks...
      </p>
    {:else}
      {#each suggestions as entry, i (i)}
        <div class="entry" class:latest={i === suggestions.length - 1}>
          <div class="question-row">
            <span class="badge">Interviewer</span>
            <p class="question-text">"{entry.question}"</p>
          </div>
          <div class="suggestion-text">
            {#if entry.suggestion}
              {entry.suggestion}{#if entry.streaming}<span class="cursor">|</span>{/if}
            {:else if entry.streaming}
              <span class="loading">Generating<span class="dots">...</span></span>
            {/if}
          </div>
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .suggestion-panel {
    height: 100%;
    display: flex;
    flex-direction: column;
  }
  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.75rem;
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
    color: #e2e8f0;
    line-height: 1.7;
    white-space: pre-wrap;
    font-size: 0.9rem;
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
