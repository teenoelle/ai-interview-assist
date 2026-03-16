export interface ParsedSuggestion {
  acknowledge: string;
  affirm: string;
  cue: string;     // 'Answer' | 'Ask'
  tell: string;    // main spoken line
  body: string;    // raw body text after --- (joined lines)
  cues: string[];  // body lines as individual stripped strings
  asks: string[];
}

export function parseSuggestion(text: string): ParsedSuggestion {
  const lines = text.split('\n');
  let acknowledge = '', affirm = '', tell = '', cue = 'Answer';
  const asks: string[] = [], cues: string[] = [], bodyLines: string[] = [];
  let pastSeparator = false;
  // Strip markdown bold markers e.g. **Affirm:** → Affirm:
  const clean = (s: string) => s.replace(/^\*+([^*]+)\*+\s*/, '$1 ').trim();

  for (const line of lines) {
    const t = line.trim();
    const c = clean(t);
    if (c === '---') { pastSeparator = true; continue; }
    if (!pastSeparator) {
      if (c.match(/^Acknowledge:/i)) acknowledge = c.replace(/^Acknowledge:\s*/i, '').trim();
      else if (c.match(/^Affirm:/i)) affirm = c.replace(/^Affirm:\s*/i, '').trim();
      else if (c.match(/^(Answer|Say|Tell):/i)) { cue = 'Answer'; tell = c.replace(/^(Answer|Say|Tell):\s*/i, '').trim(); }
      else if (c.match(/^Ask:/i) && !tell) { cue = 'Ask'; tell = c.replace(/^Ask:\s*/i, '').trim(); }
    } else {
      if (c.match(/^Ask:/i)) {
        const a = c.replace(/^Ask:\s*/i, '').trim();
        if (a) asks.push(a);
      } else if (t) {
        bodyLines.push(line);
        const stripped = t.replace(/^[#\-*•]+\s*/, '').trim();
        if (stripped) cues.push(stripped);
      }
    }
  }

  let body = bodyLines.join('\n').trim();
  if (!tell && text) {
    const first = text.replace(/^(Acknowledge:|Affirm:|Answer:|Say:)[^\n]*/im, '').trim().split(/[.\n]/)[0]?.trim() ?? '';
    tell = first.length > 80 ? first.slice(0, 80) + '…' : first;
    body = text;
  }

  return { acknowledge, affirm, cue, tell, body, cues, asks };
}

export function parseCues(body: string): { label: string; text: string }[] {
  return body.split('\n')
    .map(l => l.trim())
    .filter(l => l.match(/^(Context|Action|Result|Point):\s*.+/i))
    .map(l => { const m = l.match(/^(\w+):\s*(.+)/i)!; return { label: m[1], text: m[2].trim() }; });
}
