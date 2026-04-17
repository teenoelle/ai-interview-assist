# AI Interview Assistant

A real-time interview coaching tool that listens to your interview audio, detects questions, and surfaces structured answer suggestions — all running locally in your browser.

![License: CC BY-NC-SA 4.0](https://img.shields.io/badge/License-CC%20BY--NC--SA%204.0-lightgrey.svg)

---

## What it does

- **Transcribes** your interview audio in real time (local Whisper, Deepgram, Groq, or Gemini fallback)
- **Detects** the question type — behavioral, motivation, strengths, weaknesses, fit, closing, and more
- **Generates** structured answer suggestions streamed to a teleprompter-style panel
- **Speaks** the first suggestion section aloud so you can stay focused on the interviewer
- **Tracks** sentiment from your webcam feed (optional)
- **Stores** question history so you can navigate back during the interview
- **Tracks keywords** from the job description with a zoomable chip bar and inline definitions
- **Monitors energy** with filler word detection and speaking pace feedback
- **Shows rate limit** status across all configured providers

Answer frameworks vary by question type: STAR for behavioral, Motivation structure for "why this role", Fit structure (Acknowledge → Reframe → Gap → Choice → Bring) for level/channel mismatch questions, and so on.

---

## Tech stack

| Layer | Stack |
|---|---|
| Backend | Rust · axum 0.8 · tokio |
| Frontend | Svelte 5 · Vite 6 · TypeScript |
| Transcription | Local Whisper → Deepgram → Groq Whisper → Gemini |
| Suggestions | Groq → Mistral → Claude CLI → Claude API → Ollama → OpenRouter → Qwen → Cerebras → DeepSeek → LAN Ollama → Gemma → Gemini (order configurable in UI) |
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
| `ANTHROPIC_API_KEY` | Recommended — Claude API for suggestions |
| `GROQ_API_KEY` | Optional — fast cloud transcription + suggestion fallback |
| `GROQ_API_KEY_2` | Optional — second Groq key to double throughput |
| `DEEPGRAM_API_KEY` | Optional — Deepgram Nova-2 transcription |
| `DEEPSEEK_API_KEY` | Optional — DeepSeek suggestion fallback |
| `OPENROUTER_API_KEY` | Optional — suggestion fallback |
| `CEREBRAS_API_KEY` | Optional — suggestion fallback |
| `MISTRAL_API_KEY` | Optional — suggestion fallback |
| `QWEN_API_KEY` | Optional — suggestion fallback |
| `BONSAI_URL` | Optional — URL of a local/LAN Bonsai-compatible inference server |
| `BONSAI_MODEL` | Optional — model to use with Bonsai (default: `gemma3:4b`) |

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

Install [Ollama](https://ollama.com), then pull a model:

```bash
ollama pull llama3.2
```

A second Ollama instance on your LAN can be configured as **LAN Ollama** via the Providers panel in the UI — useful for offloading inference to a more powerful machine.

### Claude CLI (suggestion via Anthropic Pro subscription)

If you have Claude Pro, the Claude CLI provider uses your existing OAuth session instead of an API key:

```bash
npm install -g @anthropic-ai/claude-code
claude  # log in once
```

No `ANTHROPIC_API_KEY` required for this path.

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
