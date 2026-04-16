import { AudioWebSocket, VideoWebSocket } from './websocket';
import { FaceEmotionDetector } from './faceDetection';

export type LevelCallback = (micLevel: number, systemLevel: number) => void;
export type StreamsCallback = (screen: MediaStream, webcam: MediaStream | null) => void;
export type AudioFeatures = { dominantBand: 'low' | 'mid' | 'high' | 'none'; flux: number };
export type AudioFeaturesCallback = (features: AudioFeatures) => void;
export type LiveEmotionCallback = (emotion: string) => void;
export type RecordingCallback = (url: string) => void;
export type StreamEndedCallback = () => void;
export type ReshareNeededCallback = () => void;

export class MediaCapture {
  private systemStream: MediaStream | null = null;
  private micStream: MediaStream | null = null;
  private webcamStream: MediaStream | null = null;
  private systemAudioCtx: AudioContext | null = null;
  private micAudioCtx: AudioContext | null = null;
  private systemAudioWs: AudioWebSocket;
  private micAudioWs: AudioWebSocket;
  private videoWs: VideoWebSocket;
  private videoInterval: ReturnType<typeof setInterval> | null = null;
  private faceInterval: ReturnType<typeof setInterval> | null = null;
  private analyserInterval: ReturnType<typeof setInterval> | null = null;
  private systemWorklet: ScriptProcessorNode | null = null;
  private micWorklet: ScriptProcessorNode | null = null;
  private systemAudioSource: MediaStreamAudioSourceNode | null = null;
  private systemAnalyser: AnalyserNode | null = null;
  private videoElement: HTMLVideoElement | null = null;
  private faceDetector: FaceEmotionDetector | null = null;
  private _micLevel = 0;
  private _systemLevel = 0;
  private _levelCallback: LevelCallback | null = null;
  private _streamsCallback: StreamsCallback | null = null;
  private _audioFeaturesCallback: AudioFeaturesCallback | null = null;
  private _liveEmotionCallback: LiveEmotionCallback | null = null;
  private _recordingCallback: RecordingCallback | null = null;
  private _streamEndedCallback: StreamEndedCallback | null = null;
  private _reshareNeededCallback: ReshareNeededCallback | null = null;
  private screenRecordStream: MediaStream | null = null;
  private screenRecorder: MediaRecorder | null = null;
  private screenChunks: Blob[] = [];

  public micActive = false;
  private micDeviceId: string | null = null;

  constructor() {
    this.systemAudioWs = new AudioWebSocket('/ws/audio');
    this.micAudioWs    = new AudioWebSocket('/ws/audio/mic');
    this.videoWs       = new VideoWebSocket();
  }

  onLevel(cb: LevelCallback) { this._levelCallback = cb; }
  onStreamsReady(cb: StreamsCallback) { this._streamsCallback = cb; }
  onAudioFeatures(cb: AudioFeaturesCallback) { this._audioFeaturesCallback = cb; }
  onLiveEmotion(cb: LiveEmotionCallback) { this._liveEmotionCallback = cb; }
  onRecording(cb: RecordingCallback) { this._recordingCallback = cb; }
  onStreamEnded(cb: StreamEndedCallback) { this._streamEndedCallback = cb; }
  onReshareNeeded(cb: ReshareNeededCallback) { this._reshareNeededCallback = cb; }

