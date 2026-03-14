// Ring buffer for PCM audio chunks
// PCM format: 16-bit signed LE, 16kHz, mono
// Samples per second = 16000, 2 bytes per sample
// 3 seconds min = 16000 * 3 * 2 = 96000 bytes  (reduced from 4s for lower latency)
// 15 seconds max = 16000 * 15 * 2 = 480000 bytes
// 0.5s silence = 16000 * 2 / 2 = 16000 bytes

const SAMPLE_RATE: usize = 16000;
const BYTES_PER_SAMPLE: usize = 2;
pub(crate) const MIN_SEGMENT_BYTES: usize = SAMPLE_RATE * 3 * BYTES_PER_SAMPLE;
pub(crate) const MAX_SEGMENT_BYTES: usize = SAMPLE_RATE * 15 * BYTES_PER_SAMPLE;
pub(crate) const SILENCE_THRESHOLD_BYTES: usize = SAMPLE_RATE * BYTES_PER_SAMPLE / 2;

pub struct RingBuffer {
    data: Vec<u8>,
    silent_bytes: usize,
    has_speech: bool,
}

impl RingBuffer {
    pub fn new() -> Self {
        Self {
            data: Vec::with_capacity(MAX_SEGMENT_BYTES),
            silent_bytes: 0,
            has_speech: false,
        }
    }

    pub fn push(&mut self, chunk: &[u8]) {
        // Simple energy check: compute mean absolute amplitude of i16 samples
        let num_samples = chunk.len() / 2;
        let energy = if num_samples > 0 {
            let sum: f32 = chunk
                .chunks_exact(2)
                .map(|b| {
                    let sample = i16::from_le_bytes([b[0], b[1]]);
                    (sample as f32).abs()
                })
                .sum();
            sum / num_samples as f32
        } else {
            0.0
        };

        if energy > 200.0 {
            self.has_speech = true;
            self.silent_bytes = 0;
        } else {
            self.silent_bytes += chunk.len();
        }

        self.data.extend_from_slice(chunk);
    }

    pub fn should_flush(&self) -> bool {
        if self.data.len() >= MAX_SEGMENT_BYTES {
            return true;
        }
        if self.has_speech
            && self.data.len() >= MIN_SEGMENT_BYTES
            && self.silent_bytes >= SILENCE_THRESHOLD_BYTES
        {
            return true;
        }
        false
    }

    pub fn drain_segment(&mut self) -> Vec<u8> {
        let data = std::mem::take(&mut self.data);
        self.silent_bytes = 0;
        self.has_speech = false;
        data
    }
}

impl Default for RingBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn silent_chunk(bytes: usize) -> Vec<u8> {
        vec![0u8; bytes] // zero amplitude = silence
    }

    fn loud_chunk(bytes: usize) -> Vec<u8> {
        // i16 value 10000 → energy ~10000, well above 500 threshold
        let sample: i16 = 10000;
        let [lo, hi] = sample.to_le_bytes();
        (0..bytes / 2).flat_map(|_| [lo, hi]).collect()
    }

    #[test]
    fn does_not_flush_empty_buffer() {
        let buf = RingBuffer::new();
        assert!(!buf.should_flush());
    }

    #[test]
    fn does_not_flush_short_silent_buffer() {
        let mut buf = RingBuffer::new();
        buf.push(&silent_chunk(1000));
        assert!(!buf.should_flush());
    }

    #[test]
    fn flushes_at_max_size() {
        let mut buf = RingBuffer::new();
        buf.push(&loud_chunk(MAX_SEGMENT_BYTES));
        assert!(buf.should_flush());
    }

    #[test]
    fn flushes_after_speech_then_silence() {
        let mut buf = RingBuffer::new();
        // Push enough speech to exceed min segment
        buf.push(&loud_chunk(MIN_SEGMENT_BYTES + 1000));
        assert!(!buf.should_flush()); // no silence yet
        // Push 2s of silence
        buf.push(&silent_chunk(SILENCE_THRESHOLD_BYTES));
        assert!(buf.should_flush());
    }

    #[test]
    fn does_not_flush_min_size_with_silence_but_no_speech() {
        let mut buf = RingBuffer::new();
        buf.push(&silent_chunk(MIN_SEGMENT_BYTES + SILENCE_THRESHOLD_BYTES));
        // has_speech is false → should not flush despite size
        assert!(!buf.should_flush());
    }

    #[test]
    fn drain_resets_buffer() {
        let mut buf = RingBuffer::new();
        buf.push(&loud_chunk(1000));
        let drained = buf.drain_segment();
        assert_eq!(drained.len(), 1000);
        assert!(!buf.should_flush());
        assert!(buf.data.is_empty());
    }
}
