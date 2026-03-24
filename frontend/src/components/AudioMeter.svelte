<script lang="ts">
  const { micLevel, systemLevel, paused, capturing = false } = $props<{
    micLevel: number;
    systemLevel: number;
    paused: boolean;
    capturing?: boolean;
  }>();

  // Smooth the raw RMS to a visible 0-1 scale (RMS values are typically 0-0.3)
  const micPct  = $derived(Math.min(100, micLevel * 400));
  const sysPct  = $derived(Math.min(100, systemLevel * 400));
  const sysWarn = $derived(capturing && !paused && sysPct < 2);
</script>

<div class="meter" class:paused title="Audio input levels — MIC is your microphone, SYS is system audio (interviewer's voice via screen share)">
  <div class="bar-wrap" title="MIC — your microphone input level. Green bar = audio is being picked up and will be transcribed as 'You'">
    <span class="lbl">MIC</span>
    <div class="track"><div class="fill mic" style="width: {micPct}%"></div></div>
  </div>
  <div class="bar-wrap" title={sysWarn ? "SYS — no system audio detected. Stop capture, reshare your screen, and tick 'Share system audio' in the browser dialog" : "SYS — system audio level (interviewer's voice from Zoom/Teams). Blue bar = audio is being received and will be transcribed as 'Interviewer'"}>
    <span class="lbl" class:lbl-warn={sysWarn}>SYS</span>
    <div class="track"><div class="fill sys" style="width: {sysPct}%"></div></div>
  </div>
  {#if sysWarn}
    <span class="sys-warn">no system audio</span>
  {/if}
  {#if paused}
    <span class="paused-badge" title="Audio capture is paused — transcription and AI responses are suspended">PAUSED</span>
  {/if}
</div>

<style>
  .meter {
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 0.3rem 0.6rem;
    background: #0f172a;
    border: 1px solid #1e293b;
    border-radius: 0.375rem;
    min-width: 90px;
  }
  .meter.paused { opacity: 0.5; }
  .bar-wrap { display: flex; align-items: center; gap: 4px; }
  .lbl { font-size: var(--fs-xs); color: #475569; font-weight: 700; letter-spacing: 0.05em; width: 22px; }
  .lbl-warn { color: #f59e0b; }
  .sys-warn { font-size: var(--fs-xs); color: #f59e0b; font-weight: 600; text-align: center; letter-spacing: 0.05em; }
  .track {
    flex: 1; height: 4px; background: #1e293b; border-radius: 9999px; overflow: hidden;
  }
  .fill { height: 100%; border-radius: 9999px; transition: width 0.1s ease; }
  .fill.mic { background: #4ade80; }
  .fill.sys { background: #60a5fa; }
  .paused-badge {
    font-size: var(--fs-xs); color: #f59e0b; font-weight: 700;
    text-align: center; letter-spacing: 0.1em; margin-top: 1px;
  }
</style>
