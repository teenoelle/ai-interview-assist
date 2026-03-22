<script lang="ts">
  import type { TranscriptEntry } from '../lib/types';
  import PanelHeader from './PanelHeader.svelte';

  const { entries, onFlipSpeaker } = $props<{ entries: TranscriptEntry[]; onFlipSpeaker?: (idx: number) => void }>();

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
    <PanelHeader title="Transcript" actionLabel="↓ Export" onAction={exportTranscript} />
  {/if}
  <div class="entries" bind:this={container}>
    {#if entries.length === 0}
      <p class="empty">Transcript will appear here when audio is captured...</p>
    {:else}
      {#each entries as entry, i (entry.timestamp_ms)}
        <div class="entry" class:interviewer={entry.speaker === 'Interviewer'} class:you={entry.speaker === 'You'}>
          <div class="meta">
            <span class="speaker" class:interviewer={entry.speaker === 'Interviewer'}>{entry.speaker}</span>
            <span class="time">{formatTime(entry.timestamp_ms)}</span>
            {#if onFlipSpeaker}
              <button class="flip-btn" title="Flip speaker" onclick={() => onFlipSpeaker(i)}>⇄</button>
            {/if}
          </div>
          <span class="text">{entry.text}</span>
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .transcript-panel { height: 100%; display: flex; flex-direction: column; }
  .entries { flex: 1; overflow-y: auto; display: flex; flex-direction: column; gap: 0.5rem; }
  .entry {
    display: flex; flex-direction: column; gap: 0.2rem;
    padding: 0.5rem 0.75rem;
    border-radius: var(--radius-lg, 0.5rem);
    border-left: 3px solid transparent;
  }
  .entry.interviewer { border-left-color: var(--clr-speaker-them, #f87171); background: var(--bg-entry-them, #1a0a0a); }
  .entry.you         { border-left-color: var(--clr-speaker-you, #4ade80);  background: var(--bg-entry-you, #0a1f14); }
  .meta { display: flex; align-items: center; gap: 0.5rem; }
  .speaker {
    font-size: var(--fs-sm); font-weight: 700; text-transform: uppercase;
    letter-spacing: 0.07em; color: var(--clr-speaker-you, #4ade80);
  }
  .speaker.interviewer { color: var(--clr-speaker-them, #f87171); }
  .time { color: var(--clr-text-muted, #475569); font-size: var(--fs-sm); font-family: var(--ff-mono); }
  .flip-btn {
    background: none; border: none; color: #334155; font-size: var(--fs-sm);
    cursor: pointer; padding: 0 0.2rem; line-height: 1; opacity: 0;
    transition: opacity 0.15s, color 0.15s; margin-left: auto;
  }
  .entry:hover .flip-btn { opacity: 1; }
  .flip-btn:hover { color: #60a5fa; }
  .text { color: #cbd5e1; font-size: var(--fs-base); line-height: 1.5; }
  .empty { color: var(--clr-text-muted, #475569); font-style: italic; text-align: center; padding: 2rem; }
</style>
