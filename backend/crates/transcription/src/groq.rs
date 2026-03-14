use anyhow::{anyhow, Result};
use hound::{SampleFormat, WavSpec, WavWriter};
use std::sync::OnceLock;

static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
fn client() -> &'static reqwest::Client {
    CLIENT.get_or_init(reqwest::Client::new)
}

pub async fn transcribe(api_key: &str, pcm: &[u8]) -> Result<String> {
    let wav_bytes = pcm_to_wav(pcm)?;

    let part = reqwest::multipart::Part::bytes(wav_bytes)
        .file_name("audio.wav")
        .mime_str("audio/wav")?;

    let form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("model", "whisper-large-v3-turbo")
        .text("language", "en")
        .text("response_format", "text")
        .text("prompt", "Interview conversation. Speaker may have a non-native accent. Transcribe faithfully.");

    let resp = client()
        .post("https://api.groq.com/openai/v1/audio/transcriptions")
        .bearer_auth(api_key)
        .multipart(form)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!("Groq transcription error {}: {}", status, body));
    }

    let text = resp.text().await?;
    Ok(text.trim().to_string())
}

fn pcm_to_wav(pcm: &[u8]) -> Result<Vec<u8>> {
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