  async start(): Promise<void> {
    // Create AudioContext NOW while the user gesture is still active.
    // Creating it after await getDisplayMedia() (which takes several seconds)
    // causes Chrome to start the context suspended → silent onaudioprocess buffers.
    this.systemAudioCtx = new AudioContext();
    this.systemAudioCtx.resume().catch(() => {});
    console.log('[capture] AudioContext created, state:', this.systemAudioCtx.state, 'sampleRate:', this.systemAudioCtx.sampleRate);

    console.log('[capture] getDisplayMedia...');
    this.systemStream = await navigator.mediaDevices.getDisplayMedia({
      video: { frameRate: 1 } as MediaTrackConstraints,
      audio: true,
    });
    console.log('[capture] display media ok, audio tracks:', this.systemStream.getAudioTracks().length, 'video tracks:', this.systemStream.getVideoTracks().length);
    window.focus();
    this._attachVideoEndedHandlers(this.systemStream);

    try {
      console.log('[capture] requesting mic...');
      this.micStream = await navigator.mediaDevices.getUserMedia({
        audio: {
          ...(this.micDeviceId ? { deviceId: { exact: this.micDeviceId } } : {}),
          echoCancellation: false,
          noiseSuppression: false,
          autoGainControl: true,
        },
        video: false,
      });
      this.micActive = true;
      console.log('[capture] mic ok');
    } catch (e) {
      console.warn('[capture] mic denied:', e);
    }

    try {
      console.log('[capture] requesting webcam...');
      this.webcamStream = await navigator.mediaDevices.getUserMedia({ video: true, audio: false });
      console.log('[capture] webcam ok');
    } catch (e) {
      console.warn('[capture] webcam denied:', e);
    }

    console.log('[capture] connecting websockets...');
    this.systemAudioWs.connect();
    this.videoWs.connect();
    if (this.micActive) this.micAudioWs.connect();

    await new Promise((resolve) => setTimeout(resolve, 500));

    console.log('[capture] starting system audio capture...');
    this.startSystemAudioCapture();
    console.log('[capture] system audio started');
    if (this.micActive) {
      console.log('[capture] starting mic capture...');
      this.startMicCapture();
      console.log('[capture] mic capture started');
    }
    console.log('[capture] starting video capture...');
    this.startVideoCapture();
    console.log('[capture] all started');
    // Load face detector in background — silently degrades if unavailable
    this.faceDetector = new FaceEmotionDetector();
    this.faceDetector.init().catch(e => {
      console.warn('Face detection unavailable:', e);
      this.faceDetector = null;
    });

    // Notify caller with both streams for display
    if (this._streamsCallback) {
      this._streamsCallback(this.systemStream!, this.webcamStream);
    }
  }

  /** Re-acquire the screen/audio share without tearing down the audio pipeline.
   *  Call this when a meeting popup caused the video track to end.
   *  The AudioContext, AudioWorklet, and WebSockets stay connected throughout —
   *  transcription resumes instantly once the new stream is attached. */
  async reshare(): Promise<void> {
    const newStream = await navigator.mediaDevices.getDisplayMedia({
      video: { frameRate: 1, displaySurface: 'monitor' } as MediaTrackConstraints,
      audio: true,
    });
    window.focus();

    // Stop old tracks (they're already ended but clean up anyway)
    this.systemStream?.getTracks().forEach(t => { t.onended = null; t.stop(); });
    this.systemStream = newStream;

    // Swap audio source node — keep AudioContext and AudioWorklet alive
    const audioTracks = newStream.getAudioTracks();
    if (audioTracks.length > 0 && this.systemAudioCtx && this.systemWorklet) {
      this.systemAudioSource?.disconnect();
      this.systemAudioSource = this.systemAudioCtx.createMediaStreamSource(
        new MediaStream([audioTracks[0]])
      );
      this.systemAudioSource.connect(this.systemWorklet);
      if (this.systemAnalyser) this.systemAudioSource.connect(this.systemAnalyser);
    }

    // Swap video source — keep interval and canvas alive
    const videoTracks = newStream.getVideoTracks();
    if (videoTracks.length > 0 && this.videoElement) {
      this.videoElement.srcObject = new MediaStream([videoTracks[0]]);
    }

    // Watch new video tracks for the next potential interruption
    this._attachVideoEndedHandlers(newStream);

    // Notify app so screen preview updates
    this._streamsCallback?.(newStream, this.webcamStream);
  }

  private _attachVideoEndedHandlers(stream: MediaStream) {
    stream.getVideoTracks().forEach(track => {
      track.onended = () => this._reshareNeededCallback?.();
    });
  }

