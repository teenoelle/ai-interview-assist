import { AudioWebSocket, VideoWebSocket } from './websocket';

export class MediaCapture {
  private stream: MediaStream | null = null;
  private audioCtx: AudioContext | null = null;
  private audioWs: AudioWebSocket;
  private videoWs: VideoWebSocket;
  private videoInterval: ReturnType<typeof setInterval> | null = null;
  private workletNode: AudioWorkletNode | null = null;

  constructor() {
    this.audioWs = new AudioWebSocket();
    this.videoWs = new VideoWebSocket();
  }

  async start(): Promise<void> {
    this.stream = await navigator.mediaDevices.getDisplayMedia({
      video: { frameRate: 1 } as MediaTrackConstraints,
      audio: true,
    });

    this.audioWs.connect();
    this.videoWs.connect();

    // Wait for WS connections to open
    await new Promise((resolve) => setTimeout(resolve, 500));

    await this.startAudioCapture();
    this.startVideoCapture();
  }

  private async startAudioCapture() {
    if (!this.stream) return;

    const audioTracks = this.stream.getAudioTracks();
    if (audioTracks.length === 0) {
      console.warn('No audio track available');
      return;
    }

    const audioStream = new MediaStream([audioTracks[0]]);
    this.audioCtx = new AudioContext({ sampleRate: 16000 });

    await this.audioCtx.audioWorklet.addModule('/pcm-processor.js');
    const source = this.audioCtx.createMediaStreamSource(audioStream);
    this.workletNode = new AudioWorkletNode(this.audioCtx, 'pcm-processor');

    this.workletNode.port.onmessage = (e: MessageEvent) => {
      const int16Data: Int16Array = e.data;
      this.audioWs.send(int16Data.buffer);
    };

    source.connect(this.workletNode);
    this.workletNode.connect(this.audioCtx.destination);
  }

  private startVideoCapture() {
    if (!this.stream) return;
    const videoTracks = this.stream.getVideoTracks();
    if (videoTracks.length === 0) return;

    const canvas = document.createElement('canvas');
    canvas.width = 640;
    canvas.height = 360;
    const ctx = canvas.getContext('2d')!;

    const video = document.createElement('video');
    video.muted = true;
    video.autoplay = true;
    video.playsInline = true;
    video.srcObject = new MediaStream([videoTracks[0]]);
    video.play().catch((e) => console.warn('Video play failed:', e));

    const captureFrame = async () => {
      if (video.readyState >= 2 && video.videoWidth > 0) {
        ctx.drawImage(video, 0, 0, canvas.width, canvas.height);
        const blob = await new Promise<Blob | null>((resolve) =>
          canvas.toBlob(resolve, 'image/jpeg', 0.7)
        );
        if (blob) {
          const buf = await blob.arrayBuffer();
          this.videoWs.send(buf);
        }
      }
    };

    // Capture first frame as soon as video is ready, then every 30s
    video.addEventListener('loadeddata', () => captureFrame(), { once: true });
    this.videoInterval = setInterval(captureFrame, 30000);
  }

  stop() {
    if (this.videoInterval) clearInterval(this.videoInterval);
    this.workletNode?.disconnect();
    this.audioCtx?.close();
    this.stream?.getTracks().forEach((t) => t.stop());
    this.audioWs.disconnect();
    this.videoWs.disconnect();
    this.stream = null;
    this.audioCtx = null;
  }

  get active() {
    return this.stream !== null;
  }
}
