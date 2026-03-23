export interface ParsedSuggestion {
  acknowledge: string;
  solve: string;
  bridge: string;
  close: string;
  affirm: string;
  cue: string;     // 'Answer'
  tell: string;    // main spoken line
  body: string;    // raw body text (cue lines after Answer)
  cues: string[];  // unused — kept for compat
  asks: { topic: string; question: string }[];
}

export function parseSuggestion(text: string): ParsedSuggestion {
  const lines = text.split('\n');
  let acknowledge = '', solve = '', bridge = '', close = '', affirm = '', tell = '', cue = 'Answer';
  const asks: { topic: string; question: string }[] = [];
  const bodyLines: string[] = [];
  let pendingAskTopic = '';
  let pendingTell = false; // true when Answer: was seen but had no inline text
  // Strip markdown bold markers e.g. **Affirm:** → Affirm:
  const clean = (s: string) => s.replace(/^\*+([^*]+)\*+\s*/, '$1 ').trim();
  const isCueLabel = (s: string) =>
    /^(Principle|Context|Action|Result|Point|Metric|General|Example|Story|Acknowledge|Affirm|Solve|Bridge|Close|Answer|Say|Tell|Ask):/i.test(s);

  for (const line of lines) {
    const t = line.trim();
    const c = clean(t);
    if (c === '---') { pendingTell = false; continue; }

    if (c.match(/^Acknowledge:/i)) {
      pendingTell = false;
      acknowledge = c.replace(/^Acknowledge:\s*/i, '').trim();
      pendingAskTopic = '';
    } else if (c.match(/^Solve:/i)) {
      pendingTell = false;
      solve = c.replace(/^Solve:\s*/i, '').trim();
      pendingAskTopic = '';
    } else if (c.match(/^Bridge:/i)) {
      pendingTell = false;
      bridge = c.replace(/^Bridge:\s*/i, '').trim();
      pendingAskTopic = '';
    } else if (c.match(/^Close:/i)) {
      pendingTell = false;
      close = c.replace(/^Close:\s*/i, '').trim();
      pendingAskTopic = '';
    } else if (c.match(/^Affirm:/i)) {
      pendingTell = false;
      affirm = c.replace(/^Affirm:\s*/i, '').trim();
      pendingAskTopic = '';
    } else if (c.match(/^(Answer|Say|Tell):/i)) {
      cue = 'Answer';
      const inline = c.replace(/^(Answer|Say|Tell):\s*/i, '').trim();
      if (inline) {
        tell = inline;
        pendingTell = false;
      } else {
        pendingTell = true; // answer text is on the next line
      }
      pendingAskTopic = '';
    } else if (c.match(/^Ask:/i)) {
      pendingTell = false;
      const raw = c.replace(/^Ask:\s*/i, '').trim();
      const parts = raw.split(/\s*\|\s*/);
      if (parts.length >= 2) {
        const topic = parts[0]?.trim() ?? '';
        const question = parts[1]?.trim() ?? topic;
        if (topic) asks.push({ topic, question });
      } else if (raw) {
        const words = raw.replace(/[?!.]$/, '').split(/\s+/);
        const topic = words.slice(0, Math.min(3, words.length)).join(' ');
        asks.push({ topic, question: raw });
      }
      pendingAskTopic = '';
    } else if (c.match(/^(Principle|Context|Action|Result|Point|Metric|General|Example|Story):\s*.+/i)) {
      pendingTell = false;
      bodyLines.push(line);
      pendingAskTopic = '';
    } else if (pendingTell && t && !isCueLabel(c)) {
      // Capture answer text that was on its own line after Answer:
      tell = tell ? tell + ' ' + t : t;
      // Don't reset pendingTell — allow multi-line answer accumulation
    } else if (pendingAskTopic && t && !isCueLabel(c)) {
      asks.push({ topic: pendingAskTopic, question: t });
      pendingAskTopic = '';
    } else if (t && !isCueLabel(c)) {
      pendingTell = false;
    }
  }

  let body = bodyLines.join('\n').trim();

  // Positional fallback: if the model dropped all labels
  if (!acknowledge && !tell) {
    const nonEmpty = lines.map(l => l.trim()).filter(l => l && l !== '---');
    if (nonEmpty.length >= 2) {
      acknowledge = clean(nonEmpty[0]);
      tell = clean(nonEmpty[1]);
      cue = 'Answer';
      const fallbackBodyLines: string[] = [];
      const cueLabels = ['General', 'General', 'Example', 'Example'];
      let cueCount = 0;
      for (const line of nonEmpty.slice(2)) {
        const hasPipe = line.includes('|');
        if (hasPipe) {
          const parts = line.split(/\s*\|\s*/);
          const topic = parts[0]?.trim() ?? '';
          const question = parts[1]?.trim() ?? topic;
          if (topic) asks.push({ topic, question });
        } else {
          const label = cueLabels[cueCount] ?? 'General';
          fallbackBodyLines.push(`${label}: ${line}`);
          cueCount++;
        }
      }
      body = fallbackBodyLines.join('\n').trim();
    }
  }

  if (!tell && text) {
    const first = text.replace(/^(Acknowledge:|Answer:|Say:)[^\n]*/im, '').trim().split(/[.\n]/)[0]?.trim() ?? '';
    tell = first.length > 80 ? first.slice(0, 80) + '…' : first;
  }

  return { acknowledge, solve, bridge, close, affirm, cue, tell, body, cues: [], asks };
}

// Strip [General] / [Example] prefix from display text
function stripTypePrefix(text: string): string {
  return text.replace(/^\[(General Answer|General|Example|Story)\]\s*/i, '').trim();
}

// Extract [General] / [Example] type tag for the label badge
function extractTypeTag(text: string): string {
  const m = text.match(/^\[(General Answer|General|Example|Story)\]/i);
  return m ? m[1] : '';
}

export function parseCues(body: string): { label: string; text: string; typeTag: string }[] {
  return body.split('\n')
    .map(l => l.trim())
    .filter(l => l.match(/^(Principle|Context|Action|Result|Point|Metric|General|Example|Story):\s*.+/i))
    .map(l => {
      const m = l.match(/^(\w+):\s*(.+)/i)!;
      const raw = m[2].trim();
      return {
        label: m[1],
        text: stripTypePrefix(raw),
        typeTag: extractTypeTag(raw),
      };
    });
}