  private startSystemAudioCapture() {
    if (!this.systemStream) return;
    const tracks = this.systemStream.getAudioTracks();
    if (tracks.length === 0) return;
    // systemAudioCtx was pre-created in start() during the user gesture.
    if (!this.systemAudioCtx) return;
    console.log('[capture] system AudioContext state:', this.systemAudioCtx.state, 'sampleRate:', this.systemAudioCtx.sampleRate);
    this.systemAudioCtx.resume().catch(() => {});
    this.systemAudioCtx.onstatechange = () => {
      if (this.systemAudioCtx?.state === 'suspended') {
        this.systemAudioCtx.resume().catch(() => {});
      }
    };

    this.systemAudioSource = this.systemAudioCtx.createMediaStreamSource(new MediaStream([tracks[0]]));

    // ScriptProcessorNode: synchronous, no addModule needed.
    const TARGET_RATE = 16000;
    const nativeRate = this.systemAudioCtx.sampleRate;
    const ratio = nativeRate / TARGET_RATE;
    const BUFFER = 4096;
    const CHUNK_TARGET = 8000; // 0.5 s worth at 16 kHz
    const CHUNK_NATIVE = Math.round(CHUNK_TARGET * ratio);
    let buf: number[] = [];
    // Use 2 input channels — tab capture is often stereo; mono node can produce zeros on ch0
    const numInputCh = tracks[0].getSettings().channelCount ?? 2;
    this.systemWorklet = this.systemAudioCtx.createScriptProcessor(BUFFER, Math.max(numInputCh, 2), 1);
    this.systemWorklet.onaudioprocess = (e) => {
      // Mix down to mono (average ch0+ch1) so we don't miss audio on either channel
      const ch0 = e.inputBuffer.getChannelData(0);
      const ch1 = e.inputBuffer.numberOfChannels > 1 ? e.inputBuffer.getChannelData(1) : ch0;
      const float32 = new Float32Array(ch0.length);
      for (let i = 0; i < ch0.length; i++) float32[i] = (ch0[i] + ch1[i]) * 0.5;
      for (let i = 0; i < float32.length; i++) buf.push(float32[i]);
      while (buf.length >= CHUNK_NATIVE) {
        const chunk = buf.splice(0, CHUNK_NATIVE);
        // Linear-interpolation downsample to TARGET_RATE
        const out = new Float32Array(CHUNK_TARGET);
        for (let i = 0; i < CHUNK_TARGET; i++) {
          const pos = i * ratio;
          const idx = Math.floor(pos);
          const frac = pos - idx;
          const a = chunk[idx] ?? 0;
          const b = chunk[idx + 1] ?? a;
          out[i] = a + frac * (b - a);
        }
        let sumSq = 0;
        for (let i = 0; i < out.length; i++) sumSq += out[i] * out[i];
        this._systemLevel = Math.sqrt(sumSq / out.length);
        this._levelCallback?.(this._micLevel, this._systemLevel);
        const int16 = new Int16Array(CHUNK_TARGET);
        for (let i = 0; i < CHUNK_TARGET; i++) {
          const s = Math.max(-1, Math.min(1, out[i]));
          int16[i] = s < 0 ? s * 0x8000 : s * 0x7fff;
        }
        this.systemAudioWs.send(int16.buffer);
      }
    };
    // Use a MediaStreamDestination (silent sink) instead of destination.
    // Connecting to the real destination causes Chrome to silence tab-captured audio
    // to prevent feedback loops — the input buffers become all zeros.
    const sysSink = this.systemAudioCtx.createMediaStreamDestination();
    this.systemAudioSource.connect(this.systemWorklet);
    this.systemWorklet.connect(sysSink);

    // AnalyserNode for real-time spectral features
    this.systemAnalyser = this.systemAudioCtx.createAnalyser();
    this.systemAnalyser.fftSize = 256;
    this.systemAudioSource.connect(this.systemAnalyser);
    const freqBuf = new Uint8Array(this.systemAnalyser.frequencyBinCount);
    let prevFreqBuf: Uint8Array | null = null;
    this.analyserInterval = setInterval(() => {
      this.systemAnalyser!.getByteFrequencyData(freqBuf);
      const avg = (a: number, b: number) => {
        let s = 0; for (let i = a; i < b; i++) s += freqBuf[i]; return s / (b - a);
      };
      const low = avg(1, 5), mid = avg(5, 32), high = avg(32, 64);
      const total = low + mid + high;
      let dominantBand: 'low' | 'mid' | 'high' | 'none' = 'none';
      if (total > 8) {
        dominantBand = low >= mid && low >= high ? 'low' : high >= low && high >= mid ? 'high' : 'mid';
      }
      let flux = 0;
      if (prevFreqBuf) {
        let diff = 0;
        for (let i = 0; i < freqBuf.length; i++) diff += Math.abs(freqBuf[i] - prevFreqBuf[i]);
        flux = Math.min(1, diff / (freqBuf.length * 30));
      }
      prevFreqBuf = new Uint8Array(freqBuf);
      this._audioFeaturesCallback?.({ dominantBand, flux });
    }, 250);
  }

