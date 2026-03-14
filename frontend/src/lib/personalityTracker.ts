export interface PersonalityRead {
  label: string;
  description: string;
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

  if (skepticScore > 0.35)    return { label: 'Skeptic',    description: 'Needs evidence. Lead with numbers and concrete proof.', color: '#f59e0b' };
  if (enthusiastScore > 0.45) return { label: 'Enthusiast', description: 'Energized. Match their energy — be bold and direct.', color: '#4ade80' };
  if (warmScore > 0.45)       return { label: 'Connector',  description: 'Relationship-driven. Emphasize team impact and collaboration.', color: '#60a5fa' };
  if (neutralScore > 0.55)    return { label: 'Box-ticker', description: 'Process-oriented. Be structured, hit each criterion clearly.', color: '#94a3b8' };
  return { label: 'Balanced', description: 'Mixed signals — stay versatile, adapt to each question.', color: '#a78bfa' };
}
