// Ring buffer for PCM audio chunks
// PCM format: 16-bit signed LE, 16kHz, mono
// Samples per second = 16000, 2 bytes per sample
// 8 seconds max = 16000 * 8 * 2 = 256000 bytes
// 3 seconds min = 16000 * 3 * 2 = 96000 bytes

const SAMPLE_RATE: usize = 16000;
const BYTES_PER_SAMPLE: usize = 2;
const MIN_SEGMENT_BYTES: usize = SAMPLE_RATE * 3 * BYTES_PER_SAMPLE;  // 3 seconds
const MAX_SEGMENT_BYTES: usize = SAMPLE_RATE * 15 * BYTES_PER_SAMPLE; // 15 seconds
const SILENCE_THRESHOLD_BYTES: usize = SAMPLE_RATE * 1 * BYTES_PER_SAMPLE; // 1 second silence

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

        if energy > 500.0 {
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
