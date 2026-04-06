export interface FillerCount {
  word: string;
  count: number;
}

// Each entry: display label + regex that matches all variants of that filler.
// "like" uses lookbehind/lookahead to exclude verb use ("I like", "would like")
// and comparison use ("like a", "like the", "like my", ...).
const FILLER_PATTERNS: { word: string; re: RegExp }[] = [
  { word: 'um',       re: /\bum+h?\b/gi },
  { word: 'uh',       re: /\buh+\b/gi },
  { word: 'er',       re: /\ber+\b/gi },
  { word: 'hmm',      re: /\bhmm+\b/gi },
  { word: 'you know', re: /\byou know\b/gi },
  { word: 'i mean',   re: /\bi mean\b/gi },
  { word: 'kind of',  re: /\bkind of\b/gi },
  { word: 'sort of',  re: /\bsort of\b/gi },
  { word: 'basically',re: /\bbasically\b/gi },
  { word: 'literally',re: /\bliterally\b/gi },
  {
    word: 'like',
    // Exclude verb use: "I like", "you like", "would like", "don't like", etc.
    // Exclude comparison/simile: "like a", "like the", "like my", "like this", "like when", etc.
    re: /(?<!\b(?:i|you|he|she|it|we|they|would|could|wouldn't|couldn't|don't|doesn't|didn't|do|does|did|'d) )\blike\b(?!\s+(?:a\b|an\b|the\b|my\b|your\b|his\b|her\b|its\b|our\b|their\b|this\b|that\b|these\b|those\b|to\b|when\b|what\b|how\b|whom\b|which\b|if\b))/gi,
  },
];

// Source string for filler regex — used to create fresh instances to avoid
// shared lastIndex state bugs with the g flag.
const FILLER_RE_SRC = FILLER_PATTERNS.map(p => p.re.source).join('|');

// Always returns a fresh regex instance — never share a /g regex across calls.
export function fillerRe(): RegExp {
  return new RegExp(FILLER_RE_SRC, 'gi');
}

export function countFillers(text: string): FillerCount[] {
  return FILLER_PATTERNS
    .map(({ word, re }) => {
      re.lastIndex = 0;
      return { word, count: (text.match(re) ?? []).length };
    })
    .filter(f => f.count > 0);
}

export function totalFillers(counts: FillerCount[]): number {
  return counts.reduce((s, f) => s + f.count, 0);
}
