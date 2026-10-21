@echo off
chcp 65001 >nul
cd /d "%~dp0"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0Start-Test.ps1"
if errorlevel 1 echo Test launcher returned an error. Please inspect the message above.
pause
