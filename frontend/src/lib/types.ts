export type SuggestionMode = 'compound' | 'primary' | 'secondary';

export type WsEvent =
  | { type: 'transcript'; text: string; timestamp_ms: number; speaker: string }
  | { type: 'sentiment'; emotion: string; reason?: string; coaching?: string; coaching_why?: string }
  | { type: 'question_detected'; question: string; secondary_tag?: string }
  | { type: 'suggestion_token'; token: string; mode: SuggestionMode }
  | { type: 'suggestion_complete'; full_text: string; mode: SuggestionMode }
  | { type: 'error'; message: string }
  | { type: 'status'; message: string }
  | { type: 'rate_limit'; provider: string; requests_remaining: number; requests_limit: number }
  | { type: 'provider_used'; service: string; provider: string; local: boolean };

export type Emotion = 'engaged' | 'curious' | 'neutral' | 'skeptical' | 'confused' | 'bored' | 'pleased';

export type QuestionTag = 'behavioral' | 'technical' | 'culture' | 'salary' | 'closing' | 'personal' | 'motivation' | 'future' | 'strengths' | 'weaknesses' | 'situational' | 'general';

export interface DebriefResult {
  summary: string;
  strong_points: string[];
  improvement_areas: string[];
  followup_email: string[];
  followup_email_draft?: string;
}

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

export interface PracticeAnswer {
  question: string;
  answerText: string;
  score?: number;
  coaching?: string;
  strong?: string;
  vocalTone?: string;
  vocalConfidence?: number;
  recordedAt: number;
}

export interface SuggestionEntry {
  question: string;
  suggestion: string;
  streaming: boolean;
  source?: 'live' | 'simulated';
  tag?: QuestionTag;
  secondaryTag?: QuestionTag;
  // Compound mode slots (only present when secondaryTag is set)
  compoundSuggestion?: string;
  compoundStreaming?: boolean;
  secondarySuggestion?: string;
  secondaryStreaming?: boolean;
  // Closing question on-demand sections
  closingHR?: string;
  closingHRFetched?: boolean;
  closingHM?: string;
  closingHMFetched?: boolean;
  closingCEO?: string;
  closingCEOFetched?: boolean;
  redFlag?: RedFlag;
  answerFeedback?: AnswerFeedback;
  vocalFeedback?: VocalSentiment;
  confidenceScore?: number;
  matchedKeywords?: string[];
  missedKeywords?: string[];
  answered?: boolean;
  provider?: string;
  providerLocal?: boolean;
  detectedAt?: number;  // timestamp_ms when question was detected
}
