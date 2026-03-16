use anyhow::{anyhow, Result};
use hound::{SampleFormat, WavSpec, WavWriter};
use std::sync::OnceLock;

static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
fn client() -> &'static reqwest::Client {
    CLIENT.get_or_init(reqwest::Client::new)
}

pub async fn transcribe(api_key: &str, pcm: &[u8]) -> Result<String> {
    transcribe_openai_asr(
        "https://api.groq.com/openai/v1/audio/transcriptions",
        api_key,
        "whisper-large-v3",
        pcm,
    )
    .await
}

/// Call any OpenAI-compatible /audio/transcriptions endpoint.
/// Works with Groq, faster-whisper-server, whisper.cpp HTTP server, etc.
pub async fn transcribe_openai_asr(url: &str, api_key: &str, model: &str, pcm: &[u8]) -> Result<String> {
    let wav_bytes = pcm_to_wav(pcm)?;

    let part = reqwest::multipart::Part::bytes(wav_bytes)
        .file_name("audio.wav")
        .mime_str("audio/wav")?;

    let form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("model", model.to_string())
        .text("language", "en")
        .text("response_format", "text")
        .text("prompt", "Interview conversation. Speaker may have a non-native accent. Transcribe faithfully.");

    let mut req = client()
        .post(url)
        .multipart(form)
        .timeout(std::time::Duration::from_secs(60));
    if !api_key.is_empty() {
        req = req.bearer_auth(api_key);
    }
    let resp = req.send().await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!("ASR error {}: {}", status, body));
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
