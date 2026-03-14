<script lang="ts">
  const { tell, onClose } = $props<{ tell: string; onClose: () => void }>();

  let x = $state(Number(localStorage.getItem('whisper-x') ?? window.innerWidth - 280));
  let y = $state(Number(localStorage.getItem('whisper-y') ?? 80));
  let dragging = false;
  let dragOffX = 0, dragOffY = 0;

  function onMouseDown(e: MouseEvent) {
    dragging = true; dragOffX = e.clientX - x; dragOffY = e.clientY - y;
    window.addEventListener('mousemove', onMouseMove);
    window.addEventListener('mouseup', onMouseUp);
    e.preventDefault();
  }
  function onMouseMove(e: MouseEvent) {
    if (!dragging) return;
    x = Math.max(0, Math.min(window.innerWidth - 260, e.clientX - dragOffX));
    y = Math.max(0, Math.min(window.innerHeight - 80, e.clientY - dragOffY));
  }
  function onMouseUp() {
    dragging = false;
    localStorage.setItem('whisper-x', String(x));
    localStorage.setItem('whisper-y', String(y));
    window.removeEventListener('mousemove', onMouseMove);
    window.removeEventListener('mouseup', onMouseUp);
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="whisper" style="left: {x}px; top: {y}px">
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="drag-handle" onmousedown={onMouseDown}>
    <span class="whisper-dot"></span>
    <span class="whisper-label">Say</span>
    <button class="whisper-close" onclick={onClose}>✕</button>
  </div>
  <div class="whisper-text">{tell}</div>
</div>

<style>
  .whisper {
    position: fixed; z-index: 500;
    width: 260px;
    background: rgba(7, 16, 30, 0.92);
    border: 1px solid #1e3a5f;
    border-radius: 0.5rem;
    box-shadow: 0 4px 24px rgba(0,0,0,0.6);
    backdrop-filter: blur(8px);
    user-select: none;
  }
  .drag-handle {
    display: flex; align-items: center; gap: 0.4rem;
    padding: 0.3rem 0.5rem;
    cursor: grab; border-bottom: 1px solid #0f1e33;
  }
  .drag-handle:active { cursor: grabbing; }
  .whisper-dot { width: 6px; height: 6px; border-radius: 50%; background: #4ade80; flex-shrink: 0; animation: wpulse 2s ease-in-out infinite; }
  @keyframes wpulse { 0%,100%{opacity:1} 50%{opacity:0.4} }
  .whisper-label { font-size: 0.55rem; font-weight: 800; text-transform: uppercase; letter-spacing: 0.08em; color: #4ade80; flex: 1; }
  .whisper-close { margin-left: auto; background: none; border: none; color: #334155; font-size: 0.65rem; cursor: pointer; padding: 0 0.1rem; }
  .whisper-close:hover { color: #64748b; }
  .whisper-text { padding: 0.5rem 0.65rem; font-size: 0.9rem; font-weight: 600; color: #e2e8f0; line-height: 1.4; }
</style>
