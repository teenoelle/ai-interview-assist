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

  function exportTranscript() {
    const lines = entries.map(e => `[${formatTime(e.timestamp_ms)}] ${e.speaker}: ${e.text}`);
    const blob = new Blob([lines.join('\n')], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `interview-transcript-${new Date().toISOString().slice(0,10)}.txt`;
    a.click();
    URL.revokeObjectURL(url);
  }
</script>

<div class="transcript-panel">
  {#if entries.length > 0}
    <div class="panel-header">
      <button class="export-btn" onclick={exportTranscript} title="Download transcript">↓ Export</button>
    </div>
  {/if}
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
  .transcript-panel { height: 100%; display: flex; flex-direction: column; }
  .panel-header {
    display: flex; align-items: center; justify-content: space-between;
    margin-bottom: 0.75rem;
  }
  h3 {
    font-size: 1rem; font-weight: 600; color: #94a3b8; margin: 0;
    text-transform: uppercase; letter-spacing: 0.05em;
  }
  .export-btn {
    padding: 0.15rem 0.5rem; background: transparent;
    border: 1px solid #334155; border-radius: 0.25rem;
    color: #64748b; font-size: var(--fs-sm); cursor: pointer;
  }
  .export-btn:hover { border-color: #60a5fa; color: #60a5fa; }
  .entries { flex: 1; overflow-y: auto; display: flex; flex-direction: column; gap: 0.5rem; }
  .entry {
    display: flex; flex-direction: column; gap: 0.2rem;
    padding: 0.5rem 0.75rem; border-radius: 0.5rem; border-left: 3px solid transparent;
  }
  .entry.interviewer { border-left-color: #f87171; background: #1a0a0a; }
  .entry.you         { border-left-color: #4ade80; background: #0a1f14; }
  .meta { display: flex; align-items: center; gap: 0.5rem; }
  .speaker {
    font-size: var(--fs-sm); font-weight: 700; text-transform: uppercase;
    letter-spacing: 0.07em; color: #4ade80;
  }
  .speaker.interviewer { color: #f87171; }
  .time { color: #475569; font-size: var(--fs-sm); font-family: var(--ff-mono); }
  .text { color: #cbd5e1; font-size: var(--fs-base); line-height: 1.5; }
  .empty { color: #475569; font-style: italic; text-align: center; padding: 2rem; }
</style>
