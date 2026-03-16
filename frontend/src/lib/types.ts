export type WsEvent =
  | { type: 'transcript'; text: string; timestamp_ms: number; speaker: string }
  | { type: 'sentiment'; emotion: string; reason?: string; coaching?: string; coaching_why?: string }
  | { type: 'question_detected'; question: string }
  | { type: 'suggestion_token'; token: string }
  | { type: 'suggestion_complete'; full_text: string }
  | { type: 'error'; message: string }
  | { type: 'status'; message: string }
  | { type: 'rate_limit'; provider: string; requests_remaining: number; requests_limit: number };

export type Emotion = 'engaged' | 'curious' | 'neutral' | 'skeptical' | 'confused' | 'bored' | 'pleased';

export type QuestionTag = 'behavioral' | 'technical' | 'culture' | 'salary' | 'closing' | 'general';

export interface RedFlag {
  category: string;
  coachingNote: string;
}

export interface AnswerFeedback {
  coaching: string;
  missed_followup: boolean;
  missed_metric: boolean;
}

export interface VocalSentiment {
  tone: string;
  pace: string;
  confidence_score: number;
  coaching: string;
  fillers_noted: string;
}

export interface TranscriptEntry {
  text: string;
  timestamp_ms: number;
  speaker: string;
}

export interface SuggestionEntry {
  question: string;
  suggestion: string;
  streaming: boolean;
  tag?: QuestionTag;
  redFlag?: RedFlag;
  matchedStories?: { id: string; title: string; result: string }[];
  answerFeedback?: AnswerFeedback;
  vocalFeedback?: VocalSentiment;
  confidenceScore?: number;
  matchedKeywords?: string[];
  missedKeywords?: string[];
  answered?: boolean;
}
