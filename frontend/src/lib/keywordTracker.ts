export interface KeywordStatus {
  keyword: string;
  mentioned: boolean;
  mentionCount: number;
}

const KEY = 'jd-keywords';

export function saveKeywords(keywords: string[]): void {
  localStorage.setItem(KEY, JSON.stringify(keywords));
}

export function loadKeywords(): string[] {
  try { return JSON.parse(localStorage.getItem(KEY) ?? '[]'); } catch { return []; }
}

export function checkMentioned(text: string, keywords: string[]): string[] {
  const lower = text.toLowerCase();
  return keywords.filter(k => {
    const kl = k.toLowerCase();
    return lower.includes(kl) || lower.includes(kl.replace(/\s+/g, '-'));
  });
}

export function buildKeywordStatus(keywords: string[], mentionedSet: Set<string>): KeywordStatus[] {
  return keywords.map(k => ({
    keyword: k,
    mentioned: mentionedSet.has(k),
    mentionCount: 0,
  }));
}
