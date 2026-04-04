@echo off
cd /d "%~dp0"

echo Starting AI Interview Assistant...

REM Start diarization server (speaker detection — optional, falls back to heuristics if unavailable)
start "Diarization Server" /min cmd /c "python diarize_server.py"

REM Start local Whisper server (optional — falls back to Groq/Gemini if unavailable)
start "Whisper Server" /min cmd /c "start_whisper.bat"

REM Start backend (logs to backend\logs\server.log)
if not exist backend\logs mkdir backend\logs
start "AI Interview Backend" /min cmd /c "cd backend && target\release\server.exe >> logs\server.log 2>&1"

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
