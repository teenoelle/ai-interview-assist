use axum::{extract::State, Json, response::{IntoResponse, Response}};
use axum::http::{header, StatusCode};
use tokio::io::AsyncWriteExt;
use std::process::Stdio;
use crate::state::AppState;

#[derive(serde::Serialize)]
pub struct TtsVoice {
    pub id: String,
    pub name: String,
    pub source: String, // "os" | "piper"
}

#[derive(serde::Deserialize)]
pub struct SpeakRequest {
    pub text: String,
    pub voice_id: String,
    pub rate: Option<f32>,
    pub volume: Option<f32>,
}

/// GET /api/tts/voices — returns OS voices (Windows SAPI) + Piper model files
pub async fn handle_tts_voices(State(state): State<AppState>) -> Json<Vec<TtsVoice>> {
    let mut voices: Vec<TtsVoice> = Vec::new();

    // --- OS voices via PowerShell (Windows SAPI) ---
    let ps = r#"Add-Type -AssemblyName System.Speech; (New-Object System.Speech.Synthesis.SpeechSynthesizer).GetInstalledVoices() | ForEach-Object { $_.VoiceInfo.Name }"#;
    if let Ok(out) = tokio::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", ps])
        .output().await
    {
        for name in String::from_utf8_lossy(&out.stdout).lines().map(|l| l.trim()).filter(|l| !l.is_empty()) {
            voices.push(TtsVoice {
                id: format!("os:{}", name),
                name: format!("{} (Windows)", name),
                source: "os".into(),
            });
        }
    }

    // --- Piper voice models (.onnx files in PIPER_MODELS_DIR) ---
    if let Some(dir) = &state.piper_models_dir {
        if let Ok(entries) = std::fs::read_dir(dir) {
            let mut piper: Vec<TtsVoice> = entries.flatten()
                .filter_map(|e| {
                    let n = e.file_name().to_string_lossy().to_string();
                    // skip .onnx.json config files, keep only .onnx
                    if n.ends_with(".onnx") && !n.ends_with(".onnx.json") {
                        let id = n.trim_end_matches(".onnx").to_string();
                        Some(TtsVoice {
                            id: format!("piper:{}", id),
                            name: format!("{} (Piper)", id.replace('_', " ").replace('-', " ")),
                            source: "piper".into(),
                        })
                    } else { None }
                })
                .collect();
            piper.sort_by(|a, b| a.name.cmp(&b.name));
            voices.extend(piper);
        }
    }

    Json(voices)
}

/// POST /api/tts/speak — synthesize text and return audio/wav
pub async fn handle_speak(
    State(state): State<AppState>,
    Json(req): Json<SpeakRequest>,
) -> impl IntoResponse {
    let rate = req.rate.unwrap_or(1.0);
    let volume = req.volume.unwrap_or(1.0);

    let result = if req.voice_id.starts_with("os:") {
        let voice_name = &req.voice_id[3..];
        synthesize_os(voice_name, &req.text, rate, volume).await
    } else if req.voice_id.starts_with("piper:") {
        let model_id = &req.voice_id[6..];
        match (&state.piper_binary, &state.piper_models_dir) {
            (Some(bin), Some(dir)) => {
                let model_path = format!("{}/{}.onnx", dir.trim_end_matches('/'), model_id);
                synthesize_piper(bin, &model_path, &req.text, rate).await
            }
            _ => Err("Piper not configured (set PIPER_BINARY and PIPER_MODELS_DIR)".into()),
        }
    } else {
        Err("Unknown voice source".into())
    };

    match result {
        Ok(wav) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "audio/wav")
            .header(header::CONTENT_LENGTH, wav.len())
            .body(axum::body::Body::from(wav))
            .unwrap(),
        Err(e) => {
            tracing::warn!("TTS synthesis failed: {}", e);
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(axum::body::Body::from(e))
                .unwrap()
        }
    }
}

