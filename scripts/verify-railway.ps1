# verify-railway.ps1 - Smoke test for a live deployment (Windows PowerShell)
#
# Usage:
#   .\scripts\verify-railway.ps1 -BaseUrl https://your-app.example
#   $env:BASE_URL = "https://..."; .\scripts\verify-railway.ps1

param(
    [string]$BaseUrl = $env:BASE_URL
)

if (-not $BaseUrl) {
    Write-Error "Provide the deployment URL: .\\scripts\\verify-railway.ps1 -BaseUrl https://your-app"
    exit 1
}

$Base = $BaseUrl.TrimEnd('/')
$script:Pass  = 0
$script:Fail  = 0
$script:Total = 0

function Invoke-Check {
    param([string]$Label, [string]$Url, [int]$ExpectedStatus = 200)
    $script:Total++
    try {
        $resp = Invoke-WebRequest -Uri $Url -UseBasicParsing -TimeoutSec 15 -ErrorAction Stop
        if ($resp.StatusCode -ne $ExpectedStatus) { throw "expected HTTP $ExpectedStatus, got $($resp.StatusCode)" }
        Write-Host "  v  $Label -> HTTP $($resp.StatusCode)" -ForegroundColor Green
        $script:Pass++
    } catch {
        Write-Host "  x  $Label -> $($_.Exception.Message)" -ForegroundColor Red
        $script:Fail++
    }
}

Write-Host "";
Write-Host "======================================================" -ForegroundColor Cyan
Write-Host "  WasmOS Deployment Verification" -ForegroundColor Cyan
Write-Host "  Target: $Base" -ForegroundColor Cyan
Write-Host "======================================================" -ForegroundColor Cyan
Write-Host ""

Write-Host "-- 1. Health" -ForegroundColor Yellow
Invoke-Check "GET /health/live"  "$Base/health/live"  200
Invoke-Check "GET /health/ready" "$Base/health/ready" 200

Write-Host "";
Write-Host "======================================================" -ForegroundColor Cyan
$color = if ($Fail -eq 0) { "Green" } else { "Red" }
Write-Host "  Results: $Pass passed / $Fail failed / $Total total" -ForegroundColor $color
Write-Host "======================================================" -ForegroundColor Cyan
Write-Host ""

exit $(if ($Fail -eq 0) { 0 } else { 1 })
