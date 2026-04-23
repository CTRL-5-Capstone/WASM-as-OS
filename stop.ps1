#Requires -Version 5.1
<#
.SYNOPSIS  Stop WASM-as-OS backend and frontend processes.
#>
$Root    = $PSScriptRoot
$pidFile = Join-Path $Root ".wasmos-pids"

function Write-Ok   { param($m) Write-Host "  v  $m" -ForegroundColor Green  }
function Write-Warn { param($m) Write-Host "  !  $m" -ForegroundColor Yellow }

Write-Host ""
Write-Host "  WASM-as-OS -- Stopping Services" -ForegroundColor Magenta
Write-Host ""

if (Test-Path $pidFile) {
    $pids = (Get-Content $pidFile) -split ","
    foreach ($p in $pids) {
        $p = $p.Trim()
        if ($p -match "^\d+$") {
            try {
                Stop-Process -Id ([int]$p) -Force -ErrorAction Stop
                Write-Ok "Stopped PID $p"
            } catch {
                Write-Warn "PID $p not found (already stopped?)"
            }
        }
    }
    Remove-Item $pidFile -Force
} else {
    Write-Warn "No PID file (.wasmos-pids) found — trying to stop by process name..."
    Get-Process -Name "wasmos" -ErrorAction SilentlyContinue | Stop-Process -Force
    Write-Ok "Sent stop signal to wasmos process(es)."
}

Write-Host ""
Write-Host "  All services stopped." -ForegroundColor Green
Write-Host ""
