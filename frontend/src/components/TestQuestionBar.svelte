<script lang="ts">
  import { authFetch } from '../lib/api';

  const { capturing = false } = $props<{ capturing?: boolean }>();

  let sending = $state(false);
  let presetsOpen = $state(false);

  $effect(() => {
    if (capturing) presetsOpen = false;
  });

  const PRESET_GROUPS: { label: string; tag: string; questions: string[] }[] = [
    { label: 'Intro', tag: 'intro', questions: [
      'Tell me about yourself.',
      'Walk me through your background.',
    ]},
    { label: 'Behavioral', tag: 'star', questions: [
      'Tell me about a time you faced a difficult challenge.',
      'Tell me about a time you worked with a difficult stakeholder.',
      'How do you prioritize when everything is urgent?',
    ]},
    { label: 'Strengths / Weakness', tag: 'strengths', questions: [
      'What are your strengths?',
      'What is your greatest weakness?',
    ]},
    { label: 'Motivation', tag: 'motivation', questions: [
      'Why are you interested in this role?',
      'Where do you see yourself in five years?',
      'How would you describe the culture you work best in?',
    ]},
    { label: 'Salary', tag: 'salary', questions: [
      'What are your salary expectations?',
    ]},
    { label: 'Situational', tag: 'situational', questions: [
      'How would you design a system to handle high traffic?',
    ]},
    { label: 'Closing', tag: 'closing', questions: [
      'Do you have any questions for us?',
    ]},
  ];

  const TAG_COLOR: Record<string, string> = {
    intro: '#60a5fa', star: '#a78bfa', strengths: '#34d399',
    motivation: '#fb923c', salary: '#f59e0b', situational: '#38bdf8', closing: '#94a3b8',
  };

  async function send(q: string) {
    if (sending) return;
    sending = true;
    try {
      await authFetch('/api/simulate-question', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ question: q }),
      });
    } finally {
      sending = false;
    }
  }
</script>

<div class="tqb">
  <div class="tqb-header">
    <button class="tqb-toggle" onclick={() => presetsOpen = !presetsOpen}>
      {presetsOpen ? '▾' : '▸'} Examples
      <span class="tqb-count">{PRESET_GROUPS.reduce((n, g) => n + g.questions.length, 0)}</span>
    </button>
  </div>
  {#if presetsOpen}
    <div class="tqb-groups">
      {#each PRESET_GROUPS as group}
        <div class="tqb-group">
          <span class="tqb-group-label" style="color: {TAG_COLOR[group.tag]}">{group.label}</span>
          <div class="tqb-presets">
            {#each group.questions as p}
              <button class="tqb-preset" onclick={() => send(p)} disabled={sending}>{p}</button>
            {/each}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .tqb {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    padding: 0.4rem 0.5rem;
    background: #07101e;
    border-radius: 0.5rem;
    border: 1px solid #1a2d4a;
    flex-shrink: 0;
  }

  .tqb-header {
    display: flex;
    align-items: center;
  }

  .tqb-toggle {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    background: none;
    border: none;
    color: #475569;
    font-size: var(--fs-xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    cursor: pointer;
    padding: 0;
    transition: color 0.12s;
  }
  .tqb-toggle:hover { color: #64748b; }

  .tqb-count {
    background: #0d1f35;
    border: 1px solid #1e3a5f;
    color: #475569;
    font-size: var(--fs-xs);
    font-weight: 700;
    border-radius: 0.9em;
    padding: 0 0.4em;
    line-height: 1.5;
  }

  .tqb-groups {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .tqb-group {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }

  .tqb-group-label {
    font-size: var(--fs-xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    opacity: 0.8;
  }

  .tqb-presets {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
  }

  .tqb-preset {
    background: #081428;
    border: 1px solid #1e3a5f;
    color: #7dd3fc;
    font-size: var(--fs-xs);
    padding: 0.15rem 0.45rem;
    border-radius: 0.25rem;
    cursor: pointer;
    white-space: normal;
    word-break: break-word;
    transition: all 0.12s;
    line-height: 1.4;
    text-align: left;
  }
  .tqb-preset:hover:not(:disabled) {
    border-color: #38bdf8;
    color: #e0f2fe;
    background: #0c2240;
  }
  .tqb-preset:disabled { opacity: 0.4; cursor: not-allowed; }

</style>
