// Shared emotion configuration — single source of truth for colors, icons, and labels.
// Used by app.svelte (coaching log colors) and SentimentBar.svelte.

export const EMOTION_COLORS: Record<string, string> = {
  engaged:       '#22c55e',
  curious:       '#3b82f6',
  neutral:       '#94a3b8',
  skeptical:     '#f59e0b',
  confused:      '#f97316',
  bored:         '#ef4444',
  pleased:       '#a78bfa',
  enthusiastic:  '#10b981',
  'wrapping up': '#6366f1',
};

export function emotionColor(e: string): string {
  return EMOTION_COLORS[e] ?? '#94a3b8';
}

export const POSITIVE_EMOTIONS = new Set(['engaged', 'pleased', 'enthusiastic', 'curious']);
export const NEGATIVE_EMOTIONS = new Set(['skeptical', 'bored', 'confused']);

export const EMOTION_CONFIG: Record<string, { color: string; icon: string; label: string }> = {
  engaged:       { color: EMOTION_COLORS.engaged,           icon: '🎯', label: 'Engaged' },
  curious:       { color: EMOTION_COLORS.curious,           icon: '🔍', label: 'Curious' },
  neutral:       { color: EMOTION_COLORS.neutral,           icon: '😐', label: 'Neutral' },
  skeptical:     { color: EMOTION_COLORS.skeptical,         icon: '🤔', label: 'Skeptical' },
  confused:      { color: EMOTION_COLORS.confused,          icon: '😕', label: 'Confused' },
  bored:         { color: EMOTION_COLORS.bored,             icon: '😑', label: 'Bored' },
  pleased:       { color: EMOTION_COLORS.pleased,           icon: '😊', label: 'Pleased' },
  enthusiastic:  { color: EMOTION_COLORS.enthusiastic,      icon: '✨', label: 'Enthusiastic' },
  'wrapping up': { color: EMOTION_COLORS['wrapping up'],    icon: '🏁', label: 'Wrapping Up' },
};
