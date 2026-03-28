# start-local-dev.ps1 - Start Postgres (if local service), backend, and frontend dev server.
# Writes logs to:
# - wasmos/logs/wasmos-stdout.log + wasmos/logs/wasmos-stderr.log
# - frontend/logs/next-stdout.log + frontend/logs/next-stderr.log
#
# Usage:
#   .\scripts\start-local-dev.ps1

$root = Split-Path -Parent $PSScriptRoot
$wasmosDir   = Join-Path $root "wasmos"
$frontendDir = Join-Path $root "frontend"

function Stop-PortListener {
    param([int]$Port)
    $conn = Get-NetTCPConnection -State Listen -LocalPort $Port -ErrorAction SilentlyContinue | Select-Object -First 1
    if (-not $conn) { return }
    $procId = $conn.OwningProcess
    try {
        $p = Get-Process -Id $procId -ErrorAction Stop
        Write-Host "Stopping $($p.ProcessName) (PID $procId) on :$Port" -ForegroundColor Yellow
        Stop-Process -Id $procId -Force -ErrorAction Stop
        Start-Sleep -Milliseconds 500
    } catch {
        Write-Host "Could not stop PID $procId on :$Port ($($_.Exception.Message))" -ForegroundColor Red
    }
}

# Ensure local Postgres service is running (if present)
$pg = Get-Service -Name postgresql* -ErrorAction SilentlyContinue | Select-Object -First 1
if ($pg -and $pg.Status -ne 'Running') {
    Write-Host "Starting $($pg.Name)" -ForegroundColor Yellow
    Start-Service $pg.Name
    Start-Sleep -Seconds 2
}

# Free ports
Stop-PortListener -Port 8080
Stop-PortListener -Port 3001

# Backend build if needed
$exe = Join-Path $wasmosDir "target\debug\wasmos.exe"
if (-not (Test-Path $exe)) {
    Write-Host "Building backend..." -ForegroundColor Yellow
    Push-Location $wasmosDir
    cargo build
    Pop-Location
}

# Backend logs
$beLogs = Join-Path $wasmosDir "logs"
New-Item -ItemType Directory -Force -Path $beLogs | Out-Null
$beOut = Join-Path $beLogs "wasmos-stdout.log"
$beErr = Join-Path $beLogs "wasmos-stderr.log"
Remove-Item -Force -ErrorAction SilentlyContinue $beOut,$beErr
$env:RUST_LOG = "info"
$env:WASMOS__LOGGING__FORMAT = "pretty"
Start-Process -FilePath $exe -WorkingDirectory $wasmosDir -RedirectStandardOutput $beOut -RedirectStandardError $beErr | Out-Null

# Frontend logs
$feLogs = Join-Path $frontendDir "logs"
New-Item -ItemType Directory -Force -Path $feLogs | Out-Null
$feOut = Join-Path $feLogs "next-stdout.log"
$feErr = Join-Path $feLogs "next-stderr.log"
Remove-Item -Force -ErrorAction SilentlyContinue $feOut,$feErr

Write-Host "Starting frontend (npm run dev)..." -ForegroundColor Yellow
Start-Process -FilePath "cmd.exe" -ArgumentList "/c","npm run dev" -WorkingDirectory $frontendDir -RedirectStandardOutput $feOut -RedirectStandardError $feErr | Out-Null

Start-Sleep -Seconds 4

Write-Host "Backend:  http://127.0.0.1:8080" -ForegroundColor Green
Write-Host "Frontend: http://127.0.0.1:3001" -ForegroundColor Green
Write-Host "Verify:   powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\verify-local-dev.ps1" -ForegroundColor Green
