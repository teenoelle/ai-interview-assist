export function analyzeAudioTone(text: string): { emotion: string; reason: string } {
  const lower = text.toLowerCase();
  const excited = ['excellent', 'impressive', 'love that', 'great answer', 'fantastic', 'brilliant', 'perfect'].some(w => lower.includes(w));
  const skeptical = ['however,', 'but ', "i'm not sure", "don't think", 'concern', 'challenging', 'struggle', 'worry'].some(w => lower.includes(w));
  const closing = ['thank you for', "we'll be in touch", 'next steps', 'any questions for us', 'do you have any questions'].some(w => lower.includes(w));
  const curious = (lower.includes('?') || ['interesting', 'tell me more', 'curious', 'explain'].some(w => lower.includes(w)));
  if (excited) return { emotion: 'enthusiastic', reason: 'positive affirming language' };
  if (skeptical) return { emotion: 'skeptical', reason: 'qualifying/hedging language' };
  if (closing) return { emotion: 'wrapping up', reason: 'closing language detected' };
  if (curious) return { emotion: 'curious', reason: 'inquiry or question language' };
  return { emotion: 'neutral', reason: 'neutral conversational tone' };
}
