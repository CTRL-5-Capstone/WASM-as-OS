# stop-local-dev.ps1 - Stop backend and frontend dev servers by port.
# Usage:
#   .\scripts\stop-local-dev.ps1

function Stop-PortListener {
    param([int]$Port)
    $conn = Get-NetTCPConnection -State Listen -LocalPort $Port -ErrorAction SilentlyContinue | Select-Object -First 1
    if (-not $conn) {
        Write-Host "Nothing listening on :$Port" -ForegroundColor DarkGray
        return
    }
    $procId = $conn.OwningProcess
    try {
        $p = Get-Process -Id $procId -ErrorAction Stop
        Write-Host "Stopping $($p.ProcessName) (PID $procId) on :$Port" -ForegroundColor Yellow
        Stop-Process -Id $procId -Force -ErrorAction Stop
    } catch {
        Write-Host "Could not stop PID $procId on :$Port ($($_.Exception.Message))" -ForegroundColor Red
    }
}

Stop-PortListener -Port 3001
Stop-PortListener -Port 8080
