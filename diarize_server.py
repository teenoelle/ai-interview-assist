#!/usr/bin/env python3
"""
Speaker diarization sidecar for AI Interview Assistant.
Runs alongside the Rust backend on http://127.0.0.1:8001.

=== One-time setup ===

1. Install Python deps:
     pip install -r requirements_diarize.txt

2. Get a FREE Hugging Face token:
     a. Sign up at hf.co (free, no credit card)
     b. Accept model terms at: hf.co/pyannote/speaker-diarization-3.1
     c. Accept model terms at: hf.co/pyannote/segmentation-3.0
     d. Go to hf.co/settings/tokens → New token (read access)
     e. Add to your .env file:  HF_TOKEN=hf_...

   The models download automatically on first run (~1 GB total).

=== Running ===
     python diarize_server.py

=== Notes ===
- Runs on CPU by default; GPU (CUDA) used automatically if available.
- CPU diarization takes ~5–15s per 10s of audio, running in parallel
  with transcription so it doesn't block the UI.
- If this server is not running, the Rust backend falls back to the
  built-in text heuristics for speaker detection.
"""

import os
import asyncio
import tempfile
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor
from contextlib import asynccontextmanager

import uvicorn
from fastapi import FastAPI, HTTPException, Request

# ── Load .env ─────────────────────────────────────────────────────────────────

_env_path = Path(__file__).parent / ".env"
if _env_path.exists():
    for _line in _env_path.read_text(encoding="utf-8").splitlines():
        _line = _line.strip()
        if _line and not _line.startswith("#") and "=" in _line:
            _k, _, _v = _line.partition("=")
            os.environ.setdefault(_k.strip(), _v.strip())

HF_TOKEN = os.environ.get("HF_TOKEN", "")
PORT = int(os.environ.get("DIARIZE_PORT", "8001"))

# ── Globals ───────────────────────────────────────────────────────────────────

pipeline = None
# Diarization is CPU-bound; use one worker to avoid memory contention
_executor = ThreadPoolExecutor(max_workers=1)


# ── Pipeline loading ──────────────────────────────────────────────────────────

def _load_pipeline():
    """Blocking: download + load pyannote model. Called once in thread pool."""
    import torch
    from pyannote.audio import Pipeline as PyannotePipeline

    if not HF_TOKEN:
        raise RuntimeError(
            "HF_TOKEN not set.\n"
            "  1. Get a free token at hf.co/settings/tokens\n"
            "  2. Accept terms at hf.co/pyannote/speaker-diarization-3.1\n"
            "  3. Accept terms at hf.co/pyannote/segmentation-3.0\n"
            "  4. Add HF_TOKEN=hf_... to your .env file"
        )

    device = "cuda" if torch.cuda.is_available() else "cpu"
    print(f"[diarize] Loading pyannote/speaker-diarization-3.1 on {device}…")
    p = PyannotePipeline.from_pretrained(
        "pyannote/speaker-diarization-3.1",
        token=HF_TOKEN,
    )
    p.to(torch.device(device))
    print(f"[diarize] Pipeline ready on {device}.")
    return p


# ── Diarization ───────────────────────────────────────────────────────────────

def _diarize_blocking(wav_bytes: bytes) -> list[dict]:
    """Blocking: run diarization on raw WAV bytes. Called in thread pool."""
    with tempfile.NamedTemporaryFile(suffix=".wav", delete=False) as f:
        f.write(wav_bytes)
        tmp_path = f.name
    try:
        result = pipeline(tmp_path)
    finally:
        try:
            os.unlink(tmp_path)
        except OSError:
            pass

    return [
        {
            "speaker": speaker,
            "start": round(turn.start, 3),
            "end": round(turn.end, 3),
        }
        for turn, _, speaker in result.itertracks(yield_label=True)
    ]


# ── App ───────────────────────────────────────────────────────────────────────

@asynccontextmanager
async def lifespan(app: FastAPI):
    global pipeline
    loop = asyncio.get_event_loop()
    try:
        pipeline = await loop.run_in_executor(_executor, _load_pipeline)
    except Exception as exc:
        print(f"[diarize] WARNING — pipeline not loaded: {exc}")
        print("[diarize] Server will run but /diarize returns 503 until fixed.")
    yield


app = FastAPI(title="Diarization Sidecar", lifespan=lifespan)


@app.get("/health")
def health():
    return {"ok": True, "pipeline_loaded": pipeline is not None}


@app.post("/diarize")
async def diarize(request: Request):
    if pipeline is None:
        raise HTTPException(
            503,
            "Diarization pipeline not loaded — check HF_TOKEN and model terms acceptance.",
        )
    wav_bytes = await request.body()
    if not wav_bytes:
        raise HTTPException(400, "Empty request body — send raw WAV bytes.")

    loop = asyncio.get_event_loop()
    segments = await loop.run_in_executor(_executor, _diarize_blocking, wav_bytes)
    return {"segments": segments}


if __name__ == "__main__":
    print(f"[diarize] Starting on http://127.0.0.1:{PORT}")
    uvicorn.run(app, host="127.0.0.1", port=PORT, log_level="info")
