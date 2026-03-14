export type WsEvent =
  | { type: 'transcript'; text: string; timestamp_ms: number; speaker: string }
  | { type: 'sentiment'; emotion: string; reason?: string; coaching?: string; coaching_why?: string }
  | { type: 'question_detected'; question: string }
  | { type: 'suggestion_token'; token: string }
  | { type: 'suggestion_complete'; full_text: string }
  | { type: 'error'; message: string }
  | { type: 'status'; message: string }
  | { type: 'rate_limit'; provider: string; requests_remaining: number; requests_limit: number }
  | { type: 'followup_questions'; questions: string[] }
  | { type: 'answer_feedback'; feedback: string; had_metric: boolean; answered: boolean; concise: boolean }
  | { type: 'confidence_score'; score: number; matched_keywords: string[]; missing_keywords: string[] };

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
  followupQuestions?: string[];
  answerFeedback?: AnswerFeedback;
  confidenceScore?: number;
  matchedKeywords?: string[];
}
