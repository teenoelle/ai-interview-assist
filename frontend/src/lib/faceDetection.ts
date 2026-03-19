import { FaceLandmarker, FilesetResolver } from '@mediapipe/tasks-vision';

export type FaceEmotion = 'pleased' | 'engaged' | 'curious' | 'skeptical' | 'neutral' | 'confused';

const WASM_URL = 'https://cdn.jsdelivr.net/npm/@mediapipe/tasks-vision@0.10.22/wasm';
const MODEL_URL = 'https://storage.googleapis.com/mediapipe-models/face_landmarker/face_landmarker/float16/1/face_landmarker.task';

export class FaceEmotionDetector {
  private landmarker: FaceLandmarker | null = null;
  private offscreen: HTMLCanvasElement;
  private ctx: CanvasRenderingContext2D;
  private ready = false;

  constructor() {
    this.offscreen = document.createElement('canvas');
    this.offscreen.width = 320;
    this.offscreen.height = 180;
    this.ctx = this.offscreen.getContext('2d')!;
  }

  async init(): Promise<void> {
    const filesetResolver = await FilesetResolver.forVisionTasks(WASM_URL);
    this.landmarker = await FaceLandmarker.createFromOptions(filesetResolver, {
      baseOptions: { modelAssetPath: MODEL_URL, delegate: 'GPU' },
      outputFaceBlendshapes: true,
      runningMode: 'IMAGE',
      numFaces: 1,
    });
    this.ready = true;
  }

  /** Analyze a single video frame, applying cropRect if provided. Returns null if no face found. */
  analyzeFrame(
    video: HTMLVideoElement,
    cropRect: { x: number; y: number; w: number; h: number } | null,
  ): FaceEmotion | null {
    if (!this.ready || !this.landmarker || video.videoWidth === 0) return null;
    if (cropRect) {
      const sx = video.videoWidth * cropRect.x;
      const sy = video.videoHeight * cropRect.y;
      const sw = video.videoWidth * cropRect.w;
      const sh = video.videoHeight * cropRect.h;
      this.ctx.drawImage(video, sx, sy, sw, sh, 0, 0, 320, 180);
    } else {
      this.ctx.drawImage(video, 0, 0, 320, 180);
    }
    const results = this.landmarker.detect(this.offscreen);
    const categories = results.faceBlendshapes?.[0]?.categories;
    if (!categories) return null;
    return this.blendshapesToEmotion(categories);
  }

  private blendshapesToEmotion(cats: { categoryName: string; score: number }[]): FaceEmotion {
    const g = (n: string) => cats.find(c => c.categoryName === n)?.score ?? 0;

    const smile       = (g('mouthSmileLeft') + g('mouthSmileRight')) / 2;
    const cheekSquint = (g('cheekSquintLeft') + g('cheekSquintRight')) / 2;
    const browDown    = (g('browDownLeft') + g('browDownRight')) / 2;
    const browInnerUp = g('browInnerUp');
    const eyeWide     = (g('eyeWideLeft') + g('eyeWideRight')) / 2;
    const mouthFrown  = (g('mouthFrownLeft') + g('mouthFrownRight')) / 2;

    // Genuine smile (Duchenne marker — cheeks engage)
    if (smile > 0.4 && cheekSquint > 0.3) return 'pleased';
    // Raised inner brow + wide eyes → curious/surprised
    if (browInnerUp > 0.35 && eyeWide > 0.25) return 'curious';
    // Brows down + frown → skeptical/concerned
    if (browDown > 0.4 && mouthFrown > 0.15) return 'skeptical';
    // Brows down alone → focused/confused
    if (browDown > 0.45) return 'confused';
    // Any smile without cheek squint → polite, engaged
    if (smile > 0.25) return 'engaged';
    return 'neutral';
  }

  dispose() {
    this.landmarker?.close();
    this.landmarker = null;
    this.ready = false;
  }
}
