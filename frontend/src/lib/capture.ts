import { AudioWebSocket, VideoWebSocket } from './websocket';

export type LevelCallback = (micLevel: number, systemLevel: number) => void;

export type StreamsCallback = (screen: MediaStream, webcam: MediaStream | null) => void;

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
  private systemWorklet: AudioWorkletNode | null = null;
  private micWorklet: AudioWorkletNode | null = null;
  private _paused = false;
  private _micLevel = 0;
  private _systemLevel = 0;
  private _levelCallback: LevelCallback | null = null;
  private _streamsCallback: StreamsCallback | null = null;

  public micActive = false;

  constructor() {
    this.systemAudioWs = new AudioWebSocket('/ws/audio');
    this.micAudioWs    = new AudioWebSocket('/ws/audio/mic');
    this.videoWs       = new VideoWebSocket();
  }

  onLevel(cb: LevelCallback) { this._levelCallback = cb; }
  onStreamsReady(cb: StreamsCallback) { this._streamsCallback = cb; }

  get paused() { return this._paused; }
  pause()  { this._paused = true;  }
  resume() { this._paused = false; }
  togglePause() { this._paused = !this._paused; return this._paused; }

  async start(): Promise<void> {
    this.systemStream = await navigator.mediaDevices.getDisplayMedia({
      video: { frameRate: 1, displaySurface: 'monitor' } as MediaTrackConstraints,
      audio: true,
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
        if (!this._paused) this.systemAudioWs.send(e.data.buffer);
      } else if (e.data?.type === 'level') {
        this._systemLevel = e.data.rms;
        this._levelCallback?.(this._micLevel, this._systemLevel);
      }
    };
    source.connect(this.systemWorklet);
    this.systemWorklet.connect(this.systemAudioCtx.destination);
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
        if (!this._paused) this.micAudioWs.send(e.data.buffer);
      } else if (e.data?.type === 'level') {
        this._micLevel = e.data.rms;
        this._levelCallback?.(this._micLevel, this._systemLevel);
      }
    };
    source.connect(this.micWorklet);
    this.micWorklet.connect(this.micAudioCtx.destination);
  }

  private _captureFrameFn: (() => Promise<void>) | null = null;

  /** Trigger an immediate sentiment frame capture (e.g. when interviewer starts talking). */
  public triggerFrameCapture() {
    if (this._captureFrameFn) this._captureFrameFn();
  }

  private startVideoCapture() {
    if (!this.systemStream) return;
    const videoTracks = this.systemStream.getVideoTracks();
    if (videoTracks.length === 0) return;
    const canvas = document.createElement('canvas');
    canvas.width = 640; canvas.height = 360;
    const ctx = canvas.getContext('2d')!;
    const video = document.createElement('video');
    video.muted = true; video.autoplay = true; video.playsInline = true;
    video.srcObject = new MediaStream([videoTracks[0]]);
    video.play().catch((e) => console.warn('Video play failed:', e));

    const captureFrame = async () => {
      if (video.readyState >= 2 && video.videoWidth > 0) {
        ctx.drawImage(video, 0, 0, canvas.width, canvas.height);
        const blob = await new Promise<Blob | null>((resolve) =>
          canvas.toBlob(resolve, 'image/jpeg', 0.7)
        );
        if (blob) this.videoWs.send(await blob.arrayBuffer());
      }
    };
    this._captureFrameFn = captureFrame;
    video.addEventListener('loadeddata', () => captureFrame(), { once: true });
    // Backup initial capture in case loadeddata already fired
    setTimeout(() => captureFrame(), 3000);
    // 12s interval — fast enough to catch interviewer camera turning on mid-call
    this.videoInterval = setInterval(captureFrame, 12000);
  }

  stop() {
    if (this.videoInterval) clearInterval(this.videoInterval);
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
    this._paused = false;
  }

  get active() { return this.systemStream !== null; }
}
