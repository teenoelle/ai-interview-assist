use anyhow::Result;
use hound::{SampleFormat, WavSpec, WavWriter};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// ── HTTP client ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Segment {
    pub speaker: String,
    pub start: f64,
    pub end: f64,
}

/// Send WAV bytes to the diarization sidecar and return speaker segments.
/// Returns Err on network failure or non-2xx response.
pub async fn diarize(url: &str, wav_bytes: Vec<u8>) -> Result<Vec<Segment>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60)) // CPU diarization is slow
        .build()?;

    let resp = client
        .post(format!("{}/diarize", url.trim_end_matches('/')))
        .header("content-type", "audio/wav")
        .body(wav_bytes)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("diarization service HTTP {} — {}", status, body);
    }

    #[derive(serde::Deserialize)]
    struct Resp {
        segments: Vec<Segment>,
    }
    let data: Resp = resp.json().await?;
    Ok(data.segments)
}

/// Returns the speaker label with the most speaking time in the given segments.
pub fn dominant_speaker(segments: &[Segment]) -> Option<String> {
    let mut dur: HashMap<&str, f64> = HashMap::new();
    for s in segments {
        *dur.entry(s.speaker.as_str()).or_default() += s.end - s.start;
    }
    dur.into_iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(k, _)| k.to_string())
}

// ── Speaker tracker ───────────────────────────────────────────────────────────

/// Maintains cumulative speaking time per pyannote speaker label across the session.
/// Maps arbitrary labels (SPEAKER_00, SPEAKER_01) to roles.
///
/// Heuristic: the label with the most total speaking time = "You" (the candidate),
/// since candidates produce the majority of words in a job interview.
#[derive(Default)]
pub struct SpeakerTracker {
    cumulative: HashMap<String, f64>,
}

impl SpeakerTracker {
    /// Accumulate speaking duration from a diarization result.
    pub fn record(&mut self, segments: &[Segment]) {
        for s in segments {
            *self.cumulative.entry(s.speaker.clone()).or_default() +=
                (s.end - s.start).max(0.0);
        }
    }

    /// Map a pyannote label to a role.
    /// "You" = the label with the most cumulative time.
    /// "Interviewer" = any other label.
    pub fn role(&self, label: &str) -> &'static str {
        let you_label = self
            .cumulative
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(k, _)| k.as_str())
            .unwrap_or("");
        if label == you_label {
            "You"
        } else {
            "Interviewer"
        }
    }
}

pub type SharedTracker = Arc<Mutex<SpeakerTracker>>;

// ── WAV encoding (for sending PCM to sidecar) ─────────────────────────────────

pub(crate) fn pcm_to_wav(pcm: &[u8]) -> Result<Vec<u8>> {
    let spec = WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut writer = WavWriter::new(&mut buf, spec)?;
        for chunk in pcm.chunks_exact(2) {
            writer.write_sample(i16::from_le_bytes([chunk[0], chunk[1]]))?;
        }
        writer.finalize()?;
    }
    Ok(buf.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(speaker: &str, start: f64, end: f64) -> Segment {
        Segment { speaker: speaker.to_string(), start, end }
    }

    // ── dominant_speaker ─────────────────────────────────────────────────────

    #[test]
    fn dominant_speaker_picks_longest() {
        let segs = vec![seg("SPEAKER_00", 0.0, 2.0), seg("SPEAKER_01", 2.0, 9.0)];
        assert_eq!(dominant_speaker(&segs).as_deref(), Some("SPEAKER_01"));
    }

    #[test]
    fn dominant_speaker_empty_returns_none() {
        assert_eq!(dominant_speaker(&[]), None);
    }

    #[test]
    fn dominant_speaker_single_entry() {
        let segs = vec![seg("SPEAKER_00", 0.0, 5.0)];
        assert_eq!(dominant_speaker(&segs).as_deref(), Some("SPEAKER_00"));
    }

    // ── SpeakerTracker ───────────────────────────────────────────────────────

    #[test]
    fn tracker_maps_longer_speaker_to_you() {
        let mut t = SpeakerTracker::default();
        // SPEAKER_00 talks for 30s total → "You"
        t.record(&[seg("SPEAKER_00", 0.0, 15.0), seg("SPEAKER_00", 20.0, 35.0)]);
        // SPEAKER_01 talks for 5s total → "Interviewer"
        t.record(&[seg("SPEAKER_01", 35.0, 40.0)]);
        assert_eq!(t.role("SPEAKER_00"), "You");
        assert_eq!(t.role("SPEAKER_01"), "Interviewer");
    }

    #[test]
    fn tracker_single_label_is_you() {
        let mut t = SpeakerTracker::default();
        t.record(&[seg("SPEAKER_00", 0.0, 10.0)]);
        assert_eq!(t.role("SPEAKER_00"), "You");
    }

    #[test]
    fn tracker_accumulates_across_calls() {
        let mut t = SpeakerTracker::default();
        t.record(&[seg("SPEAKER_00", 0.0, 5.0)]);
        t.record(&[seg("SPEAKER_01", 5.0, 15.0)]); // SPEAKER_01 leads now
        assert_eq!(t.role("SPEAKER_01"), "You");
        assert_eq!(t.role("SPEAKER_00"), "Interviewer");
        t.record(&[seg("SPEAKER_00", 15.0, 30.0)]); // SPEAKER_00 takes lead back
        assert_eq!(t.role("SPEAKER_00"), "You");
    }

    // ── pcm_to_wav ───────────────────────────────────────────────────────────

    #[test]
    fn pcm_to_wav_produces_valid_wav_header() {
        let pcm = vec![0u8; 3200]; // 0.1s of silence at 16kHz
        let wav = pcm_to_wav(&pcm).unwrap();
        assert!(wav.starts_with(b"RIFF"), "missing RIFF header");
        assert!(&wav[8..12] == b"WAVE", "missing WAVE marker");
        assert!(wav.len() > 44, "WAV too short to have data");
    }

    #[test]
    fn pcm_to_wav_empty_input() {
        let wav = pcm_to_wav(&[]).unwrap();
        // Should produce a valid but empty WAV (header only)
        assert!(wav.starts_with(b"RIFF"));
    }
}
