<script lang="ts">
  import { loadStories, saveStory, deleteStory, updateStory, matchStories, type Story } from '../lib/storyBank';

  const { mode, matchQuestion = '', onClose } = $props<{
    mode: 'setup' | 'interview';
    matchQuestion?: string;
    onClose?: () => void;
  }>();

  let stories = $state<Story[]>(loadStories());
  let adding = $state(false);
  let editId = $state<string | null>(null);
  let form = $state({ title: '', situation: '', task: '', action: '', result: '', tags: '' });

  const matched = $derived(matchQuestion ? matchStories(matchQuestion, stories) : []);

  function resetForm() {
    form = { title: '', situation: '', task: '', action: '', result: '', tags: '' };
    adding = false;
    editId = null;
  }

  function startAdd() { resetForm(); adding = true; }

  function startEdit(s: Story) {
    form = { title: s.title, situation: s.situation, task: s.task, action: s.action, result: s.result, tags: s.tags.join(', ') };
    editId = s.id;
    adding = true;
  }

  function submit() {
    const tags = form.tags.split(',').map(t => t.trim()).filter(Boolean);
    const data = { title: form.title, situation: form.situation, task: form.task, action: form.action, result: form.result, tags };
    if (editId) {
      updateStory({ id: editId, ...data });
    } else {
      saveStory(data);
    }
    stories = loadStories();
    resetForm();
  }

  function remove(id: string) {
    deleteStory(id);
    stories = loadStories();
  }
</script>

