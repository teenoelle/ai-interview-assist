export function splitMultiQuestions(text: string): string[] {
  if (!text.includes('?')) return [text];
  const parts = text.split(/\?(?=\s+(?:[A-Z]|\b(?:also|and|what|how|why|when|where|who|can|could|would|did|do|does|is|are|was|were)\b))/);
  const cleaned = parts
    .map(p => p.trim())
    .filter(p => p.length > 8)
    .map(p => p.endsWith('?') ? p : p + '?');
  return cleaned.length >= 2 ? cleaned : [text];
}

export function fmtTime(ms: number): string {
  const s = Math.floor(ms / 1000);
  const m = Math.floor(s / 60);
  return `${m}:${String(s % 60).padStart(2, '0')}`;
}

export function fmtAgo(ts: number): string {
  const s = Math.floor((Date.now() - ts) / 1000);
  if (s < 10) return 'now';
  if (s < 60) return `${s}s`;
  const m = Math.floor(s / 60);
  return `${m}m`;
}
