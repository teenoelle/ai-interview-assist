import type { DebriefResult } from './types';

export interface SavedQaEntry {
  question: string;
  suggestion: string;
  confidenceScore?: number;
  coaching?: string;
  tag?: string;
  candidateAnswer?: string;       // what the candidate actually said
  vocalScore?: number;            // from practice recording
  vocalCoaching?: string;         // from practice recording
  vocalTone?: string;
  vocalPace?: string;
  vocalFillers?: string;
}

export interface InterviewRecord {
  id: string;
  date: string;
  company?: string;
  role?: string;
  summary: string;
  strong_points: string[];
  improvement_areas: string[];
  rehearsal_questions: string[];
  qa_entries?: SavedQaEntry[];
  transcript?: string;
  debrief_result?: DebriefResult;
}

const KEY = 'interview-history';

export function saveInterview(data: Omit<InterviewRecord, 'id' | 'date' | 'company' | 'role'>): void {
  const history = loadHistory();
  const company = localStorage.getItem('setup-company-name') ?? undefined;
  const role = localStorage.getItem('setup-role-name') ?? undefined;
  const record: InterviewRecord = { id: Date.now().toString(), date: new Date().toLocaleString(), company, role, ...data };
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
