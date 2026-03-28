# verify-local-dev.ps1 - End-to-end local integration smoke test (Windows PowerShell)
#
# Validates:
# - Backend health/metrics/API (direct)
# - Frontend pages (Next dev)
# - Frontend -> backend rewrites (proxy)
#
# Usage:
#   .\scripts\verify-local-dev.ps1
#   .\scripts\verify-local-dev.ps1 -FrontendUrl http://127.0.0.1:3001 -BackendUrl http://127.0.0.1:8080

param(
    [string]$FrontendUrl = "http://127.0.0.1:3001",
    [string]$BackendUrl  = "http://127.0.0.1:8080"
)

$frontend = $FrontendUrl.TrimEnd('/')
$backend  = $BackendUrl.TrimEnd('/')

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
        return $resp
    } catch {
        Write-Host "  x  $Label -> $($_.Exception.Message)" -ForegroundColor Red
        $script:Fail++
        return $null
    }
}

Write-Host "";
Write-Host "======================================================" -ForegroundColor Cyan
Write-Host "  WasmOS Local Integration Verification" -ForegroundColor Cyan
Write-Host "  Frontend: $frontend" -ForegroundColor Cyan
Write-Host "  Backend:  $backend" -ForegroundColor Cyan
Write-Host "======================================================" -ForegroundColor Cyan
Write-Host ""

Write-Host "-- 1. Backend (direct)" -ForegroundColor Yellow
Invoke-Check "GET /health/live"  "$backend/health/live"  200 | Out-Null
Invoke-Check "GET /health/ready" "$backend/health/ready" 200 | Out-Null
Invoke-Check "GET /metrics"      "$backend/metrics"      200 "# HELP" | Out-Null
Invoke-Check "GET /v1/tasks"     "$backend/v1/tasks"     200 | Out-Null

Write-Host "";
Write-Host "-- 2. Frontend (Next dev)" -ForegroundColor Yellow
$pages = @("/", "/tasks/", "/terminal/", "/metrics/", "/analytics/", "/audit/")
foreach ($p in $pages) { Invoke-Check "GET $p" "$frontend$p" 200 | Out-Null }

Write-Host "";
Write-Host "-- 3. Frontend -> Backend Proxy" -ForegroundColor Yellow
Invoke-Check "GET /health/ready/ (via frontend)" "$frontend/health/ready/" 200 | Out-Null
Invoke-Check "GET /v1/tasks/ (via frontend)"     "$frontend/v1/tasks/"     200 | Out-Null

Write-Host "";
Write-Host "======================================================" -ForegroundColor Cyan
$color = if ($Fail -eq 0) { "Green" } else { "Red" }
Write-Host "  Results: $Pass passed / $Fail failed / $Total total" -ForegroundColor $color
Write-Host "======================================================" -ForegroundColor Cyan
Write-Host ""

exit $(if ($Fail -eq 0) { 0 } else { 1 })
