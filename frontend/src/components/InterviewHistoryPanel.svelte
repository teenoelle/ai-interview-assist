<script lang="ts">
  import { loadHistory, deleteRecord, type InterviewRecord } from '../lib/interviewHistory';

  const { onClose, onRehearsal } = $props<{
    onClose: () => void;
    onRehearsal: (questions: string[]) => void;
  }>();

  let records = $state<InterviewRecord[]>(loadHistory());
  let expandedId = $state<string | null>(null);

  function remove(id: string) {
    deleteRecord(id);
    records = loadHistory();
  }

  function rehearse(r: InterviewRecord) {
    const questions = r.rehearsal_questions?.length
      ? r.rehearsal_questions
      : r.improvement_areas.map(a => `Practice: ${a}`);
    onRehearsal(questions);
    onClose();
  }
</script>

<div class="backdrop" onclick={onClose} role="none">
  <div class="panel" onclick={(e) => e.stopPropagation()} role="dialog">
    <div class="panel-header">
      <h2>Interview History</h2>
      <button class="close-btn" onclick={onClose}>✕</button>
    </div>

    <div class="body">
      {#if records.length === 0}
        <p class="empty">No interviews saved yet. Complete an interview and close the debrief to save.</p>
      {:else}
        {#each records as r (r.id)}
          <div class="record" class:expanded={expandedId === r.id}>
            <button class="record-header" onclick={() => expandedId = expandedId === r.id ? null : r.id}>
              <span class="record-date">{r.date}</span>
              <span class="record-summary">{r.summary.slice(0, 70)}{r.summary.length > 70 ? '…' : ''}</span>
              <span class="record-chevron">{expandedId === r.id ? '▴' : '▾'}</span>
            </button>
            {#if expandedId === r.id}
              <div class="record-body">
                <p class="record-full-summary">{r.summary}</p>
                {#if r.strong_points.length}
                  <div class="section green">
                    <div class="section-label">Strong Moments</div>
                    <ul>{#each r.strong_points as p}<li>{p}</li>{/each}</ul>
                  </div>
                {/if}
                {#if r.improvement_areas.length}
                  <div class="section yellow">
                    <div class="section-label">Improve</div>
                    <ul>{#each r.improvement_areas as p}<li>{p}</li>{/each}</ul>
                  </div>
                {/if}
                <div class="record-actions">
                  <button class="rehearse-btn" onclick={() => rehearse(r)}>Practice weak answers</button>
                  <button class="delete-btn" onclick={() => remove(r.id)}>Delete</button>
                </div>
              </div>
            {/if}
          </div>
        {/each}
      {/if}
    </div>
  </div>
</div>

<style>
  .backdrop { position: fixed; inset: 0; background: rgba(0,0,0,0.8); display: flex; align-items: center; justify-content: center; z-index: 150; }
  .panel { background: #0f172a; border: 1px solid #1e293b; border-radius: 0.75rem; width: min(600px, 95vw); max-height: 85vh; display: flex; flex-direction: column; }
  .panel-header { display: flex; align-items: center; justify-content: space-between; padding: 1rem 1.5rem; border-bottom: 1px solid #1e293b; flex-shrink: 0; }
  h2 { font-size: 1rem; font-weight: 700; color: #f1f5f9; margin: 0; }
  .close-btn { background: none; border: none; color: #64748b; font-size: 1rem; cursor: pointer; }
  .body { overflow-y: auto; padding: 0.75rem; display: flex; flex-direction: column; gap: 0.4rem; }
  .empty { color: #475569; font-style: italic; font-size: 0.85rem; text-align: center; padding: 2rem; }
  .record { background: #080d18; border: 1px solid #1e293b; border-radius: 0.4rem; overflow: hidden; }
  .record-header { width: 100%; display: flex; align-items: baseline; gap: 0.6rem; padding: 0.6rem 0.75rem; background: transparent; border: none; cursor: pointer; text-align: left; }
  .record-header:hover { background: #0d1525; }
  .record-date { font-size: 0.62rem; color: #334155; flex-shrink: 0; white-space: nowrap; }
  .record-summary { flex: 1; font-size: 0.75rem; color: #64748b; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .record-chevron { font-size: 0.6rem; color: #334155; flex-shrink: 0; }
  .record-body { padding: 0.75rem; border-top: 1px solid #1e293b; display: flex; flex-direction: column; gap: 0.6rem; }
  .record-full-summary { font-size: 0.8rem; color: #94a3b8; line-height: 1.5; margin: 0; }
  .section { display: flex; flex-direction: column; gap: 0.25rem; }
  .section-label { font-size: 0.6rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.07em; }
  .section.green .section-label { color: #4ade80; }
  .section.yellow .section-label { color: #f59e0b; }
  ul { margin: 0; padding-left: 1.1rem; display: flex; flex-direction: column; gap: 0.15rem; }
  li { font-size: 0.75rem; color: #64748b; line-height: 1.4; }
  .record-actions { display: flex; gap: 0.5rem; padding-top: 0.25rem; }
  .rehearse-btn { padding: 0.3rem 0.75rem; background: #14532d; border: none; border-radius: 0.3rem; color: #4ade80; font-size: 0.72rem; cursor: pointer; }
  .rehearse-btn:hover { background: #166534; }
  .delete-btn { padding: 0.3rem 0.6rem; background: transparent; border: 1px solid #1e293b; border-radius: 0.3rem; color: #334155; font-size: 0.72rem; cursor: pointer; margin-left: auto; }
  .delete-btn:hover { border-color: #7f1d1d; color: #fca5a5; }
</style>
