export interface RedFlag {
  category: string;
  coachingNote: string;
}

interface Pattern { triggers: string[]; category: string; coachingNote: string; }

const PATTERNS: Pattern[] = [
  { triggers: ['why did you leave', 'why are you leaving', 'why did you quit', 'why did you resign'],
    category: 'Departure', coachingNote: 'Stay positive — growth opportunity, not running away. Never criticize previous employer.' },
  { triggers: ['biggest weakness', 'area to improve', 'room for improvement', 'what would you change about yourself'],
    category: 'Weakness', coachingNote: 'Real but non-critical flaw. Show self-awareness + active steps to address it.' },
  { triggers: ['gap in', 'what were you doing between', 'employment gap', 'break from work'],
    category: 'Gap', coachingNote: 'Brief honest answer. Pivot to skills or projects you developed during that time.' },
  { triggers: ['overqualified', 'too experienced', 'why this level', 'step down'],
    category: 'Overqualified', coachingNote: 'Address commitment and excitement directly. Explain why THIS role fits your goals now.' },
  { triggers: ['fired', 'let go', 'terminated', 'laid off', 'made redundant'],
    category: 'Termination', coachingNote: 'Brief and factual, no blame. What you learned and how you moved forward.' },
{ triggers: ['only stayed', 'only a year', 'job hopping', 'move around a lot', 'short time at'],
    category: 'Short Tenure', coachingNote: 'Brief explanation (growth, restructure, opportunity), then pivot to what you achieved.' },
];

export function detectRedFlag(question: string): RedFlag | null {
  const lower = question.toLowerCase();
  for (const p of PATTERNS) {
    if (p.triggers.some(t => lower.includes(t))) {
      return { category: p.category, coachingNote: p.coachingNote };
    }
  }
  return null;
}
