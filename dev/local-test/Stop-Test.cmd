@echo off
chcp 65001 >nul
cd /d "%~dp0"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0Stop-Test.ps1"
if errorlevel 1 echo Stop was refused. No other service was stopped.
pause
