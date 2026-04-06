<script lang="ts">
  import { untrack } from 'svelte';
  import { MediaCapture } from '../lib/capture';
  type StreamsReadyCallback = (screen: MediaStream, webcam: MediaStream | null) => void;
  const { onCapture, onStreams, onReady, onLevel: onLevelProp, onRecording, onStreamEnded: onStreamEndedProp, initialCapture = null } = $props<{
    onCapture: (active: boolean) => void;
    onStreams?: StreamsReadyCallback;
    onReady?: (cap: MediaCapture) => void;
    onLevel?: (mic: number, sys: number) => void;
    onRecording?: (url: string) => void;
    onStreamEnded?: () => void;
    initialCapture?: MediaCapture | null;
  }>();

  function handleStreamEnded() {
    if (capture) {
      capture.stop();
      capture = null;
      micLevel = 0;
      systemLevel = 0;
      onCapture(false);
      onStreamEndedProp?.();
    }
  }

  function handleReshareNeeded() {
    reshareNeeded = true;
  }

  let capture: MediaCapture | null = $state(null);
  let starting = $state(false);
  let resharing = $state(false);
  let reshareNeeded = $state(false);
  let active = $derived(capture !== null || starting);
  let error = $state('');
  let micLevel = $state(0);
  let systemLevel = $state(0);

  // Adopt a pre-created capture instance (from async setup transition).
  // untrack() on the capture read prevents this effect from re-running when capture
  // is set to null by toggle() — avoiding the race where initialCapture is still
  // non-null and immediately re-sets capture, leaving the button stuck on "Stop".
  $effect(() => {
    if (initialCapture) {
      untrack(() => {
        if (!capture) capture = initialCapture;
      });
    }
  });

  async function toggle() {
    error = '';
    reshareNeeded = false;
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
        cap.onStreamEnded(handleStreamEnded);
        cap.onReshareNeeded(handleReshareNeeded);
        await cap.start();
        capture = cap;
        onCapture(true);
        onReady?.(cap);
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

  async function reshare() {
    if (!capture || resharing) return;
    resharing = true;
    try {
      await capture.reshare();
      reshareNeeded = false;
    } catch (e: unknown) {
      const msg = String(e);
      if (!msg.includes('Permission denied') && !msg.includes('NotAllowedError')) {
        error = msg;
      }
      // If user cancelled the dialog, keep reshareNeeded = true so they can try again
    } finally {
      resharing = false;
    }
  }
</script>

<div class="capture-btn-container">
  {#if error}<div class="capture-error">{error}</div>{/if}
  {#if reshareNeeded}
    <div class="reshare-banner">
      Screen share interrupted — mic still recording.
      <button class="reshare-btn" onclick={reshare} disabled={resharing}>
        {resharing ? 'Sharing…' : '↺ Reshare Screen'}
      </button>
    </div>
  {/if}
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
  .reshare-banner {
    display: flex; align-items: center; gap: 0.5rem;
    padding: 0.4rem 0.75rem; background: #451a03; border-radius: 0.375rem;
    color: #fdba74; font-size: var(--fs-base);
  }
  .reshare-btn {
    padding: 0.2rem 0.6rem; font-size: var(--fs-base); font-weight: 600;
    background: #ea580c; color: white; border: none; border-radius: 0.25rem;
    cursor: pointer; white-space: nowrap;
  }
  .reshare-btn:hover:not(:disabled) { background: #c2410c; }
  .reshare-btn:disabled { opacity: 0.6; cursor: default; }
</style>
