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
  asks: { topic: string; question: string; followUp: string }[];
  strategies: { keyword: string; text: string }[];
  // Introduction (Career Story framework)
  present: string;   // Summary: aggregate career statement
  thread: string;    // Thread: connecting throughline
  past: string;      // Story: insight-driven past moves
  future: string;    // Next: where you're headed
  // Motivation
  company: string;
  role: string;
  self: string;
  // Future/growth
  direction: string;
  alignment: string;
  contribution: string;
  // Transitions (between narrative sections)
  transition1: string;
  transition2: string;
  transition3: string;
}

export function parseSuggestion(text: string): ParsedSuggestion {
  const lines = text.split('\n');
  let acknowledge = '', solve = '', bridge = '', close = '', affirm = '', tell = '', cue = 'Answer';
  let present = '', thread = '', past = '', future = '';
  let company = '', role = '', self = '';
  let direction = '', alignment = '', contribution = '';
  let transition1 = '', transition2 = '', transition3 = '';
  const asks: { topic: string; question: string }[] = [];
  const bodyLines: string[] = [];
  let pendingAskTopic = '';
  let pendingTell = false; // true when Answer: was seen but had no inline text
  let pendingNewField: string | null = null; // tracks new-type field when LLM puts content on next line
  // Strip markdown bold markers e.g. **Affirm:** → Affirm:
  const clean = (s: string) => s.replace(/^\*+([^*]+)\*+\s*/, '$1 ').trim();
  const isCueLabel = (s: string) =>
    /^(Principle|Context|Action|Result|Point|Metric|General|Example|Story|Pivot|Acknowledge|Affirm|Solve|Bridge|Close|Answer|Say|Tell|Ask|Present|Summary|Thread|Past|Story|Future|Next|Company|Role|Self|Direction|Alignment|Contribution|Transition1|Transition2|Transition3):/i.test(s);

  for (const line of lines) {
    const t = line.trim();
    const c = clean(t);
    if (c === '---') { pendingTell = false; pendingNewField = null; continue; }

    // Helper to set a new-type field with multi-line fallback
    const setNF = (field: string, val: string) => {
      pendingTell = false; pendingAskTopic = '';
      pendingNewField = val ? null : field;
      if (val) {
        if (field === 'present') present = val;
        else if (field === 'thread') thread = val;
        else if (field === 'past') past = val;
        else if (field === 'future') future = val;
        else if (field === 'company') company = val;
        else if (field === 'role') role = val;
        else if (field === 'self') self = val;
        else if (field === 'direction') direction = val;
        else if (field === 'alignment') alignment = val;
        else if (field === 'contribution') contribution = val;
        else if (field === 'transition1') transition1 = val;
        else if (field === 'transition2') transition2 = val;
        else if (field === 'transition3') transition3 = val;
      }
    };

    if (c.match(/^Acknowledge:/i)) {
      pendingTell = false; pendingNewField = null;
      acknowledge = c.replace(/^Acknowledge:\s*/i, '').trim();
      pendingAskTopic = '';
    } else if (c.match(/^Solve:/i)) {
      pendingTell = false; pendingNewField = null;
      solve = c.replace(/^Solve:\s*/i, '').trim();
      pendingAskTopic = '';
    } else if (c.match(/^Bridge:/i)) {
      pendingTell = false; pendingNewField = null;
      bridge = c.replace(/^Bridge:\s*/i, '').trim();
      pendingAskTopic = '';
    } else if (c.match(/^Close:/i)) {
      pendingTell = false; pendingNewField = null;
      close = c.replace(/^Close:\s*/i, '').trim();
      pendingAskTopic = '';
    } else if (c.match(/^Affirm:/i)) {
      pendingTell = false; pendingNewField = null;
      affirm = c.replace(/^Affirm:\s*/i, '').trim();
      pendingAskTopic = '';
    } else if (c.match(/^(Present|Summary):/i)) {
      setNF('present', c.replace(/^(Present|Summary):\s*/i, '').trim());
    } else if (c.match(/^Thread:/i)) {
      setNF('thread', c.replace(/^Thread:\s*/i, '').trim());
    } else if (c.match(/^(Past|Story):/i) && !c.match(/^Story:\s*\[/i)) {
      // Story: as intro field (no bracket = not a body cue)
      setNF('past', c.replace(/^(Past|Story):\s*/i, '').trim());
    } else if (c.match(/^(Future|Next):/i)) {
      setNF('future', c.replace(/^(Future|Next):\s*/i, '').trim());
    } else if (c.match(/^Company:/i)) {
      setNF('company', c.replace(/^Company:\s*/i, '').trim());
    } else if (c.match(/^Role:/i)) {
      setNF('role', c.replace(/^Role:\s*/i, '').trim());
    } else if (c.match(/^Self:/i)) {
      setNF('self', c.replace(/^Self:\s*/i, '').trim());
    } else if (c.match(/^Direction:/i)) {
      setNF('direction', c.replace(/^Direction:\s*/i, '').trim());
    } else if (c.match(/^Alignment:/i)) {
      setNF('alignment', c.replace(/^Alignment:\s*/i, '').trim());
    } else if (c.match(/^Contribution:/i)) {
      setNF('contribution', c.replace(/^Contribution:\s*/i, '').trim());
    } else if (c.match(/^Transition1:/i)) {
      setNF('transition1', c.replace(/^Transition1:\s*/i, '').trim());
    } else if (c.match(/^Transition2:/i)) {
      setNF('transition2', c.replace(/^Transition2:\s*/i, '').trim());
    } else if (c.match(/^Transition3:/i)) {
      setNF('transition3', c.replace(/^Transition3:\s*/i, '').trim());
    } else if (c.match(/^(Answer|Say|Tell):/i)) {
      pendingNewField = null;
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
      pendingTell = false; pendingNewField = null;
      const raw = c.replace(/^Ask:\s*/i, '').trim();
      const parts = raw.split(/\s*\|\s*/);
      if (parts.length >= 2) {
        const topic = parts[0]?.trim() ?? '';
        const question = parts[1]?.trim() ?? topic;
        const followUp = parts[2]?.trim() ?? '';
        if (topic) asks.push({ topic, question, followUp });
      } else if (raw) {
        const words = raw.replace(/[?!.]$/, '').split(/\s+/);
        const topic = words.slice(0, Math.min(3, words.length)).join(' ');
        asks.push({ topic, question: raw, followUp: '' });
      }
      pendingAskTopic = '';
    } else if (c.match(/^(Principle|Context|Action|Result|Point|Metric|General|Example|Story|Pivot):\s*.+/i)) {
      pendingTell = false; pendingNewField = null;
      bodyLines.push(line);
      pendingAskTopic = '';
    } else if (pendingNewField && t && !isCueLabel(c)) {
      // Accumulate multi-line content into the pending new-type field
      const f = pendingNewField;
      if (f === 'present') present = present ? present + ' ' + t : t;
      else if (f === 'thread') thread = thread ? thread + ' ' + t : t;
      else if (f === 'past') past = past ? past + ' ' + t : t;
      else if (f === 'future') future = future ? future + ' ' + t : t;
      else if (f === 'company') company = company ? company + ' ' + t : t;
      else if (f === 'role') role = role ? role + ' ' + t : t;
      else if (f === 'self') self = self ? self + ' ' + t : t;
      else if (f === 'direction') direction = direction ? direction + ' ' + t : t;
      else if (f === 'alignment') alignment = alignment ? alignment + ' ' + t : t;
      else if (f === 'contribution') contribution = contribution ? contribution + ' ' + t : t;
      else if (f === 'transition1') transition1 = transition1 ? transition1 + ' ' + t : t;
      else if (f === 'transition2') transition2 = transition2 ? transition2 + ' ' + t : t;
      else if (f === 'transition3') transition3 = transition3 ? transition3 + ' ' + t : t;
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
  // Skip if we already have typed sections or ask entries (e.g. Closing type)
  if (!acknowledge && !tell && !present && !company && !direction && asks.length === 0) {
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

  if (!tell && text && asks.length === 0 && !present && !company && !direction) {
    const first = text.replace(/^(Acknowledge:|Answer:|Say:)[^\n]*/im, '').trim().split(/[.\n]/)[0]?.trim() ?? '';
    tell = first.length > 80 ? first.slice(0, 80) + '…' : first;
  }

  // Parse strategies from tell by splitting on embedded [keyword] markers
  const strategies: { keyword: string; text: string }[] = [];
  if (tell) {
    const parts = tell.split(/(?=\[[^\]]+\])/);
    for (const part of parts) {
      const m = part.match(/^\[([^\]]+)\]\s*/);
      if (m) {
        strategies.push({ keyword: m[1], text: part.slice(m[0].length).trim() });
      } else if (part.trim()) {
        strategies.push({ keyword: '', text: part.trim() });
      }
    }
  }

  return { acknowledge, solve, bridge, close, affirm, cue, tell, body, cues: [], asks, strategies,
    present, thread, past, future, company, role, self, direction, alignment, contribution,
    transition1, transition2, transition3 };
}

export function getAnswerType(
  parsed: ReturnType<typeof parseSuggestion>,
  tag?: string,
): { framework: string; label: string } {
  // Tag-specific overrides for types that share STAR fields but have distinct coaching frames
  if (tag === 'weaknesses')  return { framework: 'Weakness',    label: 'Real → Growth → Evidence → Redirect' };
  if (tag === 'situational') return { framework: 'Situational', label: 'Stakes → Approach → Reasoning → Close' };
  if (tag === 'strengths')   return { framework: 'Strengths',   label: 'Acknowledge → Strengths → Close' };

  if (parsed.present || parsed.thread || parsed.past || parsed.future)
    return { framework: 'Intro', label: 'Summary → Story → Next' };
  if (parsed.company || parsed.role || parsed.self)
    return { framework: 'Motivation', label: 'Company → Role → Self' };
  if (parsed.direction || parsed.alignment || parsed.contribution)
    return { framework: 'Future', label: 'Direction → Alignment → Contribution' };
  if (parsed.asks.length >= 3 && !parsed.acknowledge && !parsed.tell)
    return { framework: 'Closing', label: 'Questions to Ask' };
  if (parsed.tell || parsed.acknowledge)
    return { framework: 'STAR', label: 'Acknowledge → Answer → Close' };
  return { framework: '', label: '' };
}

// Strip any leading [...] prefix from display text
function stripTypePrefix(text: string): string {
  return text.replace(/^\[[^\]]+\]\s*/i, '').trim();
}

// Extract any leading [...] as a type/keyword tag
function extractTypeTag(text: string): string {
  const m = text.match(/^\[([^\]]+)\]/);
  return m ? m[1] : '';
}

export function parseCues(body: string): { label: string; text: string; typeTag: string; title: string }[] {
  return body.split('\n')
    .map(l => l.trim())
    .filter(l => l.match(/^(Principle|Context|Action|Result|Point|Metric|General|Example|Story|Pivot):\s*.+/i))
    .map(l => {
      const m = l.match(/^(\w+):\s*(.+)/i)!;
      const raw = m[2].trim();
      const typeTag = extractTypeTag(raw);
      const stripped = stripTypePrefix(raw);
      const sepIdx = stripped.indexOf(' | ');
      const title = sepIdx !== -1 ? stripped.slice(0, sepIdx).trim() : '';
      const text = sepIdx !== -1 ? stripped.slice(sepIdx + 3).trim() : stripped;
      return { label: m[1], text, typeTag, title };
    });
}
