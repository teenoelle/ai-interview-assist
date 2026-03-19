export interface PersonalityRead {
  label: string;
  description: string;
  tip: string;
  color: string;
}

export function derivePersonality(history: string[]): PersonalityRead | null {
  if (history.length < 3) return null;
  const counts: Record<string, number> = {};
  for (const e of history) counts[e] = (counts[e] ?? 0) + 1;
  const total = history.length;
  const pct = (e: string) => (counts[e] ?? 0) / total;

  const skepticScore    = pct('skeptical') + pct('confused');
  const enthusiastScore = pct('enthusiastic') + pct('pleased') + pct('engaged');
  const warmScore       = pct('curious') + pct('engaged');
  const neutralScore    = pct('neutral') + pct('wrapping up');

  if (skepticScore > 0.35)    return {
    label: 'Skeptic', color: '#f59e0b',
    description: 'Needs evidence. Lead with numbers and concrete proof.',
    tip: 'Open every answer with a metric or outcome. If they push back, say "the data showed…" rather than defending your opinion. Use STAR and land the result before the story.',
  };
  if (enthusiastScore > 0.45) return {
    label: 'Enthusiast', color: '#4ade80',
    description: 'Energized. Match their energy — be bold and direct.',
    tip: 'Mirror their pace and energy. Lead with a headline win, then expand. Use "I shipped", "I grew", "I owned" — they reward confidence. Don\'t hedge or over-qualify.',
  };
  if (warmScore > 0.45)       return {
    label: 'Connector', color: '#60a5fa',
    description: 'Relationship-driven. Emphasize team impact and collaboration.',
    tip: 'Lead with "we" — describe team wins and your role in them. Mention interpersonal moments. End each answer with a question back to build dialogue.',
  };
  if (neutralScore > 0.55)    return {
    label: 'Box-ticker', color: '#94a3b8',
    description: 'Process-oriented. Be structured, hit each criterion clearly.',
    tip: 'Follow the exact structure of each question. If they ask for three examples, give three. Signal transitions ("first…", "second…"). Confirm you\'ve addressed their criteria before moving on.',
  };
  return {
    label: 'Balanced', color: '#a78bfa',
    description: 'Mixed signals — stay versatile, adapt to each question.',
    tip: 'Watch their reaction after your opening sentence. If they lean in, go deeper. If they look ready to move on, wrap up. Match their cadence question by question.',
  };
}
