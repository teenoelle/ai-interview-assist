export interface PaceReading {
  wordsPerMinute: number;
  status: 'good' | 'fast' | 'slow' | 'idle';
  tip: string;
}

// Estimate WPM from a text chunk and its duration in seconds
export function analyzePace(recentTexts: string[], windowSec = 30): PaceReading {
  const totalWords = recentTexts.join(' ').split(/\s+/).filter(w => w.length > 0).length;
  const wpm = Math.round((totalWords / windowSec) * 60);

  if (totalWords === 0) return { wordsPerMinute: 0, status: 'idle', tip: '' };
  if (wpm > 180) return { wordsPerMinute: wpm, status: 'fast', tip: 'Slow down — you\'re speaking too fast' };
  if (wpm < 90) return { wordsPerMinute: wpm, status: 'slow', tip: 'Pick up your pace — you sound hesitant' };
  return { wordsPerMinute: wpm, status: 'good', tip: 'Good pace' };
}

// Detect energy from text (simple heuristic)
export function detectEnergySignals(texts: string[]): string | null {
  if (texts.length === 0) return null;
  const joined = texts.join(' ').toLowerCase();
  const trailOff = (joined.match(/\.\.\./g) ?? []).length;
  const shortAnswers = texts.filter(t => t.split(/\s+/).length < 6).length;
  const hedging = ['i think maybe', 'i guess', 'sort of', 'kind of', 'i don\'t know', 'not sure'].filter(h => joined.includes(h)).length;

  if (hedging >= 2) return 'Add confidence — avoid hedging phrases like "I think maybe" or "I guess"';
  if (trailOff >= 2) return 'Finish your sentences with conviction';
  if (shortAnswers >= 3) return 'Expand your answers — you\'re being too brief';
  return null;
}
