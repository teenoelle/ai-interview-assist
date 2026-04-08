export type QuestionTag =
  | 'smalltalk'
  | 'behavioral'
  | 'technical'
  | 'culture'
  | 'character'
  | 'values'
  | 'salary'
  | 'closing'
  | 'personal'
  | 'motivation'
  | 'fit'
  | 'future'
  | 'strengths'
  | 'weaknesses'
  | 'situational'
  | 'general';

// ── Trigger lists ─────────────────────────────────────────────────────────────

const SMALLTALK   = ['how are you', "how's it going", 'how is it going', 'how have you been', 'how is your day', 'how was your day', 'how was your weekend', 'how was your morning', 'how are you doing', 'how are you today', 'nice to meet you', 'great to meet you', 'pleasure to meet you', 'lovely to meet you', 'good to meet you', 'wonderful to meet you', 'ready to get started', 'shall we get started', 'before we begin', 'before we get started'];
const PERSONAL    = ['tell me about yourself', 'tell us about yourself', 'walk me through your background', 'walk us through your background', 'walk me through your experience', 'walk us through your experience', 'introduce yourself', 'give me a brief overview', 'tell me a little about yourself', 'share your background', 'tell me about your background', 'tell me about your experience', 'take me through your background'];
const MOTIVATION  = ['why do you want', 'why are you interested', 'why this role', 'why this company', 'why our company', 'why this position', 'what draws you to', 'what attracted you to', 'why did you apply', 'what interests you about', 'what excites you about', 'what brings you to'];
const FIT         = ['overqualified', 'seem overqualified', 'appears overqualified', 'why junior', 'why a junior', 'why a lower', 'why a more junior', 'why are you applying for a junior', 'why are you applying for such', 'step back', 'step down', 'lower level', 'more entry-level', 'why would you take a', 'seems like a step back', 'seems like a step down', 'taking a step back', 'taking a step down'];
const FUTURE      = ['five years', '5 years', 'see yourself in', 'career goals', 'long-term goal', 'long term goal', 'where do you see yourself', 'how do you see yourself growing', 'where do you want to be', 'what are your long', 'career path'];
const STRENGTHS   = ['greatest strength', 'biggest strength', 'what are your strengths', 'key strengths', 'what do you do well', 'strongest skill', 'what makes you good at', 'what are you good at', 'what would your colleagues say about you', 'what would your colleagues say you', 'what would your coworkers say about you', 'what would your manager say about you', 'what would your teammates say about you', 'how would your colleagues describe you', 'how would your coworkers describe you', 'how would your manager describe you', 'how would your teammates describe you', 'how would others describe you', 'how would people describe you'];
const CHARACTER   = ['what would your friends say about you', 'what would your family say about you', 'how would your friends describe you', 'describe yourself outside of work', 'outside of your professional life', 'how do people who know you well', 'how would people who know you', 'what do your close friends say', 'how would you describe yourself as a person', 'what kind of person are you', 'how would someone who knows you personally'];
const WEAKNESSES  = ['greatest weakness', 'biggest weakness', 'area for improvement', 'working to improve', 'what do you struggle', 'what would your manager say you need', 'development area', 'where do you need to improve', 'what would colleagues say', 'tell me about a weakness', 'describe a weakness', 'what do you find challenging', 'what do you find difficult professionally'];
const SITUATIONAL = ['what would you do if', 'how would you handle', 'how would you approach', 'imagine you', 'suppose you', 'if you were to', 'how would you deal with', 'hypothetically', 'if you joined and', 'if you discovered', 'what would your approach be if', 'how would you respond if'];
const BEHAVIORAL  = ['tell me about a time', 'describe a situation', 'give me an example', 'have you ever', 'what was a time', 'share an experience', 'when have you', 'describe when', 'describe a time', 'can you give me an example', 'how do you prioritize'];
const TECHNICAL   = ['how would you build', 'design a', 'implement', 'algorithm', 'complexity', 'debug', 'architecture', 'system design', 'code', 'framework', 'database', 'api', 'performance', 'scale', 'optimize', 'stay current with'];
const SALARY      = ['salary', 'compensation', 'pay', 'benefits', 'equity', 'stock', 'bonus', 'offer', 'package', 'rate', 'expectations'];
const CULTURE     = ['culture', 'work style', 'remote', 'work-life', 'mission', 'motivate', 'what drives you', 'how do you collaborate', 'how do you work with', 'how do you handle disagreement', 'cross-functional', 'working across'];
const VALUES      = ['what do you look for in', 'what are you looking for in', 'what matters most to you in', 'what is important to you in', 'what do you need from a', 'what do you value in', 'what kind of manager', 'what kind of environment', 'what kind of leadership', 'what does your ideal', 'what would your ideal', 'what are you looking for in your next', 'what are you looking for in a new'];
const CLOSING     = ['any questions', 'questions for us', 'questions for me', 'next steps', 'hear from us', 'back to you', 'anything else you', 'is there anything you'];

