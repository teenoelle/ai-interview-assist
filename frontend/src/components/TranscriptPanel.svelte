<script lang="ts">
  import type { TranscriptEntry } from '../lib/types';

  const { entries } = $props<{ entries: TranscriptEntry[] }>();

  let container: HTMLElement;

  $effect(() => {
    if (entries.length && container) {
      container.scrollTop = container.scrollHeight;
    }
  });

  function formatTime(ms: number): string {
    const d = new Date(ms);
    return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
  }
</script>

<div class="transcript-panel">
  <h3>Live Transcript</h3>
  <div class="entries" bind:this={container}>
    {#if entries.length === 0}
      <p class="empty">Transcript will appear here when audio is captured...</p>
    {:else}
      {#each entries as entry (entry.timestamp_ms)}
        <div class="entry">
          <span class="time">{formatTime(entry.timestamp_ms)}</span>
          <span class="text">{entry.text}</span>
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .transcript-panel {
    height: 100%;
    display: flex;
    flex-direction: column;
  }
  h3 {
    font-size: 1rem;
    font-weight: 600;
    color: #94a3b8;
    margin-bottom: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .entries {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .entry {
    display: flex;
    gap: 0.75rem;
    padding: 0.5rem 0;
    border-bottom: 1px solid #1e293b;
  }
  .time {
    flex-shrink: 0;
    color: #475569;
    font-size: 0.75rem;
    padding-top: 2px;
    font-family: monospace;
  }
  .text {
    color: #cbd5e1;
    font-size: 0.9rem;
    line-height: 1.5;
  }
  .empty {
    color: #475569;
    font-style: italic;
    text-align: center;
    padding: 2rem;
  }
</style>
