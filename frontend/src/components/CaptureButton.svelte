<script lang="ts">
  import { MediaCapture } from '../lib/capture';

  const { onCapture } = $props<{ onCapture: (active: boolean) => void }>();

  let capture: MediaCapture | null = $state(null);
  let active = $state(false);
  let error = $state('');

  async function toggle() {
    error = '';
    if (active && capture) {
      capture.stop();
      capture = null;
      active = false;
      onCapture(false);
    } else {
      try {
        capture = new MediaCapture();
        await capture.start();
        active = true;
        onCapture(true);
      } catch (e) {
        error = String(e);
        capture = null;
      }
    }
  }
</script>

<div class="capture-btn-container">
  {#if error}
    <div class="capture-error">{error}</div>
  {/if}
  <button onclick={toggle} class="capture-btn" class:active>
    <span class="dot"></span>
    {active ? 'Stop Capture' : 'Capture Meeting'}
  </button>
  {#if active}
    <p class="capture-hint">
      Capturing screen audio and video. AI suggestions will appear as the interview progresses.
    </p>
  {:else}
    <p class="capture-hint">
      Click to share your meeting screen. Audio will be transcribed and AI suggestions generated in
      real time.
    </p>
  {/if}
</div>

<style>
  .capture-btn-container {
    text-align: center;
    padding: 1rem;
  }
  .capture-btn {
    display: inline-flex;
    align-items: center;
    gap: 0.75rem;
    padding: 1rem 2.5rem;
    font-size: 1.1rem;
    font-weight: 600;
    background: #1e293b;
    color: #e2e8f0;
    border: 2px solid #3b82f6;
    border-radius: 9999px;
    cursor: pointer;
    transition: all 0.2s;
  }
  .capture-btn:hover {
    background: #3b82f6;
  }
  .capture-btn.active {
    background: #0f172a;
    border-color: #ef4444;
    color: #ef4444;
  }
  .capture-btn.active:hover {
    background: #ef4444;
    color: white;
  }
  .dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: #3b82f6;
    transition: background 0.2s;
  }
  .active .dot {
    background: #ef4444;
    animation: pulse 1s infinite;
  }
  @keyframes pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.3;
    }
  }
  .capture-hint {
    margin-top: 0.75rem;
    color: #64748b;
    font-size: 0.85rem;
  }
  .capture-error {
    padding: 0.75rem;
    background: #450a0a;
    border-radius: 0.5rem;
    color: #fca5a5;
    margin-bottom: 0.75rem;
    font-size: 0.85rem;
  }
</style>
