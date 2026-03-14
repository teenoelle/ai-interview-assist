export interface Story {
  id: string;
  title: string;          // short name, e.g. "Led team through rebrand"
  situation: string;      // S: context
  task: string;           // T: what you owned
  action: string;         // A: what you did
  result: string;         // R: outcome + metric
  tags: string[];         // keywords: "leadership", "conflict", "data"
}

const KEY = 'story-bank';

export function loadStories(): Story[] {
  try { return JSON.parse(localStorage.getItem(KEY) ?? '[]'); } catch { return []; }
}

export function saveStory(story: Omit<Story, 'id'>): Story {
  const stories = loadStories();
  const record: Story = { id: Date.now().toString(), ...story };
  stories.unshift(record);
  localStorage.setItem(KEY, JSON.stringify(stories));
  return record;
}

export function updateStory(story: Story): void {
  const stories = loadStories().map(s => s.id === story.id ? story : s);
  localStorage.setItem(KEY, JSON.stringify(stories));
}

export function deleteStory(id: string): void {
  localStorage.setItem(KEY, JSON.stringify(loadStories().filter(s => s.id !== id)));
}

// Match a question to relevant stories by keyword overlap
export function matchStories(question: string, stories: Story[]): Story[] {
  const q = question.toLowerCase();
  const scored = stories.map(story => {
    let score = 0;
    const text = `${story.title} ${story.tags.join(' ')} ${story.situation} ${story.task} ${story.action} ${story.result}`.toLowerCase();
    // Direct keyword match from question words
    const qWords = q.split(/\W+/).filter(w => w.length > 4);
    for (const w of qWords) {
      if (text.includes(w)) score += 2;
    }
    // Tag exact match
    for (const tag of story.tags) {
      if (q.includes(tag.toLowerCase())) score += 3;
    }
    // Behavioral signals
    const behavioral = ['time', 'situation', 'example', 'describe', 'tell me about', 'experience', 'challenge', 'conflict', 'lead', 'fail', 'success'];
    for (const b of behavioral) {
      if (q.includes(b) && score > 0) score += 1;
    }
    return { story, score };
  });
  return scored.filter(s => s.score > 0).sort((a, b) => b.score - a.score).slice(0, 2).map(s => s.story);
}
