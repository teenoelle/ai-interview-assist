<script lang="ts">
  import { MediaCapture } from '../lib/capture';
  type StreamsReadyCallback = (screen: MediaStream, webcam: MediaStream | null) => void;
  const { onCapture, onStreams, onReady, onLevel: onLevelProp, onRecording, initialCapture = null } = $props<{
    onCapture: (active: boolean) => void;
    onStreams?: StreamsReadyCallback;
    onReady?: (cap: MediaCapture) => void;
    onLevel?: (mic: number, sys: number) => void;
    onRecording?: (url: string) => void;
    initialCapture?: MediaCapture | null;
  }>();

  let capture: MediaCapture | null = $state(null);
  let starting = $state(false); // true only during the async start() call
  let active = $derived(capture !== null || starting);
  let error = $state('');
  let micLevel = $state(0);
  let systemLevel = $state(0);

  // Adopt a pre-created capture instance (from async setup transition)
  $effect(() => {
    if (initialCapture && !capture) {
      capture = initialCapture;
    }
  });

  async function toggle() {
    error = '';
    if (capture) {
      capture.stop();
      capture = null;
      micLevel = 0;
      systemLevel = 0;
      onCapture(false);
    } else if (!starting) {
      starting = true;
      try {
        const cap = new MediaCapture();
        cap.onLevel((mic, sys) => { micLevel = mic; systemLevel = sys; onLevelProp?.(mic, sys); });
        if (onStreams) cap.onStreamsReady(onStreams);
        if (onRecording) cap.onRecording(onRecording);
        await cap.start();
        capture = cap;
        onCapture(true);
        onReady?.(cap);
        // Warn if no system audio was captured (user didn't tick the checkbox)
        if (!cap.hasSystemAudio) {
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
      } finally {
        starting = false;
      }
    }
  }
</script>

<div class="capture-btn-container">
  {#if error}<div class="capture-error">{error}</div>{/if}
  <button onclick={toggle} class="capture-btn" class:active>
    <span class="dot"></span>
    {starting ? 'Starting…' : capture ? 'Stop' : 'Capture Meeting'}
  </button>
</div>

<style>
  .capture-btn-container { display: flex; flex-direction: column; align-items: flex-end; gap: 0.25rem; }
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