// ── Scoring ───────────────────────────────────────────────────────────────────

function score(triggers: string[], lower: string): number {
  return triggers.filter(t => lower.includes(t)).length;
}

// Priority-ordered: higher-priority types listed first — tiebreaker when scores are equal.
// Order mirrors the backend classify_question priority.
const PRIORITY: Array<[string[], QuestionTag]> = [
  [SMALLTALK,   'smalltalk'],
  [PERSONAL,    'personal'],
  [FIT,         'fit'],
  [MOTIVATION,  'motivation'],
  [FUTURE,      'future'],
  [CLOSING,     'closing'],
  [SALARY,      'salary'],
  [STRENGTHS,   'strengths'],
  [CHARACTER,   'character'],
  [WEAKNESSES,  'weaknesses'],
  [BEHAVIORAL,  'behavioral'],
  [SITUATIONAL, 'situational'],
  [TECHNICAL,   'technical'],
  [CULTURE,     'culture'],
  [VALUES,      'values'],
];

export function tagQuestion(q: string): QuestionTag {
  const lower = q.toLowerCase();
  const scored = PRIORITY.map(([triggers, tag]) => ({ tag, s: score(triggers, lower) }));
  const max = Math.max(...scored.map(x => x.s));
  if (max > 0) return scored.find(x => x.s === max)!.tag;
  return 'general';
}

// ── Display config ────────────────────────────────────────────────────────────

export const TAG_CONFIG: Record<QuestionTag, { label: string; color: string; bg: string }> = {
  smalltalk:   { label: 'Q: Small Talk',  color: '#67e8f9', bg: '#031a20' },
  personal:    { label: 'Q: Intro',       color: '#f472b6', bg: '#1a0a1a' },
  motivation:  { label: 'Q: Motivation',  color: '#fb923c', bg: '#1a0e00' },
  fit:         { label: 'Q: Fit',         color: '#22d3ee', bg: '#031a20' },
  future:      { label: 'Q: Future',      color: '#38bdf8', bg: '#031a2e' },
  strengths:   { label: 'Q: Strengths',   color: '#4ade80', bg: '#011a0a' },
  character:   { label: 'Q: Character',   color: '#e879f9', bg: '#1a0520' },
  weaknesses:  { label: 'Q: Weakness',    color: '#f87171', bg: '#1a0505' },
  situational: { label: 'Q: Situational', color: '#a3e635', bg: '#0f1a00' },
  behavioral:  { label: 'Q: Behavioral',  color: '#a78bfa', bg: '#1a0f33' },
  technical:   { label: 'Q: Technical',   color: '#60a5fa', bg: '#0a1525' },
  culture:     { label: 'Q: Culture',     color: '#34d399', bg: '#0a1f15' },
  values:      { label: 'Q: Values',      color: '#f0abfc', bg: '#1a0a1f' },
  salary:      { label: 'Q: Salary',      color: '#fbbf24', bg: '#1a1200' },
  closing:     { label: 'Q: Closing',     color: '#94a3b8', bg: '#0f172a' },
  general:     { label: 'Q: General',     color: '#64748b', bg: '#0d1117' },
};