  private startMicCapture() {
    if (!this.micStream) return;
    const tracks = this.micStream.getAudioTracks();
    if (tracks.length === 0) return;
    // Reuse system AudioContext if available (same native rate), otherwise create a new one.
    this.micAudioCtx = this.systemAudioCtx ?? new AudioContext();
    this.micAudioCtx.resume().catch(() => {});
    this.micAudioCtx.onstatechange = () => {
      if (this.micAudioCtx?.state === 'suspended') {
        this.micAudioCtx.resume().catch(() => {});
      }
    };

    const settings = tracks[0].getSettings();
    console.log('[mic] track settings:', JSON.stringify(settings));

    const TARGET_RATE = 16000;
    const nativeRate = this.micAudioCtx.sampleRate;
    const ratio = nativeRate / TARGET_RATE;
    const source = this.micAudioCtx.createMediaStreamSource(new MediaStream([tracks[0]]));
    const BUFFER = 4096;
    const CHUNK_TARGET = 8000;
    const CHUNK_NATIVE = Math.round(CHUNK_TARGET * ratio);
    let buf: number[] = [];
    let micDiag = 0;
    // Use the actual channel count (mic is usually mono=1). Requesting more channels
    // than the track provides causes Chrome on Windows to deliver all-zero buffers.
    const numMicCh = settings.channelCount ?? 1;
    this.micWorklet = this.micAudioCtx.createScriptProcessor(BUFFER, numMicCh, 1);
    this.micWorklet.onaudioprocess = (e) => {
      const ch0 = e.inputBuffer.getChannelData(0);
      const ch1 = e.inputBuffer.numberOfChannels > 1 ? e.inputBuffer.getChannelData(1) : ch0;
      if (micDiag < 10) {
        let maxAbs = 0;
        for (let i = 0; i < ch0.length; i++) maxAbs = Math.max(maxAbs, Math.abs(ch0[i]), Math.abs(ch1[i]));
        console.log(`[mic] #${micDiag} ch0[0]=${ch0[0].toFixed(6)} ch1[0]=${ch1[0].toFixed(6)} maxAbs=${maxAbs.toFixed(6)} nCh=${e.inputBuffer.numberOfChannels}`);
        micDiag++;
      }
      const float32 = ch0;
      for (let i = 0; i < float32.length; i++) buf.push(float32[i]);
      while (buf.length >= CHUNK_NATIVE) {
        const chunk = buf.splice(0, CHUNK_NATIVE);
        const out = new Float32Array(CHUNK_TARGET);
        for (let i = 0; i < CHUNK_TARGET; i++) {
          const pos = i * ratio;
          const idx = Math.floor(pos);
          const frac = pos - idx;
          const a = chunk[idx] ?? 0;
          const b = chunk[idx + 1] ?? a;
          out[i] = a + frac * (b - a);
        }
        let sumSq = 0;
        for (let i = 0; i < out.length; i++) sumSq += out[i] * out[i];
        this._micLevel = Math.sqrt(sumSq / out.length);
        this._levelCallback?.(this._micLevel, this._systemLevel);
        const int16 = new Int16Array(CHUNK_TARGET);
        for (let i = 0; i < CHUNK_TARGET; i++) {
          const s = Math.max(-1, Math.min(1, out[i]));
          int16[i] = s < 0 ? s * 0x8000 : s * 0x7fff;
        }
        this.micAudioWs.send(int16.buffer);
      }
    };
    const micSink = this.micAudioCtx.createMediaStreamDestination();
    source.connect(this.micWorklet);
    this.micWorklet.connect(micSink);
  }

