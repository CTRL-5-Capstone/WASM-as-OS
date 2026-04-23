@echo off
title WASM-as-OS Launcher
echo.
echo  Starting WASM-as-OS...
echo.
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0start.ps1"
pause
