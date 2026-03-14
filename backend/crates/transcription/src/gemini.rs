use anyhow::{anyhow, Result};
use base64::Engine;
use hound::{SampleFormat, WavWriter, WavSpec};
use serde_json::{json, Value};
use std::sync::OnceLock;

static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
fn client() -> &'static reqwest::Client {
    CLIENT.get_or_init(reqwest::Client::new)
}

pub async fn transcribe(api_key: &str, pcm: &[u8]) -> Result<String> {
    let wav_bytes = pcm_to_wav(pcm)?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&wav_bytes);

    let body = json!({
        "contents": [{
            "parts": [
                { "inlineData": { "mimeType": "audio/wav", "data": b64 } },
                { "text": "Transcribe this audio exactly as spoken. The speaker may have a non-native English accent — preserve their exact words and phrasing. Return only the spoken words with no commentary, labels, or punctuation beyond what was spoken. If no speech is detected, return an empty string." }
            ]
        }]
    });

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={}",
        api_key
    );

    let resp = client()
        .post(&url)
        .json(&body)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(anyhow!("Gemini API error {}: {}", status, text));
    }

    let json: Value = resp.json().await?;
    let text = json["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .unwrap_or("")
        .to_string();

    Ok(text)
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
                let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
                writer.write_sample(sample)?;
            }
        }
        writer.finalize()?;
    }
    Ok(buf.into_inner())
}
