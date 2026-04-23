#Requires -Version 5.1
<#
.SYNOPSIS  Start the WASM-as-OS backend and frontend dev server.
#>
Set-StrictMode -Version Latest

$Root     = $PSScriptRoot
$Backend  = Join-Path $Root "wasmos"
$Frontend = Join-Path $Root "frontend"

function Write-Ok   { param($m) Write-Host "  v  $m" -ForegroundColor Green }
function Write-Step { param($m) Write-Host "  >  $m" -ForegroundColor Cyan  }
function Write-Warn { param($m) Write-Host "  !  $m" -ForegroundColor Yellow }

Write-Host ""
Write-Host "  WASM-as-OS -- Starting Services" -ForegroundColor Magenta
Write-Host ""

# Prefer release binary if it exists, fall back to debug
$relBin = Join-Path $Backend "target\release\wasmos.exe"
$dbgBin = Join-Path $Backend "target\debug\wasmos.exe"

if     (Test-Path $relBin) { $bin = $relBin }
elseif (Test-Path $dbgBin) { $bin = $dbgBin }
else {
    Write-Warn "No compiled backend binary found. Run .\install.ps1 first."
    exit 1
}

# Start backend in its own window
Write-Step "Starting backend: $bin"
$backendProc = Start-Process -FilePath $bin `
    -WorkingDirectory $Backend `
    -WindowStyle Normal `
    -PassThru
Write-Ok "Backend started (PID $($backendProc.Id)) -> http://127.0.0.1:8080"

Start-Sleep -Seconds 2

# Start frontend dev server in its own window
Write-Step "Starting frontend: npm run dev"
$frontendProc = Start-Process -FilePath "cmd.exe" `
    -ArgumentList "/k npm run dev" `
    -WorkingDirectory $Frontend `
    -WindowStyle Normal `
    -PassThru
Write-Ok "Frontend started (PID $($frontendProc.Id)) -> http://127.0.0.1:3001"

# Save PIDs so stop.ps1 can clean up
"$($backendProc.Id),$($frontendProc.Id)" | Set-Content (Join-Path $Root ".wasmos-pids")

Write-Host ""
Write-Host "  Services running:" -ForegroundColor Cyan
Write-Host "    Backend  -> http://127.0.0.1:8080" -ForegroundColor White
Write-Host "    Frontend -> http://127.0.0.1:3001" -ForegroundColor White
Write-Host ""
Write-Host "  Run .\stop.ps1 to stop all services." -ForegroundColor DarkGray
Write-Host ""
