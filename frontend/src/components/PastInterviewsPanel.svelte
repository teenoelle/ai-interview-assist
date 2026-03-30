<script lang="ts">
  import { loadHistory, deleteRecord, type InterviewRecord } from '../lib/interviewHistory';
  import { authFetch } from '../lib/api';
  import type { ReviewReport, ReviewSummary } from './ReviewPanel.svelte';
  import SessionDetailModal from './SessionDetailModal.svelte';
  import SessionDetailView from './SessionDetailView.svelte';

  const { onClose, onReport, onRehearsal } = $props<{
    onClose: () => void;
    onReport: (report: ReviewReport) => void;
    onRehearsal: (questions: string[]) => void;
  }>();

  // ── Layout mode ────────────────────────────────────────────────────────────
  let viewMode = $state<'modal' | 'split'>(
    (localStorage.getItem('reports-view-mode') as 'modal' | 'split') ?? 'split'
  );
  $effect(() => { localStorage.setItem('reports-view-mode', viewMode); });

  // Tabs
  let tab = $state<'sessions' | 'recordings'>('sessions');

  // ── Sessions (localStorage) ────────────────────────────────────────────────
  let records = $state<InterviewRecord[]>(loadHistory());
  let selectedRecord = $state<InterviewRecord | null>(null);

  function removeRecord(id: string) {
    deleteRecord(id);
    records = loadHistory();
    if (selectedRecord?.id === id) selectedRecord = null;
  }

  // ── Recordings (server) ────────────────────────────────────────────────────
  let recordings = $state<ReviewSummary[]>([]);
  let recordingsLoaded = $state(false);
  let recordingsLoading = $state(false);
  let recordingSearch = $state('');

  const filteredRecordings = $derived(
    recordingSearch.trim()
      ? recordings.filter(r => (r.source_filename ?? '').toLowerCase().includes(recordingSearch.toLowerCase()))
      : recordings
  );

  async function loadRecordings() {
    recordingsLoading = true;
    try {
      const resp = await authFetch('/api/reviews');
      if (resp.ok) recordings = await resp.json();
    } catch { /* ignore */ }
    recordingsLoaded = true;
    recordingsLoading = false;
  }

  async function openRecording(id: string) {
    const resp = await authFetch(`/api/review/${id}`);
    if (resp.ok) {
      onReport(await resp.json());
      onClose();
    }
  }

  async function deleteRecording(id: string) {
    try {
      await authFetch(`/api/review/${id}`, { method: 'DELETE' });
      recordings = recordings.filter(r => r.id !== id);
    } catch { /* ignore */ }
  }

  function switchToRecordings() {
    tab = 'recordings';
    if (!recordingsLoaded) void loadRecordings();
  }

  // ── Upload flow ────────────────────────────────────────────────────────────
  let showUpload = $state(false);
  let uploadFile = $state<File | null>(null);
  let dragging = $state(false);
  let uploading = $state(false);
  let uploadPct = $state(0);
  let uploadStep = $state('');
  let uploadError = $state('');

  const ACCEPT = '.mp4,.webm,.mov,.mkv,.avi,.mp3,.m4a,.wav,.ogg,.flac';

  function onDrop(e: DragEvent) {
    e.preventDefault();
    dragging = false;
    const f = e.dataTransfer?.files[0];
    if (f) uploadFile = f;
  }

  function onFileChange(e: Event) {
    const f = (e.target as HTMLInputElement).files?.[0];
    if (f) uploadFile = f;
  }

  function cancelUpload() {
    showUpload = false;
    uploadFile = null;
    uploadError = '';
    uploading = false;
  }

  async function upload() {
    if (!uploadFile) return;
    uploading = true;
    uploadError = '';
    uploadPct = 0;
    uploadStep = 'Uploading…';

    const fd = new FormData();
    fd.append('file', uploadFile);

    let id: string;
    try {
      const resp = await fetch('/api/review/upload', { method: 'POST', body: fd });
      if (!resp.ok) throw new Error(await resp.text());
      ({ id } = await resp.json());
    } catch (e) {
      uploadError = String(e);
      uploading = false;
      return;
    }

    const es = new EventSource(`/api/review/${id}/events`);
    es.onmessage = async (ev) => {
      const p: { pct: number; step: string; done: boolean; error?: string } = JSON.parse(ev.data);
      uploadPct = p.pct;
      uploadStep = p.step;
      if (p.done) {
        es.close();
        if (p.error) {
          uploadError = p.error;
          uploading = false;
        } else {
          const r = await fetch(`/api/review/${id}`);
          if (r.ok) {
            onReport(await r.json());
            onClose();
          } else {
            uploadError = 'Could not load report.';
            uploading = false;
          }
        }
      }
    };
    es.onerror = () => {
      es.close();
      uploadError = 'Connection lost during processing.';
      uploading = false;
    };
  }

  function fmtSize(bytes: number) {
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
    return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  }

  function fmtDate(ts: number) {
    return new Date(ts).toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' });
  }

  function fmtDuration(secs: number) {
    const m = Math.floor(secs / 60);
    const s = Math.round(secs % 60);
    return m > 0 ? `${m}m ${s}s` : `${s}s`;
  }
