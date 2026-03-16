<script lang="ts">
  import { MediaCapture } from '../lib/capture';
  import AudioMeter from './AudioMeter.svelte';
  type StreamsReadyCallback = (screen: MediaStream, webcam: MediaStream | null) => void;
  const { onCapture, onStreams, onReady } = $props<{
    onCapture: (active: boolean) => void;
    onStreams?: StreamsReadyCallback;
    onReady?: (cap: MediaCapture) => void;
  }>();

  let capture: MediaCapture | null = $state(null);
  let active = $state(false);
  let paused = $state(false);
  let error = $state('');
  let micLevel = $state(0);
  let systemLevel = $state(0);

  async function toggle() {
    error = '';
    if (active && capture) {
      capture.stop();
      capture = null;
      active = false;
      paused = false;
      micLevel = 0;
      systemLevel = 0;
      onCapture(false);
    } else {
      try {
        capture = new MediaCapture();
        capture.onLevel((mic, sys) => { micLevel = mic; systemLevel = sys; });
        if (onStreams) capture.onStreamsReady(onStreams);
        await capture.start();
        active = true;
        onCapture(true);
        onReady?.(capture);
        // Warn if no system audio was captured (user didn't tick the checkbox)
        if (!capture.hasSystemAudio) {
          error = 'No system audio captured — interviewer audio won\'t be transcribed. Stop, reshare your screen, and tick "Share system audio" in the browser dialog. For Zoom/Teams desktop, share your Entire Screen.';
        }
      } catch (e: unknown) {
        const msg = String(e);
        if (msg.includes('Permission denied') || msg.includes('NotAllowedError')) {
          error = 'Screen share permission denied. Click "Capture Meeting", select Entire Screen, and check "Share system audio" for Zoom/Teams.';
        } else if (msg.includes('NotFoundError')) {
          error = 'No screen or microphone found. Check your devices and try again.';
        } else {
          error = msg;
        }
        capture = null;
      }
    }
  }

  function togglePause() {
    if (!capture) return;
    paused = capture.togglePause();
  }
</script>

<div class="capture-btn-container">
  {#if error}<div class="capture-error">{error}</div>{/if}
  <div class="controls">
    <button onclick={toggle} class="capture-btn" class:active>
      <span class="dot"></span>
      {active ? 'Stop' : 'Capture Meeting'}
    </button>
    {#if active}
      <button onclick={togglePause} class="pause-btn" class:paused title="Pause/resume audio (P)">
        {paused ? '▶' : '⏸'}
      </button>
      <AudioMeter {micLevel} {systemLevel} {paused} />
    {/if}
  </div>
</div>

<style>
  .capture-btn-container { display: flex; flex-direction: column; align-items: flex-end; gap: 0.25rem; }
  .controls { display: flex; align-items: center; gap: 0.5rem; }
  .capture-btn {
    display: inline-flex; align-items: center; gap: 0.5rem;
    padding: 0.5rem 1.25rem; font-size: var(--fs-base); font-weight: 600;
    background: #1e293b; color: #e2e8f0;
    border: 2px solid #3b82f6; border-radius: 9999px; cursor: pointer;
    transition: all 0.2s; white-space: nowrap;
  }
  .capture-btn:hover { background: #3b82f6; }
  .capture-btn.active { background: #0f172a; border-color: #ef4444; color: #ef4444; }
  .capture-btn.active:hover { background: #ef4444; color: white; }
  .pause-btn {
    width: 2rem; height: 2rem; display: flex; align-items: center; justify-content: center;
    background: #1e293b; border: 1px solid #334155; border-radius: 50%;
    color: #94a3b8; cursor: pointer; font-size: var(--fs-base); transition: all 0.15s;
  }
  .pause-btn:hover { border-color: #f59e0b; color: #f59e0b; }
  .pause-btn.paused { border-color: #f59e0b; color: #f59e0b; background: #1a1500; }
  .dot {
    width: 8px; height: 8px; border-radius: 50%;
    background: #3b82f6; transition: background 0.2s;
  }
  .active .dot { background: #ef4444; animation: pulse 1s infinite; }
  @keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.3; } }
  .capture-error {
    padding: 0.5rem; background: #450a0a; border-radius: 0.375rem;
    color: #fca5a5; font-size: var(--fs-base);
  }
</style>
