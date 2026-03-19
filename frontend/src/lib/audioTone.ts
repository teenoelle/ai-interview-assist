export type AudioFeatures = {
  dominantBand?: 'low' | 'mid' | 'high' | 'none';
  flux?: number; // 0-1, spectral change rate — proxy for animated vs monotone speech
};

/**
 * Analyze interviewer vocal tone from text + optional audio energy + spectral features.
 * energyLevel: RMS (Float32, typically 0.0–0.3; >0.07 = emphatic speech).
 * audioFeatures: from AnalyserNode — dominantBand + flux.
 */
export function analyzeAudioTone(
  text: string,
  energyLevel?: number,
  audioFeatures?: AudioFeatures,
): { emotion: string; reason: string } {
  const lower = text.toLowerCase();
  const excited  = ['excellent', 'impressive', 'love that', 'great answer', 'fantastic', 'brilliant', 'perfect'].some(w => lower.includes(w));
  const skeptical = ['however,', 'but ', "i'm not sure", "don't think", 'concern', 'challenging', 'struggle', 'worry'].some(w => lower.includes(w));
  const closing  = ['thank you for', "we'll be in touch", 'next steps', 'any questions for us', 'do you have any questions'].some(w => lower.includes(w));
  const curious  = (lower.includes('?') || ['interesting', 'tell me more', 'curious', 'explain'].some(w => lower.includes(w)));

  const energy   = energyLevel ?? 0;
  const emphatic = energy > 0.07;
  const subdued  = energy > 0.003 && energy < 0.018;

  const flux      = audioFeatures?.flux ?? 0;
  const animated  = flux > 0.35;  // rapid spectral changes → lively, expressive speech
  const monotone  = flux < 0.08 && energy > 0.01; // steady spectrum while speaking → flat delivery
  const highBand  = audioFeatures?.dominantBand === 'high'; // bright/energetic voice quality

  if (excited)   return { emotion: 'enthusiastic', reason: emphatic || animated ? 'positive language + high vocal energy' : 'positive affirming language' };
  if (skeptical) return { emotion: emphatic ? 'pressing' : 'skeptical', reason: emphatic ? 'challenging language delivered emphatically' : 'qualifying/hedging language' };
  if (closing)   return { emotion: 'wrapping up', reason: 'closing language detected' };
  if (curious)   return { emotion: (emphatic || animated || highBand) ? 'enthusiastic' : 'curious', reason: animated ? 'engaged questioning with animated delivery' : 'inquiry or question language' };

  // Audio-only signals when text is neutral
  if (emphatic || (animated && highBand)) return { emotion: 'engaged', reason: animated ? 'animated vocal delivery' : 'elevated vocal energy' };
  if (monotone)  return { emotion: 'reserved', reason: 'flat, low-animation delivery' };
  if (subdued)   return { emotion: 'reserved', reason: 'low vocal energy — may be thinking or cautious' };

  return { emotion: 'neutral', reason: 'neutral conversational tone' };
}
