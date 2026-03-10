<script lang="ts">
  const { questions, systemPrompt, onStartInterview } = $props<{
    questions: string[];
    systemPrompt: string;
    onStartInterview: () => void;
  }>();

  let currentIdx = $state(0);
  let hints = $state<Record<number, string>>({});
  let loadingHint = $state(false);

  const currentQuestion = $derived(questions[currentIdx] ?? '');

  async function getHint() {
    if (hints[currentIdx] || loadingHint) return;
    loadingHint = true;
    try {
      const resp = await fetch('/api/practice-question', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ question: currentQuestion }),
      });
      if (resp.ok) {
        const data = await resp.json();
        hints = { ...hints, [currentIdx]: data.suggestion };
      }
    } catch { /* ignore */ }
    loadingHint = false;
  }
</script>

<div class="practice">
  <div class="practice-header">
    <div>
      <h2>Practice Mode</h2>
      <p class="subtitle">Review predicted questions and prepare your answers before the interview</p>
    </div>
    <button class="start-btn" onclick={onStartInterview}>I'm Ready — Start Interview →</button>
  </div>

  {#if questions.length === 0}
    <div class="empty">No predicted questions available. Check your job description and try again.</div>
  {:else}
    <div class="progress">Question {currentIdx + 1} of {questions.length}</div>

    <div class="question-card">
      <p class="question-text">{currentQuestion}</p>
      <button class="hint-btn" onclick={getHint} disabled={loadingHint || !!hints[currentIdx]}>
        {loadingHint ? 'Loading...' : hints[currentIdx] ? 'Hints loaded' : '💡 Get AI talking points'}
      </button>
      {#if hints[currentIdx]}
        <div class="hints">
          {@html hints[currentIdx].replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')}
        </div>
      {/if}
    </div>

    <div class="nav">
      <button
        class="nav-btn"
        disabled={currentIdx === 0}
        onclick={() => currentIdx--}
      >← Previous</button>
      <div class="dots">
        {#each questions as _, i}
          <button
            class="dot"
            class:active={i === currentIdx}
            class:hinted={!!hints[i]}
            onclick={() => currentIdx = i}
          ></button>
        {/each}
      </div>
      <button
        class="nav-btn"
        disabled={currentIdx === questions.length - 1}
        onclick={() => currentIdx++}
      >Next →</button>
    </div>
  {/if}
</div>

<style>
  .practice {
    max-width: 720px; margin: 0 auto; padding: 2rem;
    display: flex; flex-direction: column; gap: 1.5rem;
  }
  .practice-header {
    display: flex; align-items: flex-start; justify-content: space-between; gap: 1rem; flex-wrap: wrap;
  }
  h2 { font-size: 1.5rem; font-weight: 800; color: #f1f5f9; margin: 0 0 0.25rem; }
  .subtitle { color: #64748b; font-size: 0.875rem; margin: 0; }
  .start-btn {
    padding: 0.6rem 1.5rem; background: #3b82f6; color: white;
    border: none; border-radius: 0.5rem; font-size: 0.9rem; font-weight: 600;
    cursor: pointer; white-space: nowrap; transition: background 0.2s;
  }
  .start-btn:hover { background: #2563eb; }
  .progress { font-size: 0.75rem; color: #475569; text-align: center; }
  .question-card {
    background: #0f172a; border: 1px solid #1e293b; border-radius: 0.75rem;
    padding: 1.5rem; display: flex; flex-direction: column; gap: 1rem;
  }
  .question-text { font-size: 1.15rem; color: #e2e8f0; line-height: 1.6; margin: 0; font-style: italic; }
  .hint-btn {
    align-self: flex-start; padding: 0.4rem 1rem;
    background: transparent; border: 1px solid #3b82f6; border-radius: 0.375rem;
    color: #60a5fa; font-size: 0.8rem; cursor: pointer; transition: all 0.15s;
  }
  .hint-btn:hover:not(:disabled) { background: #1e3a5f; }
  .hint-btn:disabled { opacity: 0.5; cursor: default; }
  .hints {
    background: #0a1020; border-left: 3px solid #3b82f6; border-radius: 0.5rem;
    padding: 1rem; color: #cbd5e1; font-size: 0.875rem; line-height: 1.8;
    white-space: pre-wrap;
  }
  :global(.hints strong) { color: #f1f5f9; font-size: 0.95rem; }
  .nav { display: flex; align-items: center; justify-content: space-between; }
  .nav-btn {
    padding: 0.4rem 1rem; background: transparent;
    border: 1px solid #334155; border-radius: 0.375rem;
    color: #94a3b8; font-size: 0.85rem; cursor: pointer; transition: all 0.15s;
  }
  .nav-btn:hover:not(:disabled) { border-color: #60a5fa; color: #60a5fa; }
  .nav-btn:disabled { opacity: 0.3; cursor: default; }
  .dots { display: flex; gap: 0.4rem; flex-wrap: wrap; justify-content: center; }
  .dot {
    width: 8px; height: 8px; border-radius: 50%;
    background: #1e293b; border: none; cursor: pointer; transition: all 0.15s;
  }
  .dot.active { background: #3b82f6; transform: scale(1.3); }
  .dot.hinted { background: #22c55e; }
  .empty { color: #475569; font-style: italic; text-align: center; padding: 3rem; }
</style>
