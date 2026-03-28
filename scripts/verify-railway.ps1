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
    param([string]$Label, [string]$Url, [int]$ExpectedStatus = 200, [string]$BodyContains = "")
    $script:Total++
    try {
        $resp = Invoke-WebRequest -Uri $Url -UseBasicParsing -TimeoutSec 15 -ErrorAction Stop
        if ($resp.StatusCode -ne $ExpectedStatus) { throw "expected HTTP $ExpectedStatus, got $($resp.StatusCode)" }
        if ($BodyContains -and -not $resp.Content.Contains($BodyContains)) { throw "body missing '$BodyContains'" }
        Write-Host "  v  $Label -> HTTP $($resp.StatusCode)" -ForegroundColor Green
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
Write-Host "  WasmOS Deployment Verification" -ForegroundColor Cyan
Write-Host "  Target: $Base" -ForegroundColor Cyan
Write-Host "======================================================" -ForegroundColor Cyan
Write-Host ""

Write-Host "-- 1. Health" -ForegroundColor Yellow
Invoke-Check "GET /health/live"  "$Base/health/live"  200 | Out-Null
Invoke-Check "GET /health/ready" "$Base/health/ready" 200 | Out-Null

Write-Host "";
Write-Host "-- 2. Metrics" -ForegroundColor Yellow
Invoke-Check "GET /metrics" "$Base/metrics" 200 "# HELP" | Out-Null

Write-Host "";
Write-Host "-- 3. API" -ForegroundColor Yellow
Invoke-Check "GET /v1/tasks" "$Base/v1/tasks" 200 | Out-Null
Invoke-Check "GET /v1/stats" "$Base/v1/stats" 200 | Out-Null

Write-Host "";
Write-Host "-- 4. Frontend pages" -ForegroundColor Yellow
$pages = @("/", "/tasks/", "/terminal/", "/metrics/", "/analytics/", "/audit/")
foreach ($page in $pages) {
    Invoke-Check "GET $page" "$Base$page" 200 | Out-Null
}

Write-Host "";
Write-Host "======================================================" -ForegroundColor Cyan
$color = if ($Fail -eq 0) { "Green" } else { "Red" }
Write-Host "  Results: $Pass passed / $Fail failed / $Total total" -ForegroundColor $color
Write-Host "======================================================" -ForegroundColor Cyan
Write-Host ""

exit $(if ($Fail -eq 0) { 0 } else { 1 })
