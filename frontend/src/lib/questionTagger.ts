export type QuestionTag = 'behavioral' | 'technical' | 'culture' | 'salary' | 'closing' | 'general';

const BEHAVIORAL = ['tell me about a time', 'describe a situation', 'give me an example', 'walk me through', 'have you ever', 'what was a time', 'share an experience', 'when have you', 'describe when', 'describe a time', 'can you give'];
const TECHNICAL = ['how would you build', 'how do you', 'design a', 'implement', 'algorithm', 'complexity', 'debug', 'architecture', 'system design', 'code', 'technical', 'framework', 'database', 'api', 'performance', 'scale', 'optimize'];
const SALARY = ['salary', 'compensation', 'pay', 'benefits', 'equity', 'stock', 'bonus', 'expectations', 'offer', 'package', 'rate'];
const CULTURE = ['culture', 'values', 'team', 'environment', 'work style', 'remote', 'work-life', 'management style', 'mission', 'motivate', 'what drives you'];
const CLOSING = ['any questions', 'questions for us', 'next steps', 'timeline', 'hear from us', 'back to you', 'anything else', 'is there anything'];

export function tagQuestion(q: string): QuestionTag {
  const lower = q.toLowerCase();
  if (BEHAVIORAL.some(t => lower.includes(t))) return 'behavioral';
  if (SALARY.some(t => lower.includes(t))) return 'salary';
  if (CLOSING.some(t => lower.includes(t))) return 'closing';
  if (CULTURE.some(t => lower.includes(t))) return 'culture';
  if (TECHNICAL.some(t => lower.includes(t))) return 'technical';
  return 'general';
}

export const TAG_CONFIG: Record<QuestionTag, { label: string; color: string; bg: string }> = {
  behavioral: { label: 'Story',     color: '#a78bfa', bg: '#1a0f33' },
  technical:  { label: 'Technical', color: '#60a5fa', bg: '#0a1525' },
  culture:    { label: 'Culture',   color: '#34d399', bg: '#0a1f15' },
  salary:     { label: 'Salary',    color: '#fbbf24', bg: '#1a1200' },
  closing:    { label: 'Closing',   color: '#94a3b8', bg: '#0f172a' },
  general:    { label: 'General',   color: '#64748b', bg: '#0d1117' },
};