  private _captureFrameFn: (() => Promise<void>) | null = null;
  private _cropRect: { x: number; y: number; w: number; h: number } | null = null;
  private _sentimentEnabled = true;

  /** Update the crop rect used for sentiment frame capture. Pass null for full frame. */
  public setCropRect(rect: { x: number; y: number; w: number; h: number } | null) {
    this._cropRect = rect;
  }

  /** Enable or disable sending video frames to the backend (saves API credits). */
  public setSentimentEnabled(enabled: boolean) {
    this._sentimentEnabled = enabled;
  }

  /** Start opt-in screen recording of this tab. Triggers a second getDisplayMedia prompt. */
  public async startScreenRecording(): Promise<void> {
    if (this.screenRecorder) return; // already recording
    try {
      this.screenRecordStream = await navigator.mediaDevices.getDisplayMedia({
        video: true, audio: true, preferCurrentTab: true,
      } as DisplayMediaStreamOptions);
      const mimeType = MediaRecorder.isTypeSupported('video/webm;codecs=vp9') ? 'video/webm;codecs=vp9' : 'video/webm';
      this.screenChunks = [];
      this.screenRecorder = new MediaRecorder(this.screenRecordStream, { mimeType });
      this.screenRecorder.ondataavailable = (e) => { if (e.data.size > 0) this.screenChunks.push(e.data); };
      this.screenRecorder.onstop = () => {
        const blob = new Blob(this.screenChunks, { type: mimeType });
        this._recordingCallback?.(URL.createObjectURL(blob));
        this.screenRecordStream?.getTracks().forEach((t) => t.stop());
        this.screenRecordStream = null;
      };
      this.screenRecorder.start(5000);
    } catch {
      console.info('Screen recording not started (cancelled or unsupported).');
    }
  }

  /** Trigger an immediate sentiment frame capture (e.g. when interviewer starts talking). */
  public triggerFrameCapture() {
    if (this._captureFrameFn && this._sentimentEnabled) this._captureFrameFn();
  }

