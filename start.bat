@echo off
cd /d "%~dp0"

echo Starting AI Interview Assistant...

REM Start backend
start "AI Interview Backend" /min cmd /c "cd backend && target\release\server.exe"

REM Wait a moment for backend to bind
timeout /t 2 /nobreak >nul

REM Open browser
start "" "http://localhost:3000"

echo.
echo App running at http://localhost:3000
echo Close this window or press Ctrl+C to stop.
pause >nul
