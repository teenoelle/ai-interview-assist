<script lang="ts">
  const { onReport, onCancel } = $props<{
    onReport: (report: ReviewReport) => void;
    onCancel: () => void;
  }>();

  interface ReviewReport {
    id: string;
    created_at: number;
    duration_secs: number;
    source_filename: string;
    source_type: string;
    transcript: unknown[];
    qa_pairs: unknown[];
    vocal_summary: { avg_wpm: number; total_answers: number };
    speaker_summary: { you_pct: number; them_pct: number; you_word_count: number; them_word_count: number; turn_count: number };
    keywords_mentioned: string[];
  }

  let file = $state<File | null>(null);
  let dragging = $state(false);
  let uploading = $state(false);
  let pct = $state(0);
  let step = $state('');
  let errorMsg = $state('');

  const ACCEPT = '.mp4,.webm,.mov,.mkv,.avi,.mp3,.m4a,.wav,.ogg,.flac';

  function onDrop(e: DragEvent) {
    e.preventDefault();
    dragging = false;
    const f = e.dataTransfer?.files[0];
    if (f) file = f;
  }

  function onFileChange(e: Event) {
    const f = (e.target as HTMLInputElement).files?.[0];
    if (f) file = f;
  }

  async function upload() {
    if (!file) return;
    uploading = true;
    errorMsg = '';
    pct = 0;
    step = 'Uploading…';

    const fd = new FormData();
    fd.append('file', file);

    let id: string;
    try {
      const resp = await fetch('/api/review/upload', { method: 'POST', body: fd });
      if (!resp.ok) throw new Error(await resp.text());
      ({ id } = await resp.json());
    } catch (e) {
      errorMsg = String(e);
      uploading = false;
      return;
    }

    // SSE progress
    const es = new EventSource(`/api/review/${id}/events`);
    es.onmessage = async (ev) => {
      const p: { pct: number; step: string; done: boolean; error?: string } = JSON.parse(ev.data);
      pct = p.pct;
      step = p.step;
      if (p.done) {
        es.close();
        if (p.error) {
          errorMsg = p.error;
          uploading = false;
        } else {
          const r = await fetch(`/api/review/${id}`);
          if (r.ok) onReport(await r.json());
          else { errorMsg = 'Could not load report.'; uploading = false; }
        }
      }
    };
    es.onerror = () => {
      es.close();
      errorMsg = 'Connection lost during processing.';
      uploading = false;
    };
  }

  function fmtSize(bytes: number) {
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
    return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  }
</script>

