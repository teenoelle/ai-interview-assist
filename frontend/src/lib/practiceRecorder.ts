export interface PracticeResult {
  transcript: string;
  duration: number;
  wordCount: number;
  fillerCount: number;
  fillerDetail: string;
}

const FILLERS = ['um', 'uh', 'like', 'you know', 'kind of', 'sort of', 'basically', 'literally', 'right'];

export class PracticeRecorder {
  private recognition: any | null = null;
  private startTime = 0;
  private finalText = '';

  get supported(): boolean {
    return typeof window !== 'undefined' &&
      ('SpeechRecognition' in window || 'webkitSpeechRecognition' in window);
  }

  start(onInterim: (text: string) => void): void {
    this.finalText = '';
    this.startTime = Date.now();

    if (!this.supported) return;

    const SR = (window as any).SpeechRecognition || (window as any).webkitSpeechRecognition;
    this.recognition = new SR();
    this.recognition.continuous = true;
    this.recognition.interimResults = true;
    this.recognition.lang = 'en-US';

    this.recognition.onresult = (e: any) => {
      let interim = '';
      for (let i = e.resultIndex; i < e.results.length; i++) {
        const t = e.results[i][0].transcript;
        if (e.results[i].isFinal) this.finalText += t + ' ';
        else interim += t;
      }
      onInterim((this.finalText + interim).trim());
    };

    this.recognition.onerror = () => {};
    this.recognition.onend = () => {};
    this.recognition.start();
  }

  stop(): PracticeResult {
    const duration = Math.max(1, (Date.now() - this.startTime) / 1000);
    try { this.recognition?.stop(); } catch { /* ignore */ }
    this.recognition = null;

    const text = this.finalText.trim();
    const words = text.split(/\s+/).filter(Boolean);
    const wordCount = words.length;
    const lower = text.toLowerCase();

    const counts: Record<string, number> = {};
    for (const filler of FILLERS) {
      const escaped = filler.replace(/\s+/g, '\\s+');
      const re = new RegExp(`\\b${escaped}\\b`, 'gi');
      const m = lower.match(re);
      if (m?.length) counts[filler] = m.length;
    }
    const fillerCount = Object.values(counts).reduce((a, b) => a + b, 0);
    const fillerDetail = Object.entries(counts).map(([f, c]) => `${f} ×${c}`).join(', ');

    return { transcript: text, duration, wordCount, fillerCount, fillerDetail };
  }

  abort(): void {
    try { this.recognition?.abort(); } catch { /* ignore */ }
    this.recognition = null;
  }
}