/// Synthesize via Windows SAPI using PowerShell; returns raw WAV bytes.
async fn synthesize_os(voice_name: &str, text: &str, rate: f32, volume: f32) -> Result<Vec<u8>, String> {
    // SAPI rate: -10 (slowest) to 10 (fastest), 0 = normal. Browser rate 1.0 → 0.
    let sapi_rate = ((rate - 1.0) * 5.0).clamp(-10.0, 10.0) as i32;
    let sapi_vol = (volume * 100.0).clamp(0.0, 100.0) as u32;

    // Escape single quotes for PowerShell string literals
    let voice_esc = voice_name.replace('\'', "''");
    let text_esc = text.replace('\'', "''").replace('\n', " ");

    let ps = format!(
        r#"Add-Type -AssemblyName System.Speech; $s = New-Object System.Speech.Synthesis.SpeechSynthesizer; $s.SelectVoice('{v}'); $s.Rate = {r}; $s.Volume = {vol}; $ms = New-Object System.IO.MemoryStream; $s.SetOutputToWaveStream($ms); $s.Speak('{t}'); $s.Dispose(); $out = [Console]::OpenStandardOutput(); $out.Write($ms.ToArray(), 0, $ms.Length); $out.Flush()"#,
        v = voice_esc, r = sapi_rate, vol = sapi_vol, t = text_esc
    );

    let output = tokio::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &ps])
        .output()
        .await
        .map_err(|e| format!("PowerShell spawn failed: {e}"))?;

    if !output.stdout.is_empty() {
        // Inject 300ms of silence after the WAV header to prevent OS audio device latency clipping the first phoneme.
        // WAV header is 44 bytes; detect sample rate and channels from header to compute silence size.
        let wav = output.stdout;
        if wav.len() > 44 {
            let channels = u16::from_le_bytes([wav[22], wav[23]]) as usize;
            let sample_rate = u32::from_le_bytes([wav[24], wav[25], wav[26], wav[27]]) as usize;
            let bits = u16::from_le_bytes([wav[34], wav[35]]) as usize;
            let silence_bytes = sample_rate * 300 / 1000 * channels * bits / 8;
            let silence = vec![0u8; silence_bytes];
            // Rebuild: header + silence + original PCM data
            let pcm = &wav[44..];
            Ok(make_wav_with_prefix(&silence, pcm, sample_rate as u32, channels as u16, bits as u16))
        } else {
            Ok(wav)
        }
    } else {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(format!("SAPI synthesis failed: {err}"))
    }
}

/// Synthesize via Piper TTS binary; returns WAV bytes (wraps raw PCM output).
async fn synthesize_piper(binary: &str, model_path: &str, text: &str, rate: f32) -> Result<Vec<u8>, String> {
    // Piper length_scale > 1 = slower, < 1 = faster. Inverse of rate.
    let length_scale = format!("{:.3}", (1.0_f32 / rate.max(0.1)));

    let mut child = tokio::process::Command::new(binary)
        .arg("--model").arg(model_path)
        .arg("--output_raw")
        .arg("--length_scale").arg(&length_scale)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to spawn Piper: {e}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes()).await.map_err(|e| e.to_string())?;
    }

    let out = child.wait_with_output().await.map_err(|e| e.to_string())?;

    if out.stdout.is_empty() {
        return Err("Piper produced no audio output".into());
    }

    // Piper --output_raw: 16-bit signed PCM, 22050 Hz, mono
    // Prepend 300ms of silence to absorb OS audio device latency on first playback
    let silence_samples = (22050 * 300 / 1000) * 2; // 300ms * 2 bytes/sample
    let mut pcm = vec![0u8; silence_samples];
    pcm.extend_from_slice(&out.stdout);
    Ok(make_wav(&pcm, 22050, 1, 16))
}

fn make_wav_with_prefix(prefix: &[u8], pcm: &[u8], sample_rate: u32, channels: u16, bits: u16) -> Vec<u8> {
    let mut combined = prefix.to_vec();
    combined.extend_from_slice(pcm);
    make_wav(&combined, sample_rate, channels, bits)
}

/// Wrap raw PCM bytes in a minimal RIFF/WAV container.
fn make_wav(pcm: &[u8], sample_rate: u32, channels: u16, bits: u16) -> Vec<u8> {
    let data_len = pcm.len() as u32;
    let byte_rate = sample_rate * channels as u32 * bits as u32 / 8;
    let block_align = channels * bits / 8;
    let mut w = Vec::with_capacity(44 + pcm.len());
    w.extend_from_slice(b"RIFF");
    w.extend_from_slice(&(36 + data_len).to_le_bytes());
    w.extend_from_slice(b"WAVE");
    w.extend_from_slice(b"fmt ");
    w.extend_from_slice(&16u32.to_le_bytes());       // chunk size
    w.extend_from_slice(&1u16.to_le_bytes());         // PCM format
    w.extend_from_slice(&channels.to_le_bytes());
    w.extend_from_slice(&sample_rate.to_le_bytes());
    w.extend_from_slice(&byte_rate.to_le_bytes());
    w.extend_from_slice(&block_align.to_le_bytes());
    w.extend_from_slice(&bits.to_le_bytes());
    w.extend_from_slice(b"data");
    w.extend_from_slice(&data_len.to_le_bytes());
    w.extend_from_slice(pcm);
    w
}
