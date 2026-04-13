export interface FillerCount {
  word: string;
  count: number;
}

// Each entry: display label + regex that matches all variants of that filler.
// "like" uses lookbehind/lookahead to exclude verb use ("I like", "would like")
// and comparison use ("like a", "like the", "like my", ...).
const FILLER_PATTERNS: { word: string; re: RegExp }[] = [
  { word: 'um',       re: /\buh?mm*h?\b/gi },
  { word: 'uh',       re: /\buh+m?\b/gi },
  { word: 'eh',       re: /\beh+\b/gi },
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

// Hedging phrases — weaken answers by signalling uncertainty.
const HEDGE_PATTERNS: { word: string; re: RegExp }[] = [
  { word: 'i think',   re: /\bi think\b/gi },
  { word: 'i guess',   re: /\bi guess\b/gi },
  { word: 'i suppose', re: /\bi suppose\b/gi },
  { word: 'maybe',     re: /\bmaybe\b/gi },
  { word: 'probably',  re: /\bprobably\b/gi },
  { word: 'not sure',  re: /\bnot sure\b/gi },
  { word: "i'm not sure", re: /\bi(?:'m| am) not sure\b/gi },
  { word: 'if that makes sense', re: /\bif that makes sense\b/gi },
  { word: 'does that make sense', re: /\bdoes that make sense\b/gi },
  { word: 'something like that', re: /\bsomething like that\b/gi },
  { word: 'sort of',   re: /\bsort of\b/gi },
];

// Source strings — used to build fresh regex instances per call.
const FILLER_RE_SRC = FILLER_PATTERNS.map(p => p.re.source).join('|');
const HEDGE_RE_SRC  = HEDGE_PATTERNS.map(p => p.re.source).join('|');

// Always returns a fresh regex instance — never share a /g regex across calls.
export function fillerRe(): RegExp {
  return new RegExp(FILLER_RE_SRC, 'gi');
}

export function hedgeRe(): RegExp {
  return new RegExp(HEDGE_RE_SRC, 'gi');
}

export function countFillers(text: string): FillerCount[] {
  return FILLER_PATTERNS
    .map(({ word, re }) => {
      re.lastIndex = 0;
      return { word, count: (text.match(re) ?? []).length };
    })
    .filter(f => f.count > 0);
}

export function countHedges(text: string): FillerCount[] {
  return HEDGE_PATTERNS
    .map(({ word, re }) => {
      re.lastIndex = 0;
      return { word, count: (text.match(re) ?? []).length };
    })
    .filter(f => f.count > 0);
}

export function totalFillers(counts: FillerCount[]): number {
  return counts.reduce((s, f) => s + f.count, 0);
}
