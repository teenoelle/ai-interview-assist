use anyhow::{anyhow, Result};
use std::sync::OnceLock;

static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
fn client() -> &'static reqwest::Client {
    CLIENT.get_or_init(reqwest::Client::new)
}

/// Transcribe PCM audio via Deepgram Nova-2 (200 hours/month free tier).
/// PCM must be 16-bit signed LE, 16kHz, mono.
pub async fn transcribe(api_key: &str, pcm: &[u8]) -> Result<String> {
    let wav_bytes = pcm_to_wav(pcm)?;

    let resp = client()
        .post("https://api.deepgram.com/v1/listen?model=nova-2&language=en&smart_format=true&disfluencies=true")
        .header("Authorization", format!("Token {}", api_key))
        .header("Content-Type", "audio/wav")
        .body(wav_bytes)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!("Deepgram error {}: {}", status, body));
    }

    let body: serde_json::Value = resp.json().await?;
    let transcript = body["results"]["channels"][0]["alternatives"][0]["transcript"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();
    Ok(transcript)
}

fn pcm_to_wav(pcm: &[u8]) -> Result<Vec<u8>> {
    use hound::{SampleFormat, WavSpec, WavWriter};
    let spec = WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut writer = WavWriter::new(&mut buf, spec)?;
        for chunk in pcm.chunks(2) {
            if chunk.len() == 2 {
                writer.write_sample(i16::from_le_bytes([chunk[0], chunk[1]]))?;
            }
        }
        writer.finalize()?;
    }
    Ok(buf.into_inner())
}
