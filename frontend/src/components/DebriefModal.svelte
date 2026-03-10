<script lang="ts">
  import type { TranscriptEntry, SuggestionEntry } from '../lib/types';

  const { transcript, suggestions, onClose } = $props<{
    transcript: TranscriptEntry[];
    suggestions: SuggestionEntry[];
    onClose: () => void;
  }>();

  interface DebriefResult {
    summary: string;
    strong_points: string[];
    improvement_areas: string[];
    followup_email: string[];
  }

  let loading = $state(true);
  let result = $state<DebriefResult | null>(null);
  let error = $state('');

  async function fetchDebrief() {
    try {
      const resp = await fetch('/api/debrief', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          transcript: transcript.map(e => ({ speaker: e.speaker, text: e.text })),
          suggestions: suggestions.filter(s => s.suggestion).map(s => ({ question: s.question, suggestion: s.suggestion })),
        }),
      });
      if (!resp.ok) throw new Error(`Debrief failed: ${resp.status}`);
      result = await resp.json();
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  fetchDebrief();

  function copyEmail() {
    if (!result) return;
    const text = result.followup_email.map(p => `• ${p}`).join('\n');
    navigator.clipboard.writeText(text);
  }
</script>

<div class="modal-backdrop" onclick={onClose} role="none">
  <div class="modal" onclick={(e) => e.stopPropagation()} role="dialog">
    <div class="modal-header">
      <h2>Interview Debrief</h2>
      <button class="close-btn" onclick={onClose}>✕</button>
    </div>

    <div class="modal-body">
      {#if loading}
        <div class="loading">Analyzing your interview...</div>
      {:else if error}
        <div class="error">{error}</div>
      {:else if result}
        <section>
          <h3>Overall</h3>
          <p class="summary">{result.summary}</p>
        </section>

        <div class="two-col">
          <section>
            <h3 class="green">Strong Moments</h3>
            <ul>
              {#each result.strong_points as point}
                <li>{point}</li>
              {/each}
            </ul>
          </section>

          <section>
            <h3 class="yellow">Areas to Improve</h3>
            <ul>
              {#each result.improvement_areas as area}
                <li>{area}</li>
              {/each}
            </ul>
          </section>
        </div>

        <section>
          <div class="followup-header">
            <h3>Follow-up Email Points</h3>
            <button class="copy-btn" onclick={copyEmail}>Copy</button>
          </div>
          <ul>
            {#each result.followup_email as point}
              <li>{point}</li>
            {/each}
          </ul>
        </section>
      {/if}
    </div>
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed; inset: 0; background: rgba(0,0,0,0.7);
    display: flex; align-items: center; justify-content: center;
    z-index: 100;
  }
  .modal {
    background: #0f172a; border: 1px solid #1e293b; border-radius: 0.75rem;
    width: min(700px, 95vw); max-height: 85vh; display: flex; flex-direction: column;
  }
  .modal-header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 1rem 1.5rem; border-bottom: 1px solid #1e293b; flex-shrink: 0;
  }
  h2 { font-size: 1.2rem; font-weight: 700; color: #f1f5f9; margin: 0; }
  .close-btn {
    background: none; border: none; color: #64748b; font-size: 1.1rem;
    cursor: pointer; padding: 0.2rem 0.4rem;
  }
  .close-btn:hover { color: #e2e8f0; }
  .modal-body { overflow-y: auto; padding: 1.5rem; display: flex; flex-direction: column; gap: 1.5rem; }
  section { display: flex; flex-direction: column; gap: 0.5rem; }
  h3 { font-size: 0.8rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.07em; color: #94a3b8; margin: 0; }
  h3.green { color: #4ade80; }
  h3.yellow { color: #f59e0b; }
  .summary { color: #cbd5e1; line-height: 1.6; font-size: 0.9rem; margin: 0; }
  ul { margin: 0; padding-left: 1.25rem; display: flex; flex-direction: column; gap: 0.3rem; }
  li { color: #94a3b8; font-size: 0.875rem; line-height: 1.5; }
  .two-col { display: grid; grid-template-columns: 1fr 1fr; gap: 1.5rem; }
  .followup-header { display: flex; align-items: center; justify-content: space-between; }
  .copy-btn {
    padding: 0.15rem 0.6rem; background: transparent;
    border: 1px solid #334155; border-radius: 0.25rem;
    color: #64748b; font-size: 0.72rem; cursor: pointer;
  }
  .copy-btn:hover { border-color: #60a5fa; color: #60a5fa; }
  .loading { color: #60a5fa; font-style: italic; text-align: center; padding: 2rem; }
  .error { color: #fca5a5; padding: 1rem; background: #450a0a; border-radius: 0.5rem; }
</style>
