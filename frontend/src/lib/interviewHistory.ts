export interface InterviewRecord {
  id: string;
  date: string;
  summary: string;
  strong_points: string[];
  improvement_areas: string[];
  rehearsal_questions: string[];
}

const KEY = 'interview-history';

export function saveInterview(data: Omit<InterviewRecord, 'id' | 'date'>): void {
  const history = loadHistory();
  const record: InterviewRecord = { id: Date.now().toString(), date: new Date().toLocaleString(), ...data };
  history.unshift(record);
  if (history.length > 20) history.splice(20);
  localStorage.setItem(KEY, JSON.stringify(history));
}

export function loadHistory(): InterviewRecord[] {
  try { return JSON.parse(localStorage.getItem(KEY) ?? '[]'); } catch { return []; }
}

export function deleteRecord(id: string): void {
  localStorage.setItem(KEY, JSON.stringify(loadHistory().filter(r => r.id !== id)));
}
