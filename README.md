# AI Interview Assistant

A real-time interview coaching tool that listens to your interview audio, detects questions, and surfaces structured answer suggestions — all running locally in your browser.

![License: CC BY-NC-SA 4.0](https://img.shields.io/badge/License-CC%20BY--NC--SA%204.0-lightgrey.svg)

---

## What it does

- **Transcribes** your interview audio in real time (local Whisper or cloud fallback)
- **Detects** the question type — behavioral, motivation, strengths, weaknesses, fit, closing, and more
- **Generates** structured answer suggestions streamed to a teleprompter-style panel
- **Speaks** the first suggestion section aloud so you can stay focused on the interviewer
- **Tracks** sentiment from your webcam feed (optional)
- **Stores** question history so you can navigate back during the interview

Answer frameworks vary by question type: STAR for behavioral, Motivation structure for "why this role", Fit structure (Acknowledge → Reframe → Gap → Choice → Bring) for level/channel mismatch questions, and so on.

---

## Tech stack

| Layer | Stack |
|---|---|
| Backend | Rust · axum 0.8 · tokio |
| Frontend | Svelte 5 · Vite 6 · TypeScript |
| Transcription | Local Whisper → Groq Whisper → Gemini |
| Suggestions | Claude Haiku → Groq → Cerebras → OpenRouter → Qwen → Mistral → Ollama → Gemini |
| Sentiment | Claude Haiku → Ollama Vision (llava) → Gemini Vision |
| Audio capture | Browser `getDisplayMedia` + AudioWorklet (no extension required) |

---

## Setup

### 1. API keys

```bash
cp backend/.env.example backend/.env
```

Edit `backend/.env` and fill in your keys. Only `GEMINI_API_KEY` is required — everything else enables higher-quality or faster fallbacks.

| Key | Purpose |
|---|---|
| `GEMINI_API_KEY` | Required — transcription fallback, sentiment, context |
| `ANTHROPIC_API_KEY` | Recommended — Claude Haiku for suggestions (highest quality) |
| `GROQ_API_KEY` | Optional — fast cloud transcription + suggestion fallback |
| `OPENROUTER_API_KEY` | Optional — suggestion fallback |
| `CEREBRAS_API_KEY` | Optional — suggestion fallback |
| `MISTRAL_API_KEY` | Optional — suggestion fallback |
| `QWEN_API_KEY` | Optional — suggestion fallback |

### 2. Build the backend

```bash
cd backend
cargo build --release
```

### 3. Build the frontend

```bash
cd frontend
npm install
npm run build
```

The frontend builds into `frontend/dist/` and is served as static files by the backend.

### 4. Run

```bash
cd backend
# On Windows (PowerShell):
$env:$(cat .env | Where-Object { $_ -notmatch '^#' -and $_ -ne '' } | ForEach-Object { $_ }) 2>$null; .\target\release\server.exe
# Or use start.bat from the project root
```

Open **http://localhost:3000**

---

## Optional local services

### Local Whisper (lowest latency transcription)

```bash
pip install faster-whisper-server
faster-whisper-server --port 8000 Systran/faster-whisper-large-v3
```

Or double-click `start_whisper.bat`.

### Ollama (offline suggestion fallback)

Install [Ollama](https://ollama.com), then:

```bash
ollama pull llama3.2
```

### Speaker diarization

Requires a Hugging Face token with [pyannote](https://huggingface.co/pyannote/speaker-diarization-3.1) access. Set `HF_TOKEN` in `.env`, then:

```bash
pip install -r requirements_diarize.txt
python diarize_server.py
```

---

## Keyboard shortcuts

| Key | Action |
|---|---|
| `↑` / `↓` | Navigate question history |
| `→` | Jump to the most recent question (saves position) |
| `←` | Return to the saved position |

---

## License

[CC BY-NC-SA 4.0](LICENSE) — free to use and modify; commercial use is prohibited; derivatives must use the same license.