  private startVideoCapture() {
    if (!this.systemStream) return;
    const videoTracks = this.systemStream.getVideoTracks();
    if (videoTracks.length === 0) return;
    const canvas = document.createElement('canvas');
    canvas.width = 320; canvas.height = 180;
    const ctx = canvas.getContext('2d')!;
    this.videoElement = document.createElement('video');
    this.videoElement.muted = true;
    this.videoElement.autoplay = true;
    this.videoElement.playsInline = true;
    this.videoElement.srcObject = new MediaStream([videoTracks[0]]);
    this.videoElement.play().catch((e) => console.warn('Video play failed:', e));

    const captureFrame = async () => {
      const video = this.videoElement;
      if (!video || video.readyState < 2 || video.videoWidth === 0) return;
      const crop = this._cropRect;
      if (crop) {
        const sw = video.videoWidth * crop.w;
        const sh = video.videoHeight * crop.h;
        const sx = video.videoWidth * crop.x;
        const sy = video.videoHeight * crop.y;
        ctx.drawImage(video, sx, sy, sw, sh, 0, 0, canvas.width, canvas.height);
      } else {
        ctx.drawImage(video, 0, 0, canvas.width, canvas.height);
      }
      const blob = await new Promise<Blob | null>((resolve) =>
        canvas.toBlob(resolve, 'image/jpeg', 0.7)
      );
      if (blob && this._sentimentEnabled) this.videoWs.send(await blob.arrayBuffer());
    };
    this._captureFrameFn = captureFrame;
    this.videoElement.addEventListener('loadeddata', () => captureFrame(), { once: true });
    setTimeout(() => captureFrame(), 3000);
    this.videoInterval = setInterval(captureFrame, 6000);

    this.faceInterval = setInterval(() => {
      if (!this.faceDetector || !this._liveEmotionCallback || !this.videoElement) return;
      if (this.videoElement.readyState < 2 || this.videoElement.videoWidth === 0) return;
      const emo = this.faceDetector.analyzeFrame(this.videoElement, this._cropRect);
      if (emo) this._liveEmotionCallback(emo);
    }, 500);
  }

  stop() {
    // Clear callbacks before stopping tracks so onended doesn't re-trigger them.
    this._streamEndedCallback = null;
    this._reshareNeededCallback = null;
    if (this.videoInterval) clearInterval(this.videoInterval);
    if (this.faceInterval) clearInterval(this.faceInterval);
    if (this.analyserInterval) clearInterval(this.analyserInterval);
    this.faceDetector?.dispose();
    this.faceDetector = null;
    this.systemAudioSource?.disconnect();
    this.systemAudioSource = null;
    this.systemAnalyser = null;
    this.systemWorklet?.disconnect();
    this.micWorklet?.disconnect();
    this.systemAudioCtx?.close();
    // micAudioCtx may be shared with systemAudioCtx — only close if distinct
    if (this.micAudioCtx && this.micAudioCtx !== this.systemAudioCtx) {
      this.micAudioCtx.close();
    }
    this.systemStream?.getTracks().forEach((t) => { t.onended = null; t.stop(); });
    this.micStream?.getTracks().forEach((t) => t.stop());
    this.webcamStream?.getTracks().forEach((t) => t.stop());
    this.systemAudioWs.disconnect();
    this.micAudioWs.disconnect();
    this.videoWs.disconnect();
    this.systemStream = null;
    this.micStream = null;
    this.webcamStream = null;
    this.videoElement = null;
    this.micActive = false;
    // Finalise screen recording
    if (this.screenRecorder && this.screenRecorder.state !== 'inactive') {
      this.screenRecorder.stop();
    } else {
      this.screenRecordStream?.getTracks().forEach((t) => t.stop());
      this.screenRecordStream = null;
    }
    this.screenRecorder = null;
  }

  /** Switch to a different microphone without stopping the full capture session. */
  async switchMic(deviceId: string): Promise<void> {
    this.micDeviceId = deviceId || null;
    if (!this.micActive || !this.micAudioCtx) return;

    // Tear down the current mic pipeline
    this.micWorklet?.disconnect();
    this.micWorklet = null;
    this.micStream?.getTracks().forEach(t => t.stop());
    this.micStream = null;
    this._micLevel = 0;

    try {
      this.micStream = await navigator.mediaDevices.getUserMedia({
        audio: {
          ...(deviceId ? { deviceId: { exact: deviceId } } : {}),
          echoCancellation: false,
          noiseSuppression: false,
          autoGainControl: true,
        },
        video: false,
      });
      this.startMicCapture();
      console.log('[capture] switched mic to', deviceId || 'default');
    } catch (e) {
      console.warn('[capture] switchMic failed:', e);
      this.micActive = false;
    }
  }

  get active() { return this.systemStream !== null; }

  get hasSystemAudio(): boolean {
    return (this.systemStream?.getAudioTracks().length ?? 0) > 0;
  }
}
