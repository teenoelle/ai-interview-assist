export function computeConfidence(answer: string, suggestion: string): { score: number; matched: string[]; missed: string[] } {
  const stop = new Set(['about','after','also','another','any','are','been','before','being','both','but','can','could','did','does','doing','down','each','even','every','for','from','get','got','has','have','here','how','its','just','like','made','make','more','most','much','need','never','new','now','only','other','our','out','over','same','should','since','some','such','than','that','the','their','them','then','there','these','they','this','those','through','time','too','under','very','was','were','what','when','where','which','while','who','will','with','would','your']);
  const toWords = (t: string) => [...new Set(t.toLowerCase().split(/\W+/).filter(w => w.length > 3 && !stop.has(w)))];
  const ansWords = new Set(toWords(answer));
  const suggWords = toWords(suggestion);
  if (suggWords.length === 0) return { score: 0, matched: [], missed: [] };
  const matched = suggWords.filter(w => ansWords.has(w));
  const missed = suggWords.filter(w => !ansWords.has(w)).slice(0, 6);
  return { score: Math.min(100, Math.round((matched.length / Math.min(suggWords.length, 10)) * 100)), matched, missed };
}
