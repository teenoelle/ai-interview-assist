@echo off
cd /d "%~dp0"

echo Starting AI Interview Assistant...

REM Start diarization server (optional — falls back to heuristics if unavailable)
start "Diarization Server" /min cmd /c "python diarize_server.py"

REM Start local Whisper server (optional — falls back to Deepgram/Groq/Gemini if unavailable)
start "Whisper Server" /min cmd /c "start_whisper.bat"

REM Ensure Ollama server is running (start if not already listening on port 11434)
netstat -ano | findstr ":11434" | findstr "LISTENING" >nul 2>&1
if errorlevel 1 (
    echo Starting Ollama server...
    set OLLAMA_MAX_LOADED_MODELS=1
    start "Ollama Server" /min "C:\Users\tyfel\AppData\Local\Programs\Ollama\ollama app.exe"
    timeout /t 5 /nobreak >nul
) else (
    echo Ollama already running.
)
REM Model warmup is handled by the backend via HTTP on startup (keep_alive 60m)

REM Give Whisper a head-start before the backend tries to warm it up
echo Waiting for Whisper to start...
timeout /t 8 /nobreak >nul

REM Start backend (loads .env automatically, logs to backend\target\release\logs\server.log)
if not exist backend\logs mkdir backend\logs
start "AI Interview Backend" /min cmd /c "cd backend && target\release\server.exe"

REM Wait for backend to bind
timeout /t 3 /nobreak >nul

REM Open browser
start "" "http://localhost:3000"

echo.
echo App running at http://localhost:3000
echo Diarization server: http://localhost:8001
echo Local Whisper:      http://localhost:8000
echo.
echo Close this window or press Ctrl+C to stop.
pause >nul
