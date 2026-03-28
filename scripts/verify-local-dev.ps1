# verify-local-dev.ps1 - Local smoke test (backend only)
#
# Usage:
#   .\scripts\verify-local-dev.ps1
#   .\scripts\verify-local-dev.ps1 -BackendUrl http://127.0.0.1:8080

param(
    [string]$BackendUrl = "http://127.0.0.1:8080"
)

$backend = $BackendUrl.TrimEnd('/')

$script:Pass = 0
$script:Fail = 0
$script:Total = 0

function Invoke-Check {
    param([string]$Label, [string]$Url, [int]$ExpectedStatus = 200, [string]$BodyContains = "")
    $script:Total++
    try {
        $resp = Invoke-WebRequest -Uri $Url -UseBasicParsing -TimeoutSec 15 -ErrorAction Stop
        $code = $resp.StatusCode
        if ($code -ne $ExpectedStatus) { throw "expected HTTP $ExpectedStatus, got $code" }
        if ($BodyContains -and -not $resp.Content.Contains($BodyContains)) { throw "body missing '$BodyContains'" }
        Write-Host "  v  $Label -> HTTP $code" -ForegroundColor Green
        $script:Pass++
    } catch {
        Write-Host "  x  $Label -> $($_.Exception.Message)" -ForegroundColor Red
        $script:Fail++
    }
}

Write-Host "";
Write-Host "======================================================" -ForegroundColor Cyan
Write-Host "  WasmOS Local Verification" -ForegroundColor Cyan
Write-Host "  Backend:  $backend" -ForegroundColor Cyan
Write-Host "======================================================" -ForegroundColor Cyan
Write-Host ""

Invoke-Check "GET /health/live"  "$backend/health/live"  200
Invoke-Check "GET /health/ready" "$backend/health/ready" 200
Invoke-Check "GET /metrics"      "$backend/metrics"      200 "# HELP"

Write-Host "";
Write-Host "======================================================" -ForegroundColor Cyan
$color = if ($Fail -eq 0) { "Green" } else { "Red" }
Write-Host "  Results: $Pass passed / $Fail failed / $Total total" -ForegroundColor $color
Write-Host "======================================================" -ForegroundColor Cyan
Write-Host ""

exit $(if ($Fail -eq 0) { 0 } else { 1 })
