<script lang="ts">
  import { authFetch } from '../lib/api';

  const { capturing = false, onSimulate } = $props<{ capturing?: boolean; onSimulate?: (q: string) => void }>();

  let sending = $state(false);
  let presetsOpen = $state(false);

  $effect(() => {
    if (capturing) presetsOpen = false;
  });

  const PRESET_GROUPS: { label: string; tag: string; questions: string[] }[] = [
    { label: 'Small Talk', tag: 'smalltalk', questions: [
      'How are you doing today?',
      'How is your day going?',
      'Nice to meet you!',
      'How was your morning?',
      'Ready to get started?',
    ]},
    { label: 'Intro', tag: 'personal', questions: [
      'Tell me about yourself.',
      'Walk me through your background.',
    ]},
    { label: 'Motivation', tag: 'motivation', questions: [
      'Why are you interested in this role?',
      'What draws you to this company?',
      'Why are you applying for a junior position?',
      'You seem overqualified — why this role?',
      'Why are you looking to step back from your current level?',
      'Your background is strong — why a more entry-level position?',
    ]},
    { label: 'Future', tag: 'future', questions: [
      'Where do you see yourself in five years?',
      'What are your long-term career goals?',
      'How does this role fit into your career path?',
    ]},
    { label: 'Strengths', tag: 'strengths', questions: [
      'What are your strengths?',
      'What would your colleagues say you do best?',
    ]},
    { label: 'Character', tag: 'character', questions: [
      'What would your friends say about you?',
      'How would people who know you personally describe you?',
      'How would you describe yourself as a person?',
    ]},
    { label: 'Weakness', tag: 'weaknesses', questions: [
      'What is your greatest weakness?',
      'Tell me about an area you are working to improve.',
    ]},
    { label: 'Behavioral', tag: 'behavioral', questions: [
      'Tell me about a time you faced a difficult challenge.',
      'Tell me about a time you worked with a difficult stakeholder.',
      'How do you prioritize when everything is urgent?',
    ]},
    { label: 'Situational', tag: 'situational', questions: [
      'How would you handle a project with an unclear scope?',
      'What would you do if you disagreed with your manager?',
    ]},
    { label: 'Technical', tag: 'technical', questions: [
      'Walk me through how you would design a scalable system.',
      'How do you approach debugging a production issue?',
      'How do you stay current with new technologies?',
    ]},
    { label: 'Culture', tag: 'culture', questions: [
      'How do you collaborate with cross-functional teams?',
      'How do you handle disagreements within a team?',
      'How do you typically approach working across departments?',
    ]},
    { label: 'Values', tag: 'values', questions: [
      'What do you look for in a company?',
      'What do you look for in a manager?',
      'What do you look for in a team?',
      'What kind of environment do you thrive in?',
      'What does your ideal manager look like?',
      'What are you looking for in your next role?',
      'What kind of work environment do you do your best in?',
    ]},
    { label: 'Salary', tag: 'salary', questions: [
      'What are your salary expectations?',
    ]},
    { label: 'Closing', tag: 'closing', questions: [
      'Do you have any questions for us?',
    ]},
  ];

  const TAG_COLOR: Record<string, string> = {
    smalltalk: '#67e8f9',
    personal: '#f472b6', motivation: '#fb923c', future: '#38bdf8',
    strengths: '#4ade80', character: '#e879f9', weaknesses: '#f87171', behavioral: '#a78bfa',
    situational: '#a3e635', technical: '#60a5fa', culture: '#34d399',
    values: '#f0abfc', salary: '#fbbf24', closing: '#94a3b8',
  };

  async function send(q: string) {
    if (sending) return;
    onSimulate?.(q);
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
      {presetsOpen ? '▾' : '▸'} Example Questions
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
    flex-direction: column;
    gap: 0;
  }

  .tqb-preset {
    background: none;
    border: none;
    border-left: 2px solid #1a2d4a;
    color: #94a3b8;
    font-size: var(--fs-sm);
    padding: 0.15rem 0.5rem;
    border-radius: 0;
    cursor: pointer;
    white-space: normal;
    word-break: break-word;
    transition: all 0.12s;
    line-height: 1.2;
    text-align: left;
  }
  .tqb-preset:hover:not(:disabled) {
    border-left-color: #3b82f6;
    color: #e2e8f0;
    background: rgba(59,130,246,0.05);
  }
  .tqb-preset:disabled { opacity: 0.4; cursor: not-allowed; }

</style>
