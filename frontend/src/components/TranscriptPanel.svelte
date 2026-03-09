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
        <div class="entry" class:interviewer={entry.speaker === 'Interviewer'} class:you={entry.speaker === 'You'}>
          <div class="meta">
            <span class="speaker" class:interviewer={entry.speaker === 'Interviewer'}>{entry.speaker}</span>
            <span class="time">{formatTime(entry.timestamp_ms)}</span>
          </div>
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
    flex-direction: column;
    gap: 0.2rem;
    padding: 0.5rem 0.75rem;
    border-radius: 0.5rem;
    border-left: 3px solid transparent;
  }
  .entry.interviewer { border-left-color: #60a5fa; background: #0f1e33; }
  .entry.you         { border-left-color: #4ade80; background: #0a1f14; }
  .meta {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .speaker {
    font-size: 0.7rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: #4ade80;
  }
  .speaker.interviewer { color: #60a5fa; }
  .time {
    color: #475569;
    font-size: 0.7rem;
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
