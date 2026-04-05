import { AudioWebSocket, VideoWebSocket } from './websocket';
import { FaceEmotionDetector } from './faceDetection';

export type LevelCallback = (micLevel: number, systemLevel: number) => void;
export type StreamsCallback = (screen: MediaStream, webcam: MediaStream | null) => void;
export type AudioFeatures = { dominantBand: 'low' | 'mid' | 'high' | 'none'; flux: number };
export type AudioFeaturesCallback = (features: AudioFeatures) => void;
export type LiveEmotionCallback = (emotion: string) => void;
export type RecordingCallback = (url: string) => void;
export type StreamEndedCallback = () => void;

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
  private systemWorklet: AudioWorkletNode | null = null;
  private micWorklet: AudioWorkletNode | null = null;
  private faceDetector: FaceEmotionDetector | null = null;
  private _micLevel = 0;
  private _systemLevel = 0;
  private _levelCallback: LevelCallback | null = null;
  private _streamsCallback: StreamsCallback | null = null;
  private _audioFeaturesCallback: AudioFeaturesCallback | null = null;
  private _liveEmotionCallback: LiveEmotionCallback | null = null;
  private _recordingCallback: RecordingCallback | null = null;
  private _streamEndedCallback: StreamEndedCallback | null = null;
  private screenRecordStream: MediaStream | null = null;
  private screenRecorder: MediaRecorder | null = null;
  private screenChunks: Blob[] = [];

  public micActive = false;

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

  async start(): Promise<void> {
    this.systemStream = await navigator.mediaDevices.getDisplayMedia({
      video: { frameRate: 1, displaySurface: 'monitor' } as MediaTrackConstraints,
      audio: true,
    });
    // Return focus to the interview app after the OS/browser switches to the captured window
    window.focus();

    // Notify the app when the browser ends the screen share (e.g. user clicks "Stop sharing"
    // or a meeting notification causes Chrome to drop the capture).
    this.systemStream.getTracks().forEach((track) => {
      track.onended = () => this._streamEndedCallback?.();
    });

    try {
      this.micStream = await navigator.mediaDevices.getUserMedia({ audio: true, video: false });
      this.micActive = true;
    } catch {
      console.warn('Microphone access denied — using single-stream mode.');
    }

    try {
      this.webcamStream = await navigator.mediaDevices.getUserMedia({ video: true, audio: false });
    } catch {
      console.warn('Webcam access denied — self-view disabled.');
    }

    this.systemAudioWs.connect();
    this.videoWs.connect();
    if (this.micActive) this.micAudioWs.connect();

    await new Promise((resolve) => setTimeout(resolve, 500));

    await this.startSystemAudioCapture();
    if (this.micActive) await this.startMicCapture();
    this.startVideoCapture();
    // Load face detector in background — silently degrades if unavailable
    this.faceDetector = new FaceEmotionDetector();
    this.faceDetector.init().catch(e => {
      console.warn('Face detection unavailable:', e);
      this.faceDetector = null;
    });
    // Screen recording is opt-in — call startScreenRecording() explicitly if needed.

    // Notify caller with both streams for display
    if (this._streamsCallback) {
      this._streamsCallback(this.systemStream!, this.webcamStream);
    }
  }

  private async startSystemAudioCapture() {
    if (!this.systemStream) return;
    const tracks = this.systemStream.getAudioTracks();
    if (tracks.length === 0) return;
    this.systemAudioCtx = new AudioContext({ sampleRate: 16000 });
    await this.systemAudioCtx.audioWorklet.addModule('/pcm-processor.js');
    const source = this.systemAudioCtx.createMediaStreamSource(new MediaStream([tracks[0]]));
    this.systemWorklet = new AudioWorkletNode(this.systemAudioCtx, 'pcm-processor');
    this.systemWorklet.port.onmessage = (e: MessageEvent) => {
      if (e.data instanceof Int16Array) {
        this.systemAudioWs.send(e.data.buffer);
      } else if (e.data?.type === 'level') {
        this._systemLevel = e.data.rms;
        this._levelCallback?.(this._micLevel, this._systemLevel);
      }
    };
    source.connect(this.systemWorklet);
    this.systemWorklet.connect(this.systemAudioCtx.destination);

    // Chrome throttles setInterval in hidden tabs (2s → ~60s), so use onstatechange instead.
    // This fires immediately when the browser suspends the context and is not subject to throttling.
    this.systemAudioCtx.onstatechange = () => {
      if (this.systemAudioCtx?.state === 'suspended') {
        this.systemAudioCtx.resume().catch(() => {});
      }
    };

    // Item 8: AnalyserNode for real-time spectral features
    const analyser = this.systemAudioCtx.createAnalyser();
    analyser.fftSize = 256; // 128 bins, each ~62.5 Hz wide at 16 kHz
    source.connect(analyser);
    const freqBuf = new Uint8Array(analyser.frequencyBinCount);
    let prevFreqBuf: Uint8Array | null = null;
    this.analyserInterval = setInterval(() => {
      analyser.getByteFrequencyData(freqBuf);
      // Band averages: low=80-300 Hz (bins 1-4), mid=300-2kHz (5-32), high=2-4kHz (32-64)
      const avg = (a: number, b: number) => {
        let s = 0; for (let i = a; i < b; i++) s += freqBuf[i]; return s / (b - a);
      };
      const low = avg(1, 5), mid = avg(5, 32), high = avg(32, 64);
      const total = low + mid + high;
      let dominantBand: 'low' | 'mid' | 'high' | 'none' = 'none';
      if (total > 8) {
        dominantBand = low >= mid && low >= high ? 'low' : high >= low && high >= mid ? 'high' : 'mid';
      }
      // Spectral flux: normalised sum of absolute bin differences
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

  private async startMicCapture() {
    if (!this.micStream) return;
    const tracks = this.micStream.getAudioTracks();
    if (tracks.length === 0) return;
    this.micAudioCtx = new AudioContext({ sampleRate: 16000 });
    await this.micAudioCtx.audioWorklet.addModule('/pcm-processor.js');
    const source = this.micAudioCtx.createMediaStreamSource(new MediaStream([tracks[0]]));
    this.micWorklet = new AudioWorkletNode(this.micAudioCtx, 'pcm-processor');
    this.micWorklet.port.onmessage = (e: MessageEvent) => {
      if (e.data instanceof Int16Array) {
        this.micAudioWs.send(e.data.buffer);
      } else if (e.data?.type === 'level') {
        this._micLevel = e.data.rms;
        this._levelCallback?.(this._micLevel, this._systemLevel);
      }
    };
    source.connect(this.micWorklet);
    this.micWorklet.connect(this.micAudioCtx.destination);

    this.micAudioCtx.onstatechange = () => {
      if (this.micAudioCtx?.state === 'suspended') {
        this.micAudioCtx.resume().catch(() => {});
      }
    };
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
    // Item 4: 320×180 canvas halves payload, keeps enough detail for vision models
    const canvas = document.createElement('canvas');
    canvas.width = 320; canvas.height = 180;
    const ctx = canvas.getContext('2d')!;
    const video = document.createElement('video');
    video.muted = true; video.autoplay = true; video.playsInline = true;
    video.srcObject = new MediaStream([videoTracks[0]]);
    video.play().catch((e) => console.warn('Video play failed:', e));

    const captureFrame = async () => {
      if (video.readyState >= 2 && video.videoWidth > 0) {
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
      }
    };
    this._captureFrameFn = captureFrame;
    video.addEventListener('loadeddata', () => captureFrame(), { once: true });
    setTimeout(() => captureFrame(), 3000);
    // Item 1: 6s interval (was 12s) for faster backend sentiment updates
    this.videoInterval = setInterval(captureFrame, 6000);

    // Item 5: client-side face detection on the cropped interviewer region (500ms)
    this.faceInterval = setInterval(() => {
      if (!this.faceDetector || !this._liveEmotionCallback) return;
      if (video.readyState < 2 || video.videoWidth === 0) return;
      const emo = this.faceDetector.analyzeFrame(video, this._cropRect);
      if (emo) this._liveEmotionCallback(emo);
    }, 500);
  }

  stop() {
    // Clear before stopping tracks so onended on those tracks doesn't re-trigger handleStreamEnded.
    this._streamEndedCallback = null;
    if (this.videoInterval) clearInterval(this.videoInterval);
    if (this.faceInterval) clearInterval(this.faceInterval);
    if (this.analyserInterval) clearInterval(this.analyserInterval);
    this.faceDetector?.dispose();
    this.faceDetector = null;
    this.systemWorklet?.disconnect();
    this.micWorklet?.disconnect();
    this.systemAudioCtx?.close();
    this.micAudioCtx?.close();
    this.systemStream?.getTracks().forEach((t) => t.stop());
    this.micStream?.getTracks().forEach((t) => t.stop());
    this.webcamStream?.getTracks().forEach((t) => t.stop());
    this.systemAudioWs.disconnect();
    this.micAudioWs.disconnect();
    this.videoWs.disconnect();
    this.systemStream = null;
    this.micStream = null;
    this.webcamStream = null;
    this.micActive = false;
    // Finalise screen recording — onstop handler assembles blob and fires _recordingCallback
    if (this.screenRecorder && this.screenRecorder.state !== 'inactive') {
      this.screenRecorder.stop();
    } else {
      // No recording was started; stop any lingering tracks
      this.screenRecordStream?.getTracks().forEach((t) => t.stop());
      this.screenRecordStream = null;
    }
    this.screenRecorder = null;
  }

  get active() { return this.systemStream !== null; }

  get hasSystemAudio(): boolean {
    return (this.systemStream?.getAudioTracks().length ?? 0) > 0;
  }
}