<div class="story-panel">
  {#if mode === 'setup'}
    <div class="panel-header">
      <h3>Story Bank</h3>
      <p class="subtitle">Add 5–8 personal achievement stories. During interviews, the AI will suggest which story to tell for each behavioral question.</p>
    </div>
  {:else}
    <div class="interview-header">
      <span class="panel-title">Story Bank</span>
      {#if onClose}<button class="close-btn" onclick={onClose}>✕</button>{/if}
    </div>
    {#if matched.length > 0}
      <div class="matched-section">
        <div class="matched-label">Relevant stories for this question</div>
        {#each matched as s}
          <div class="matched-story">
            <div class="story-title">{s.title}</div>
            <div class="star-row"><span class="star-label">S</span><span class="star-text">{s.situation}</span></div>
            <div class="star-row"><span class="star-label">T</span><span class="star-text">{s.task}</span></div>
            <div class="star-row"><span class="star-label star-a">A</span><span class="star-text">{s.action}</span></div>
            <div class="star-row"><span class="star-label star-r">R</span><span class="star-text">{s.result}</span></div>
          </div>
        {/each}
      </div>
    {:else if matchQuestion}
      <p class="no-match">No stories matched. Browse below or add a new one.</p>
    {/if}
  {/if}

  {#if adding}
    <div class="form">
      <div class="form-field">
        <label>Story title <span class="hint">(short name you'll recognize)</span></label>
        <input bind:value={form.title} placeholder="e.g. Led rebrand under tight deadline" />
      </div>
      <div class="form-field">
        <label><span class="star-label">S</span> Situation</label>
        <textarea bind:value={form.situation} rows={2} placeholder="What was the context? What was at stake?"></textarea>
      </div>
      <div class="form-field">
        <label><span class="star-label">T</span> Task</label>
        <textarea bind:value={form.task} rows={2} placeholder="What were you specifically responsible for?"></textarea>
      </div>
      <div class="form-field">
        <label><span class="star-label star-a">A</span> Action</label>
        <textarea bind:value={form.action} rows={2} placeholder="What did you do? Be specific about your role."></textarea>
      </div>
      <div class="form-field">
        <label><span class="star-label star-r">R</span> Result</label>
        <textarea bind:value={form.result} rows={2} placeholder="What happened? Include numbers/metrics if possible."></textarea>
      </div>
      <div class="form-field">
        <label>Tags <span class="hint">(comma-separated keywords)</span></label>
        <input bind:value={form.tags} placeholder="e.g. leadership, conflict, data, cross-functional" />
      </div>
      <div class="form-actions">
        <button class="btn-save" onclick={submit} disabled={!form.title.trim()}>Save Story</button>
        <button class="btn-cancel" onclick={resetForm}>Cancel</button>
      </div>
    </div>
  {:else}
    <button class="btn-add-story" onclick={startAdd}>+ Add Story</button>
  {/if}

  {#if stories.length > 0}
    <div class="stories-list">
      {#each stories as s (s.id)}
        <div class="story-card">
          <div class="story-card-header">
            <span class="story-card-title">{s.title}</span>
            <div class="story-card-actions">
              <button class="btn-edit" onclick={() => startEdit(s)}>Edit</button>
              <button class="btn-delete" onclick={() => remove(s.id)}>✕</button>
            </div>
          </div>
          {#if s.tags.length > 0}
            <div class="tag-row">
              {#each s.tags as tag}
                <span class="tag">{tag}</span>
              {/each}
            </div>
          {/if}
          <div class="story-result">R: {s.result.slice(0, 80)}{s.result.length > 80 ? '…' : ''}</div>
        </div>
      {/each}
    </div>
  {:else if !adding}
    <p class="empty-stories">No stories yet. Add your first achievement story above.</p>
  {/if}
</div>

<style>
  .story-panel { display: flex; flex-direction: column; gap: 0.75rem; height: 100%; overflow-y: auto; }
  .panel-header { display: flex; flex-direction: column; gap: 0.25rem; }
  h3 { font-size: 1rem; font-weight: 700; color: #f1f5f9; margin: 0; }
  .subtitle { font-size: 0.8rem; color: #64748b; margin: 0; line-height: 1.4; }
  .interview-header { display: flex; align-items: center; justify-content: space-between; flex-shrink: 0; }
  .panel-title { font-size: 0.72rem; font-weight: 700; color: #334155; text-transform: uppercase; letter-spacing: 0.08em; }
  .close-btn { background: none; border: none; color: #334155; cursor: pointer; font-size: 0.9rem; padding: 0; }
  .close-btn:hover { color: #64748b; }

  .matched-section { display: flex; flex-direction: column; gap: 0.5rem; }
  .matched-label { font-size: 0.62rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.07em; color: #4ade80; }
  .matched-story {
    background: #071a0f; border: 1px solid #14532d; border-radius: 0.4rem;
    padding: 0.6rem 0.75rem; display: flex; flex-direction: column; gap: 0.3rem;
  }
  .story-title { font-size: 0.78rem; font-weight: 700; color: #e2e8f0; }
  .star-row { display: flex; gap: 0.4rem; align-items: flex-start; }
  .star-label {
    flex-shrink: 0; width: 1.2em; font-size: 0.6rem; font-weight: 800;
    text-transform: uppercase; color: #4ade80; background: #14532d;
    border-radius: 0.2rem; text-align: center; padding: 0.05rem 0.2rem; margin-top: 0.05rem;
  }
  .star-label.star-a { color: #fb923c; background: #431407; }
  .star-label.star-r { color: #60a5fa; background: #1a2d4a; }
  .star-text { font-size: 0.75rem; color: #94a3b8; line-height: 1.4; }
  .no-match { font-size: 0.75rem; color: #334155; font-style: italic; margin: 0; }

  .form { display: flex; flex-direction: column; gap: 0.6rem; background: #0f172a; border: 1px solid #1e293b; border-radius: 0.5rem; padding: 0.75rem; }
  .form-field { display: flex; flex-direction: column; gap: 0.25rem; }
  .form-field label { display: flex; align-items: center; gap: 0.4rem; font-size: 0.72rem; font-weight: 600; color: #94a3b8; }
  .hint { font-size: 0.65rem; font-weight: 400; color: #475569; }
  .form-field input, .form-field textarea {
    background: #1e293b; border: 1px solid #334155; border-radius: 0.375rem;
    color: #e2e8f0; font-size: 0.82rem; padding: 0.4rem 0.6rem; width: 100%;
    resize: vertical; font-family: inherit;
  }
  .form-field input:focus, .form-field textarea:focus { outline: none; border-color: #3b82f6; }
  .form-actions { display: flex; gap: 0.5rem; }
  .btn-save {
    padding: 0.35rem 1rem; background: #3b82f6; border: none; border-radius: 0.375rem;
    color: white; font-size: 0.8rem; font-weight: 600; cursor: pointer;
  }
  .btn-save:hover:not(:disabled) { background: #2563eb; }
  .btn-save:disabled { opacity: 0.4; cursor: default; }
  .btn-cancel { padding: 0.35rem 0.75rem; background: transparent; border: 1px solid #334155; border-radius: 0.375rem; color: #64748b; font-size: 0.8rem; cursor: pointer; }
  .btn-cancel:hover { border-color: #64748b; color: #94a3b8; }
  .btn-add-story { align-self: flex-start; padding: 0.3rem 0.75rem; background: transparent; border: 1px solid #3b82f6; border-radius: 0.375rem; color: #60a5fa; font-size: 0.8rem; cursor: pointer; }
  .btn-add-story:hover { background: #1e3a5f; }

  .stories-list { display: flex; flex-direction: column; gap: 0.4rem; }
  .story-card { background: #080d18; border: 1px solid #1e293b; border-radius: 0.4rem; padding: 0.5rem 0.6rem; display: flex; flex-direction: column; gap: 0.25rem; }
  .story-card-header { display: flex; align-items: center; justify-content: space-between; gap: 0.5rem; }
  .story-card-title { font-size: 0.78rem; font-weight: 600; color: #e2e8f0; flex: 1; }
  .story-card-actions { display: flex; gap: 0.25rem; flex-shrink: 0; }
  .btn-edit { padding: 0.1rem 0.45rem; background: transparent; border: 1px solid #1e293b; border-radius: 0.2rem; color: #475569; font-size: 0.62rem; cursor: pointer; }
  .btn-edit:hover { border-color: #60a5fa; color: #60a5fa; }
  .btn-delete { padding: 0.1rem 0.4rem; background: transparent; border: 1px solid #1e293b; border-radius: 0.2rem; color: #334155; font-size: 0.62rem; cursor: pointer; }
  .btn-delete:hover { border-color: #ef4444; color: #ef4444; }
  .tag-row { display: flex; flex-wrap: wrap; gap: 0.2rem; }
  .tag { font-size: 0.58rem; padding: 0.05rem 0.35rem; background: #0f172a; border: 1px solid #1e293b; border-radius: 9999px; color: #475569; }
  .story-result { font-size: 0.72rem; color: #475569; font-style: italic; }
  .empty-stories { font-size: 0.8rem; color: #334155; font-style: italic; text-align: center; padding: 1rem 0; margin: 0; }
</style>
