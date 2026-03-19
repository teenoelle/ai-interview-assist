/**
 * Analyze interviewer vocal tone from transcribed text + optional audio energy level.
 * energyLevel: RMS from AudioWorklet (Float32, typically 0.0–0.3; >0.07 = emphatic speech).
 */
export function analyzeAudioTone(text: string, energyLevel?: number): { emotion: string; reason: string } {
  const lower = text.toLowerCase();
  const excited = ['excellent', 'impressive', 'love that', 'great answer', 'fantastic', 'brilliant', 'perfect'].some(w => lower.includes(w));
  const skeptical = ['however,', 'but ', "i'm not sure", "don't think", 'concern', 'challenging', 'struggle', 'worry'].some(w => lower.includes(w));
  const closing = ['thank you for', "we'll be in touch", 'next steps', 'any questions for us', 'do you have any questions'].some(w => lower.includes(w));
  const curious = (lower.includes('?') || ['interesting', 'tell me more', 'curious', 'explain'].some(w => lower.includes(w)));

  const energy = energyLevel ?? 0;
  const emphatic = energy > 0.07;   // noticeably louder/more forceful than normal speech
  const subdued  = energy > 0.003 && energy < 0.018; // speech present but quiet/hesitant

  if (excited) return { emotion: 'enthusiastic', reason: emphatic ? 'positive language + high vocal energy' : 'positive affirming language' };
  if (skeptical) return { emotion: emphatic ? 'pressing' : 'skeptical', reason: emphatic ? 'challenging language delivered emphatically' : 'qualifying/hedging language' };
  if (closing) return { emotion: 'wrapping up', reason: 'closing language detected' };
  if (curious) return { emotion: emphatic ? 'enthusiastic' : 'curious', reason: emphatic ? 'engaged questioning with high energy' : 'inquiry or question language' };

  // Audio-only signals when text is neutral
  if (emphatic) return { emotion: 'engaged', reason: 'elevated vocal energy' };
  if (subdued)  return { emotion: 'reserved', reason: 'low vocal energy — may be thinking or cautious' };

  return { emotion: 'neutral', reason: 'neutral conversational tone' };
}