</script>

<div class="backdrop" onclick={onClose} role="none">
  <div class="panel" class:split={viewMode === 'split'} onclick={(e) => e.stopPropagation()} role="dialog">

    <div class="panel-header">
      <h2>Reports</h2>
      <div class="header-right">
        <div class="view-toggle">
          <button
            class="toggle-btn"
            class:active={viewMode === 'modal'}
            onclick={() => { viewMode = 'modal'; selectedRecord = null; }}
          >List</button>
          <button
            class="toggle-btn"
            class:active={viewMode === 'split'}
            onclick={() => viewMode = 'split'}
          >Split</button>
        </div>
        <button class="close-btn" onclick={onClose}>✕</button>
      </div>
    </div>

    <div class="tab-bar">
      <button class="tab" class:active={tab === 'sessions'} onclick={() => tab = 'sessions'}>Sessions</button>
      <button class="tab" class:active={tab === 'recordings'} onclick={switchToRecordings}>Recordings</button>
    </div>

    {#if tab === 'sessions' && viewMode === 'split'}

      <!-- ── Split layout ─────────────────────────────────────────────────── -->
      <div class="split-layout">
        <div class="split-left">
          {#if records.length === 0}
            <p class="empty">No sessions saved yet.</p>
          {:else}
            {#each records as r (r.id)}
              <button
                class="record"
                class:selected={selectedRecord?.id === r.id}
                onclick={() => selectedRecord = r}
              >
                <span class="record-label">
                  {#if r.company || r.role}
                    {[r.company, r.role].filter(Boolean).join(' · ')}
                  {:else}
                    {r.summary}
                  {/if}
                </span>
                <span class="record-date">{r.date}</span>
              </button>
            {/each}
          {/if}
        </div>

        <div class="split-right">
          {#if selectedRecord}
            <div class="split-record-header">
              {#if selectedRecord.company || selectedRecord.role}
                <span class="split-record-title">{[selectedRecord.company, selectedRecord.role].filter(Boolean).join(' · ')}</span>
              {/if}
              <span class="split-record-date">{selectedRecord.date}</span>
            </div>
            <SessionDetailView
              record={selectedRecord}
              onDelete={(id) => removeRecord(id)}
              onRehearsal={(questions) => { onRehearsal(questions); onClose(); }}
            />
          {:else}
            <div class="split-placeholder">
              <span>Select a session to view its report</span>
            </div>
          {/if}
        </div>
      </div>

    {:else}

      <!-- ── Standard body (modal mode or recordings tab) ───────────────────── -->
      <div class="body">

        {#if tab === 'sessions'}
          {#if records.length === 0}
            <p class="empty">No sessions saved yet. Complete an interview and close the debrief to save.</p>
          {:else}
            {#if weakSpots.length > 0}
              <div class="weak-section">
                <div class="weak-title">Recurring Weak Spots</div>
                {#each weakSpots as ws}
                  <div class="weak-item">
                    <span class="weak-text">{ws.key}</span>
                    <span class="weak-count">×{ws.count}</span>
                  </div>
                {/each}
              </div>
            {/if}

            {#each records as r (r.id)}
              <button class="record" onclick={() => selectedRecord = r}>
                <span class="record-label">
                  {#if r.company || r.role}
                    {[r.company, r.role].filter(Boolean).join(' · ')}
                  {:else}
                    {r.summary}
                  {/if}
                </span>
                <span class="record-date">{r.date}</span>
                <span class="record-chevron">▸</span>
              </button>
            {/each}
          {/if}

        {:else}

          {#if showUpload}
            <div class="upload-section">
              {#if !uploading}
                <div
                  class="dropzone"
                  class:dragging
                  ondragover={(e) => { e.preventDefault(); dragging = true; }}
                  ondragleave={() => dragging = false}
                  ondrop={onDrop}
                  role="none"
                >
                  {#if uploadFile}
                    <div class="file-info">
                      <span class="file-name">{uploadFile.name}</span>
                      <span class="file-size">{fmtSize(uploadFile.size)}</span>
                    </div>
                    <button class="change-btn" onclick={() => uploadFile = null}>Change file</button>
                  {:else}
                    <span class="dz-icon">⬆</span>
                    <span class="dz-label">Drop a video or audio file here</span>
                    <label class="browse-btn">
                      Browse files
                      <input type="file" accept={ACCEPT} onchange={onFileChange} hidden />
                    </label>
                    <span class="dz-formats">MP4 · WebM · MOV · MP3 · M4A · WAV</span>
                  {/if}
                </div>
                {#if uploadError}
                  <div class="upload-error">{uploadError}</div>
                {/if}
                <div class="upload-actions">
                  <button class="cancel-btn" onclick={cancelUpload}>Cancel</button>
                  <button class="upload-btn" disabled={!uploadFile} onclick={upload}>Analyze Recording</button>
                </div>
              {:else}
                <div class="progress-wrap">
                  <div class="progress-step">{uploadStep || 'Processing…'}</div>
                  <div class="progress-bar"><div class="progress-fill" style="width:{uploadPct}%"></div></div>
                  <div class="progress-pct">{uploadPct}%</div>
                </div>
                {#if uploadError}
                  <div class="upload-error">{uploadError}</div>
                  <button class="cancel-btn" onclick={cancelUpload}>Close</button>
                {/if}
              {/if}
            </div>
          {:else}
            <div class="recordings-toolbar">
              <input class="search-input" type="text" placeholder="Search recordings…" bind:value={recordingSearch} />
              <button class="upload-open-btn" onclick={() => { showUpload = true; uploadFile = null; uploadError = ''; }}>
                ⬆ Upload Recording
              </button>
            </div>

            {#if recordingsLoading}
              <p class="empty">Loading…</p>
            {:else if filteredRecordings.length === 0}
              <p class="empty">{recordingSearch ? 'No matching recordings.' : 'No recordings yet. Upload a recording to get started.'}</p>
            {:else}
              <div class="recordings-list">
                {#each filteredRecordings as r}
                  <div class="recording-item">
                    <div class="recording-meta">
                      <span class="recording-name">{r.source_filename ?? 'Untitled'}</span>
                      <span class="recording-date">{r.created_at ? fmtDate(r.created_at) : ''}</span>
                    </div>
                    <div class="recording-stats">
                      {#if r.duration_secs}<span>{fmtDuration(r.duration_secs)}</span>{/if}
                      <span>{r.qa_count} Q&A</span>
                      <span>{r.avg_wpm} wpm</span>
                      <span>{Math.round(r.you_pct)}% you</span>
                    </div>
                    <div class="recording-actions">
                      <button class="open-btn" onclick={() => openRecording(r.id)}>Open Report</button>
                      <button class="delete-btn" onclick={() => deleteRecording(r.id)}>Delete</button>
                    </div>
                  </div>
                {/each}
              </div>
            {/if}
          {/if}

        {/if}
      </div>

    {/if}
  </div>
</div>

{#if selectedRecord && viewMode === 'modal'}
  <SessionDetailModal
    record={selectedRecord}
    onClose={() => selectedRecord = null}
    onDelete={(id) => { removeRecord(id); selectedRecord = null; }}
    onRehearsal={(questions) => { onRehearsal(questions); onClose(); }}
  />
{/if}

<style>
  .backdrop { position: fixed; inset: 0; background: rgba(0,0,0,0.8); display: flex; align-items: center; justify-content: center; z-index: 150; }
  .panel { background: #0f172a; border: 1px solid #1e293b; border-radius: 0.75rem; width: min(620px, 95vw); max-height: 85vh; display: flex; flex-direction: column; transition: width 0.2s; }
  .panel.split { width: min(1100px, 95vw); max-height: 88vh; }

  .panel-header { display: flex; align-items: center; justify-content: space-between; padding: 0.9rem 1.25rem 0.6rem; flex-shrink: 0; }
  h2 { font-size: 1rem; font-weight: 700; color: #f1f5f9; margin: 0; }
  .header-right { display: flex; align-items: center; gap: 0.5rem; }

  .view-toggle { display: flex; gap: 1px; background: #1e293b; border-radius: 0.3rem; padding: 2px; }
  .toggle-btn { background: none; border: none; color: #475569; font-size: 0.85rem; cursor: pointer; padding: 0.15rem 0.4rem; border-radius: 0.2rem; line-height: 1; transition: color 0.12s, background 0.12s; }
  .toggle-btn:hover { color: #94a3b8; }
  .toggle-btn.active { background: #334155; color: #e2e8f0; }

  .close-btn { background: none; border: none; color: #64748b; font-size: 1rem; cursor: pointer; padding: 0.2rem 0.4rem; }
  .close-btn:hover { color: #e2e8f0; }

  .tab-bar { display: flex; border-bottom: 1px solid #1e293b; padding: 0 1.25rem; flex-shrink: 0; }
  .tab { background: none; border: none; border-bottom: 2px solid transparent; color: #475569; font-size: var(--fs-sm); font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; padding: 0.45rem 0.75rem; cursor: pointer; transition: color 0.12s, border-color 0.12s; margin-bottom: -1px; }
  .tab:hover { color: #94a3b8; }
  .tab.active { color: #60a5fa; border-bottom-color: #60a5fa; }

  /* Standard (modal mode) body */
  .body { overflow-y: auto; padding: 0.75rem 1rem; display: flex; flex-direction: column; gap: 0.4rem; flex: 1; }
  .empty { color: #475569; font-style: italic; font-size: var(--fs-base); text-align: center; padding: 2rem; margin: 0; }

  /* Split layout */
  .split-layout { display: grid; grid-template-columns: 260px 1fr; flex: 1; min-height: 0; overflow: hidden; }
  .split-left { border-right: 1px solid #1e293b; overflow-y: auto; padding: 0.6rem 0.75rem; display: flex; flex-direction: column; gap: 0.35rem; }
  .split-right { display: flex; flex-direction: column; min-height: 0; overflow: hidden; }
  .split-record-header { padding: 0.6rem 1.25rem 0; flex-shrink: 0; display: flex; flex-direction: column; gap: 0.1rem; border-bottom: 1px solid #1e293b; padding-bottom: 0.5rem; }
  .split-record-title { font-size: var(--fs-sm); font-weight: 700; color: #e2e8f0; }
  .split-record-date { font-size: var(--fs-xs); color: #475569; }
  .split-placeholder { flex: 1; display: flex; align-items: center; justify-content: center; color: #334155; font-style: italic; font-size: var(--fs-sm); }

  /* Sessions */

  .record { width: 100%; display: flex; flex-direction: column; gap: 0.15rem; padding: 0.55rem 0.75rem; background: #080d18; border: 1px solid #1e293b; border-radius: 0.4rem; cursor: pointer; text-align: left; transition: background 0.12s; position: relative; }
  .record:hover { background: #0d1525; border-color: #334155; }
  .record.selected { background: #0d1a30; border-color: #2563eb; }
  .record-label { font-size: var(--fs-sm); color: #94a3b8; font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; padding-right: 1rem; }
  .record-date { font-size: var(--fs-xs); color: #475569; }
  .record-chevron { position: absolute; right: 0.75rem; top: 50%; transform: translateY(-50%); font-size: var(--fs-xs); color: #334155; }

  /* Recordings */
  .recordings-toolbar { display: flex; gap: 0.5rem; align-items: center; margin-bottom: 0.25rem; flex-shrink: 0; }
  .search-input { flex: 1; background: #060e1a; border: 1px solid #1e293b; border-radius: 0.3rem; color: #94a3b8; font-size: var(--fs-sm); padding: 0.3rem 0.6rem; }
  .search-input:focus { outline: none; border-color: #334155; }
  .upload-open-btn { background: #081428; border: 1px solid #1e3a5f; color: #7dd3fc; font-size: var(--fs-xs); padding: 0.3rem 0.65rem; border-radius: 0.3rem; cursor: pointer; white-space: nowrap; transition: all 0.12s; }
  .upload-open-btn:hover { border-color: #38bdf8; color: #e0f2fe; background: #0c2240; }

  .recordings-list { display: flex; flex-direction: column; gap: 0.4rem; }
  .recording-item { background: #080d18; border: 1px solid #1e293b; border-radius: 0.4rem; padding: 0.6rem 0.75rem; display: flex; flex-direction: column; gap: 0.3rem; }
  .recording-meta { display: flex; align-items: baseline; gap: 0.6rem; min-width: 0; }
  .recording-name { font-size: var(--fs-sm); color: #94a3b8; font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; flex: 1; }
  .recording-date { font-size: var(--fs-xs); color: #334155; flex-shrink: 0; white-space: nowrap; }
  .recording-stats { display: flex; gap: 0.6rem; flex-wrap: wrap; }
  .recording-stats span { font-size: var(--fs-xs); color: #475569; }
  .recording-actions { display: flex; gap: 0.5rem; }
  .open-btn { padding: 0.25rem 0.65rem; background: #0c1f38; border: 1px solid #1e3a5f; border-radius: 0.3rem; color: #60a5fa; font-size: var(--fs-xs); cursor: pointer; transition: all 0.12s; }
  .open-btn:hover { border-color: #3b82f6; color: #93c5fd; background: #0f2a50; }

  .delete-btn { padding: 0.25rem 0.6rem; background: transparent; border: 1px solid #1e293b; border-radius: 0.3rem; color: #334155; font-size: var(--fs-sm); cursor: pointer; }
  .delete-btn:hover { border-color: #7f1d1d; color: #fca5a5; }

  /* Upload flow */
  .upload-section { display: flex; flex-direction: column; gap: 0.75rem; }
  .dropzone { border: 2px dashed #1e293b; border-radius: 0.6rem; padding: 2rem 1.5rem; text-align: center; display: flex; flex-direction: column; align-items: center; gap: 0.5rem; transition: border-color 0.15s, background 0.15s; cursor: default; }
  .dropzone.dragging { border-color: #3b82f6; background: rgba(59,130,246,0.06); }
  .dz-icon { font-size: 1.5rem; color: #334155; }
  .dz-label { color: #94a3b8; font-size: var(--fs-base); }
  .dz-formats { color: #334155; font-size: var(--fs-xs); letter-spacing: 0.04em; margin-top: 0.15rem; }
  .browse-btn { padding: 0.35rem 0.9rem; background: #1e293b; border: 1px solid #334155; border-radius: 0.375rem; color: #94a3b8; font-size: var(--fs-sm); cursor: pointer; transition: all 0.15s; }
  .browse-btn:hover { border-color: #60a5fa; color: #60a5fa; }
  .file-info { display: flex; flex-direction: column; gap: 0.2rem; align-items: center; }
  .file-name { color: #e2e8f0; font-size: var(--fs-base); font-weight: 600; word-break: break-all; }
  .file-size { color: #64748b; font-size: var(--fs-sm); }
  .change-btn { background: none; border: 1px solid #334155; color: #64748b; border-radius: 0.3rem; padding: 0.2rem 0.5rem; font-size: var(--fs-sm); cursor: pointer; }
  .change-btn:hover { color: #94a3b8; border-color: #475569; }
  .upload-actions { display: flex; justify-content: flex-end; gap: 0.6rem; }
  .cancel-btn { padding: 0.35rem 0.9rem; background: none; border: 1px solid #334155; border-radius: 0.4rem; color: #64748b; font-size: var(--fs-sm); cursor: pointer; }
  .cancel-btn:hover { border-color: #475569; color: #94a3b8; }
  .upload-btn { padding: 0.35rem 1rem; background: #2563eb; border: none; border-radius: 0.4rem; color: white; font-size: var(--fs-sm); font-weight: 600; cursor: pointer; transition: background 0.15s; }
  .upload-btn:hover:not(:disabled) { background: #3b82f6; }
  .upload-btn:disabled { opacity: 0.4; cursor: default; }
  .upload-error { color: #fca5a5; background: #450a0a; border-radius: 0.4rem; padding: 0.5rem 0.75rem; font-size: var(--fs-sm); }
  .progress-wrap { display: flex; flex-direction: column; gap: 0.5rem; padding: 0.75rem 0; }
  .progress-step { color: #94a3b8; font-size: var(--fs-base); text-align: center; }
  .progress-bar { height: 5px; background: #1e293b; border-radius: 3px; overflow: hidden; }
  .progress-fill { height: 100%; background: #3b82f6; border-radius: 3px; transition: width 0.4s; }
  .progress-pct { color: #60a5fa; font-size: var(--fs-sm); text-align: center; font-weight: 600; }
</style>
