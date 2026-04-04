export interface CombinedVoice {
  id: string;       // "os:...", "piper:...", or "browser:..."
  name: string;
  source: 'os' | 'piper' | 'browser';
}

let audioCtx: AudioContext | null = null;
let currentSource: AudioBufferSourceNode | null = null;
let currentGain: GainNode | null = null;
let _sinkId = '';

export async function getAudioOutputs(): Promise<{ deviceId: string; label: string }[]> {
  try {
    const devices = await navigator.mediaDevices.enumerateDevices();
    return devices
      .filter(d => d.kind === 'audiooutput')
      .map(d => ({ deviceId: d.deviceId, label: d.label || `Speaker (${d.deviceId.slice(0, 6)})` }));
  } catch {
    return [];
  }
}

export function setOutputDevice(deviceId: string) {
  _sinkId = deviceId;
  // Recreate context on next speak() so it picks up new sinkId
  if (audioCtx && audioCtx.state !== 'closed') {
    audioCtx.close();
    audioCtx = null;
  }
}

function getAudioCtx(): AudioContext {
  if (!audioCtx || audioCtx.state === 'closed') {
    audioCtx = new AudioContext();
    if (_sinkId && 'setSinkId' in audioCtx) {
      (audioCtx as any).setSinkId(_sinkId).catch(() => {});
    }
  }
  return audioCtx;
}

// --- Voice loading ---

export async function loadAllVoices(): Promise<CombinedVoice[]> {
  const [backend, browser] = await Promise.all([fetchBackendVoices(), waitForBrowserVoices()]);
  return [...backend, ...browser];
}

async function fetchBackendVoices(): Promise<CombinedVoice[]> {
  try {
    const r = await fetch('/api/tts/voices');
    if (!r.ok) return [];
    const list: { id: string; name: string; source: string }[] = await r.json();
    return list.map(v => ({ id: v.id, name: v.name, source: v.source as 'os' | 'piper' }));
  } catch {
    return [];
  }
}

function waitForBrowserVoices(): Promise<CombinedVoice[]> {
  return new Promise(resolve => {
    const get = () => {
      const all = speechSynthesis.getVoices();
      const local = all.filter(v => v.localService);
      const list = local.length > 0 ? local : all;
      return list.map(v => ({
        id: `browser:${v.voiceURI}`,
        name: `${v.name} (Browser)`,
        source: 'browser' as const,
      }));
    };
    const voices = get();
    if (voices.length > 0) { resolve(voices); return; }
    const handler = () => { speechSynthesis.removeEventListener('voiceschanged', handler); resolve(get()); };
    speechSynthesis.addEventListener('voiceschanged', handler);
    setTimeout(() => { speechSynthesis.removeEventListener('voiceschanged', handler); resolve(get()); }, 2000);
  });
}

// --- Playback ---

export function stopSpeaking() {
  if (currentSource) {
    try { currentSource.stop(); } catch { /* already stopped */ }
    currentSource = null;
    currentGain = null;
  }
  speechSynthesis.cancel();
}

export function isSpeaking(): boolean {
  if (currentSource !== null) return true;
  return speechSynthesis.speaking;
}

export async function speak(text: string, voiceId: string, rate: number, volume: number): Promise<void> {
  stopSpeaking();
  if (!text) return;

  if (voiceId.startsWith('browser:')) {
    const uri = voiceId.slice(8);
    const voice = speechSynthesis.getVoices().find(v => v.voiceURI === uri);
    const utt = new SpeechSynthesisUtterance(text);
    if (voice) utt.voice = voice;
    utt.rate = rate;
    utt.volume = volume;
    speechSynthesis.speak(utt);
  } else {
    // OS or Piper — ask backend to synthesize, then play via AudioContext
    try {
      const r = await fetch('/api/tts/speak', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ text, voice_id: voiceId, rate, volume }),
      });
      if (!r.ok) return;
      const arrayBuffer = await r.arrayBuffer();

      const ctx = getAudioCtx();
      if (ctx.state === 'suspended') await ctx.resume();

      // decodeAudioData fully decodes before playback — no startup clipping
      const decoded = await ctx.decodeAudioData(arrayBuffer);

      const gain = ctx.createGain();
      gain.gain.value = Math.min(4, Math.max(0, volume));
      gain.connect(ctx.destination);

      const source = ctx.createBufferSource();
      source.buffer = decoded;
      source.connect(gain);
      currentSource = source;
      currentGain = gain;
      source.onended = () => {
        if (currentSource === source) {
          currentSource = null;
          currentGain = null;
        }
      };
      source.start(0);
    } catch { /* ignore */ }
  }
}
