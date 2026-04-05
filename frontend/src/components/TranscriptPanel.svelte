<script lang="ts">
  import { tick } from 'svelte';
  import type { TranscriptEntry } from '../lib/types';
  import PanelHeader from './PanelHeader.svelte';
  import { FILLER_RE } from '../lib/filler';

  const { entries, onFlipSpeaker, jdKeywords = [] } = $props<{
    entries: TranscriptEntry[];
    onFlipSpeaker?: (idx: number) => void;
    jdKeywords?: string[];
  }>();

  let container: HTMLElement;
  // Track whether the user has scrolled up to read history — if so, don't auto-scroll.
  let userScrolledUp = false;

  function onScroll() {
    if (!container) return;
    const distFromBottom = container.scrollHeight - container.scrollTop - container.clientHeight;
    userScrolledUp = distFromBottom > 80;
  }

  $effect(() => {
    if (entries.length && container && !userScrolledUp) {
      tick().then(() => {
        container.scrollTop = container.scrollHeight;
      });
    }
  });

  function formatTime(ms: number): string {
    const d = new Date(ms);
    return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
  }

  function wordCount(text: string): number {
    return text.trim().split(/\s+/).filter(Boolean).length;
  }

  function escapeHtml(s: string): string {
    return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  }

  function highlightText(text: string, isYou: boolean): string {
    let html = escapeHtml(text);

    // JD keyword highlighting (both speakers)
    if (jdKeywords.length > 0) {
      const kwPattern = jdKeywords
        .filter(k => k.length > 2)
        .map(k => k.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'))
        .join('|');
      if (kwPattern) {
        html = html.replace(new RegExp(`\\b(${kwPattern})\\b`, 'gi'),
          '<mark class="kw-hit">$1</mark>');
      }
    }

    // Filler word highlighting (You speaker only)
    if (isYou) {
      html = html.replace(FILLER_RE, '<mark class="filler-hit">$&</mark>');
    }

    return html;
  }

  function fillerCount(text: string): number {
    return (text.match(FILLER_RE) ?? []).length;
  }

  function wpmLabel(text: string, durationMs: number): string {
    if (durationMs < 1000) return '';
    const wpm = Math.round(wordCount(text) / (durationMs / 60000));
    return wpm > 10 && wpm < 400 ? `${wpm} wpm` : '';
  }
</script>

<div class="transcript-panel">
  {#if entries.length > 0}
    <PanelHeader title="Transcript" />
  {/if}
  <div class="entries" bind:this={container} onscroll={onScroll}>
    {#if entries.length === 0}
      <p class="empty">Transcript will appear here when audio is captured...</p>
    {:else}
      {#each entries as entry, i (entry.timestamp_ms)}
        {@const isYou = entry.speaker === 'You'}
        {@const nextEntry = entries[i + 1]}
        {@const durationMs = nextEntry ? nextEntry.timestamp_ms - entry.timestamp_ms : 0}
        {@const wpm = isYou ? wpmLabel(entry.text, durationMs) : ''}
        {@const fillers = isYou ? fillerCount(entry.text) : 0}
        {@const words = isYou ? wordCount(entry.text) : 0}
        <div class="entry" class:interviewer={entry.speaker === 'Interviewer'} class:you={isYou}>
          <div class="meta">
            <span class="speaker" class:interviewer={entry.speaker === 'Interviewer'}>{entry.speaker}</span>
            <span class="time">{formatTime(entry.timestamp_ms)}</span>
            {#if isYou && words > 0}
              <span class="entry-stat">{words}w</span>
            {/if}
            {#if wpm}
              <span class="entry-stat entry-wpm">{wpm}</span>
            {/if}
            {#if fillers > 0}
              <span class="entry-stat entry-filler" title="{fillers} filler word{fillers > 1 ? 's' : ''}">{fillers} filler{fillers > 1 ? 's' : ''}</span>
            {/if}
            {#if onFlipSpeaker}
              <button class="flip-btn" title="Flip speaker" onclick={() => onFlipSpeaker(i)}>⇄</button>
            {/if}
          </div>
          <span class="text">{@html highlightText(entry.text, isYou)}</span>
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

  .entry-stat {
    font-size: var(--fs-xs); color: #334155; font-variant-numeric: tabular-nums;
  }
  .entry-wpm { color: #1e3a5f; }
  .entry-filler { color: #78350f; }

  :global(.filler-hit) {
    background: transparent;
    color: #f59e0b;
    font-style: italic;
    text-decoration: underline dotted #b45309;
  }
  :global(.kw-hit) {
    background: transparent;
    color: #34d399;
    font-weight: 600;
  }
</style>
