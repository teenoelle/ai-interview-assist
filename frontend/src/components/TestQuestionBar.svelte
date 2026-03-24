<script lang="ts">
  const { capturing = false } = $props<{ capturing?: boolean }>();

  let question = $state('');
  let sending = $state(false);
  let presetsOpen = $state(true);

  // Auto-collapse presets when capture starts; restore when it stops
  $effect(() => {
    if (capturing) presetsOpen = false;
    else presetsOpen = true;
  });

  const PRESETS = [
    'Tell me about yourself.',
    'Walk me through your background.',
    'Tell me about a time you faced a difficult challenge.',
    'Tell me about a time you worked with a difficult stakeholder.',
    'How do you prioritize when everything is urgent?',
    'What are your strengths?',
    'What is your greatest weakness?',
    'Why are you interested in this role?',
    'Where do you see yourself in five years?',
    'How would you describe the culture you work best in?',
    'What are your salary expectations?',
    'How would you design a system to handle high traffic?',
    'Do you have any questions for us?',
  ];

  async function submit(q: string) {
    const text = q.trim();
    if (!text || sending) return;
    sending = true;
    try {
      await fetch('/api/simulate-question', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ question: text }),
      });
      question = '';
    } finally {
      sending = false;
    }
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      submit(question);
    }
  }
</script>

<div class="tqb">
  <div class="tqb-input-row">
    <button class="tqb-toggle" onclick={() => presetsOpen = !presetsOpen} title={presetsOpen ? 'Hide presets' : 'Show presets'}>
      {presetsOpen ? '▾' : '▸'} Presets
    </button>
    <input
      class="tqb-input"
      type="text"
      placeholder="Type a test question…"
      bind:value={question}
      onkeydown={onKeydown}
      disabled={sending}
    />
    <button class="tqb-send" onclick={() => submit(question)} disabled={sending || !question.trim()}>
      {sending ? '…' : 'Send'}
    </button>
  </div>
  {#if presetsOpen}
    <div class="tqb-presets">
      {#each PRESETS as p}
        <button class="tqb-preset" onclick={() => submit(p)} disabled={sending}>{p}</button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .tqb {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    padding: 0.4rem 0.5rem;
    background: #07101e;
    border-radius: 0.5rem;
    border: 1px solid #1a2d4a;
    flex-shrink: 0;
  }

  .tqb-presets {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
  }

  .tqb-preset {
    background: #0d1f35;
    border: 1px solid #1e3a5f;
    color: #64748b;
    font-size: var(--fs-xs);
    padding: 0.15rem 0.45rem;
    border-radius: 0.25rem;
    cursor: pointer;
    white-space: nowrap;
    transition: all 0.12s;
    line-height: 1.4;
  }
  .tqb-preset:hover:not(:disabled) {
    border-color: #3b82f6;
    color: #93c5fd;
    background: #0f2847;
  }
  .tqb-preset:disabled { opacity: 0.4; cursor: not-allowed; }

  .tqb-input-row {
    display: flex;
    gap: 0.35rem;
    align-items: center;
  }

  .tqb-toggle {
    background: none;
    border: 1px solid #1e3a5f;
    color: #475569;
    font-size: var(--fs-xs);
    padding: 0.25rem 0.4rem;
    border-radius: 0.25rem;
    cursor: pointer;
    flex-shrink: 0;
    white-space: nowrap;
    transition: all 0.12s;
  }
  .tqb-toggle:hover { color: #64748b; border-color: #334155; }

  .tqb-input {
    flex: 1;
    background: #040d1a;
    border: 1px solid #1e3a5f;
    color: #e2e8f0;
    font-size: var(--fs-sm);
    padding: 0.3rem 0.5rem;
    border-radius: 0.3rem;
    outline: none;
    transition: border-color 0.12s;
  }
  .tqb-input:focus { border-color: #3b82f6; }
  .tqb-input::placeholder { color: #334155; }
  .tqb-input:disabled { opacity: 0.5; }

  .tqb-send {
    background: #1e3a5f;
    border: 1px solid #3b82f6;
    color: #93c5fd;
    font-size: var(--fs-sm);
    font-weight: 700;
    padding: 0.3rem 0.75rem;
    border-radius: 0.3rem;
    cursor: pointer;
    transition: all 0.12s;
    flex-shrink: 0;
  }
  .tqb-send:hover:not(:disabled) { background: #2d4f7c; }
  .tqb-send:disabled { opacity: 0.4; cursor: not-allowed; }
</style>
