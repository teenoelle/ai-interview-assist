@echo off
echo Starting faster-whisper-server on port 8000...
echo Model: Systran/faster-whisper-medium.en (downloads ~800MB on first run)
echo.
"C:\Users\tyfel\AppData\Roaming\Python\Python313\Scripts\faster-whisper-server.exe" --port 8000 Systran/faster-whisper-medium.en