<div class="modal-backdrop" onclick={onCancel} role="none">
  <div class="modal" onclick={(e) => e.stopPropagation()} role="dialog">
    <div class="modal-header">
      <h2>Review a Recording</h2>
      <button class="close-btn" onclick={onCancel}>✕</button>
    </div>

    <div class="modal-body">
      {#if !uploading}
        <!-- Drop zone -->
        <div
          class="dropzone"
          class:dragging
          ondragover={(e) => { e.preventDefault(); dragging = true; }}
          ondragleave={() => { dragging = false; }}
          ondrop={onDrop}
          role="none"
        >
          {#if file}
            <div class="file-info">
              <span class="file-name">{file.name}</span>
              <span class="file-size">{fmtSize(file.size)}</span>
            </div>
            <button class="change-btn" onclick={() => file = null}>Change file</button>
          {:else}
            <span class="dz-icon">⬆</span>
            <span class="dz-label">Drop a video or audio file here</span>
            <span class="dz-sub">or</span>
            <label class="browse-btn">
              Browse files
              <input type="file" accept={ACCEPT} onchange={onFileChange} hidden />
            </label>
            <span class="dz-formats">MP4 · WebM · MOV · MP3 · M4A · WAV · OGG</span>
          {/if}
        </div>

        {#if errorMsg}
          <div class="error">{errorMsg}</div>
        {/if}

        <div class="actions">
          <button class="cancel-btn" onclick={onCancel}>Cancel</button>
          <button class="upload-btn" disabled={!file} onclick={upload}>
            Analyze Recording
          </button>
        </div>
      {:else}
        <!-- Progress -->
        <div class="progress-wrap">
          <div class="progress-step">{step || 'Processing…'}</div>
          <div class="progress-bar">
            <div class="progress-fill" style="width: {pct}%"></div>
          </div>
          <div class="progress-pct">{pct}%</div>
        </div>
        {#if errorMsg}
          <div class="error">{errorMsg}</div>
          <div class="actions">
            <button class="cancel-btn" onclick={onCancel}>Close</button>
          </div>
        {/if}
      {/if}
    </div>
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed; inset: 0; background: rgba(0,0,0,0.75);
    display: flex; align-items: center; justify-content: center;
    z-index: 200;
  }
  .modal {
    background: #0f172a; border: 1px solid #1e293b; border-radius: 0.75rem;
    width: min(500px, 95vw); display: flex; flex-direction: column;
  }
  .modal-header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 1rem 1.5rem; border-bottom: 1px solid #1e293b;
  }
  h2 { font-size: 1.1rem; font-weight: 700; color: #f1f5f9; margin: 0; }
  .close-btn {
    background: none; border: none; color: #64748b; font-size: 1rem;
    cursor: pointer; padding: 0.2rem 0.4rem;
  }
  .close-btn:hover { color: #e2e8f0; }
  .modal-body { padding: 1.5rem; display: flex; flex-direction: column; gap: 1.25rem; }

  .dropzone {
    border: 2px dashed #1e293b; border-radius: 0.75rem;
    padding: 2.5rem 1.5rem; text-align: center;
    display: flex; flex-direction: column; align-items: center; gap: 0.6rem;
    transition: border-color 0.15s, background 0.15s;
    cursor: pointer;
  }
  .dropzone.dragging { border-color: #3b82f6; background: rgba(59,130,246,0.06); }
  .dz-icon { font-size: 2rem; color: #334155; }
  .dz-label { color: #94a3b8; font-size: 0.9rem; }
  .dz-sub { color: #475569; font-size: 0.8rem; }
  .dz-formats { color: #334155; font-size: 0.75rem; letter-spacing: 0.04em; margin-top: 0.25rem; }
  .browse-btn {
    padding: 0.4rem 1rem; background: #1e293b; border: 1px solid #334155;
    border-radius: 0.375rem; color: #94a3b8; font-size: 0.85rem;
    cursor: pointer; transition: all 0.15s;
  }
  .browse-btn:hover { border-color: #60a5fa; color: #60a5fa; }

  .file-info { display: flex; flex-direction: column; gap: 0.25rem; align-items: center; }
  .file-name { color: #e2e8f0; font-size: 0.9rem; font-weight: 600; word-break: break-all; }
  .file-size { color: #64748b; font-size: 0.8rem; }
  .change-btn {
    background: none; border: 1px solid #334155; color: #64748b;
    border-radius: 0.3rem; padding: 0.25rem 0.6rem; font-size: 0.8rem; cursor: pointer;
  }
  .change-btn:hover { color: #94a3b8; border-color: #475569; }

  .actions { display: flex; justify-content: flex-end; gap: 0.75rem; }
  .cancel-btn {
    padding: 0.45rem 1rem; background: none; border: 1px solid #334155;
    border-radius: 0.4rem; color: #64748b; font-size: 0.9rem; cursor: pointer;
  }
  .cancel-btn:hover { border-color: #475569; color: #94a3b8; }
  .upload-btn {
    padding: 0.45rem 1.25rem; background: #2563eb; border: none;
    border-radius: 0.4rem; color: white; font-size: 0.9rem; font-weight: 600;
    cursor: pointer; transition: background 0.15s;
  }
  .upload-btn:hover:not(:disabled) { background: #3b82f6; }
  .upload-btn:disabled { opacity: 0.4; cursor: default; }

  .progress-wrap { display: flex; flex-direction: column; gap: 0.6rem; padding: 1rem 0; }
  .progress-step { color: #94a3b8; font-size: 0.9rem; text-align: center; }
  .progress-bar { height: 6px; background: #1e293b; border-radius: 3px; overflow: hidden; }
  .progress-fill { height: 100%; background: #3b82f6; border-radius: 3px; transition: width 0.4s; }
  .progress-pct { color: #60a5fa; font-size: 0.85rem; text-align: center; font-weight: 600; }

  .error {
    color: #fca5a5; background: #450a0a; border-radius: 0.5rem;
    padding: 0.6rem 0.9rem; font-size: 0.85rem;
  }
</style>
