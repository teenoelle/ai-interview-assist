export type WsEvent =
  | { type: 'transcript'; text: string; timestamp_ms: number; speaker: string }
  | { type: 'sentiment'; emotion: string }
  | { type: 'question_detected'; question: string }
  | { type: 'suggestion_token'; token: string }
  | { type: 'suggestion_complete'; full_text: string }
  | { type: 'error'; message: string }
  | { type: 'status'; message: string };

export type Emotion = 'engaged' | 'curious' | 'neutral' | 'skeptical' | 'confused' | 'bored' | 'pleased';

export interface TranscriptEntry {
  text: string;
  timestamp_ms: number;
  speaker: string;  // "Interviewer" or "You"
}

export interface SuggestionEntry {
  question: string;
  suggestion: string;
  streaming: boolean;
}
