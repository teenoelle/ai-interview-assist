const FILLER_WORDS = ['um', 'uh', 'like', 'you know', 'basically', 'actually', 'literally', 'right', 'so'];

export interface FillerCount {
  word: string;
  count: number;
}

export function countFillers(text: string): FillerCount[] {
  const lower = ` ${text.toLowerCase()} `;
  return FILLER_WORDS
    .map(word => ({
      word,
      count: (lower.match(new RegExp(`\\b${word}\\b`, 'g')) || []).length,
    }))
    .filter(f => f.count > 0);
}

export function totalFillers(counts: FillerCount[]): number {
  return counts.reduce((s, f) => s + f.count, 0);
}
