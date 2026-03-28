# verify-railway.ps1 - Smoke-test a live WasmOS Railway deployment (Windows PowerShell)
#
# Usage:
#   .\scripts\verify-railway.ps1 -BaseUrl https://wasmos-production.up.railway.app
#   $env:BASE_URL = "https://..."; .\scripts\verify-railway.ps1
#
# Requirements: PowerShell 5.1+ (Windows built-in) or PowerShell 7+
# Exit code: 0 = all passed, 1 = failures found

param(
    [string]$BaseUrl = $env:BASE_URL
)

if (-not $BaseUrl) {
    Write-Error "Provide the Railway URL: .\verify-railway.ps1 -BaseUrl https://your-app.up.railway.app"
    exit 1
}
$Base = $BaseUrl.TrimEnd('/')

$Pass  = 0
$Fail  = 0
$Total = 0

function Invoke-Check {
    param([string]$Label, [string]$Url, [int]$ExpectedStatus = 200, [string]$BodyContains = "")
    $script:Total++
    try {
        $resp = Invoke-WebRequest -Uri $Url -UseBasicParsing -TimeoutSec 15 -ErrorAction Stop
        $code = $resp.StatusCode
        if ($code -ne $ExpectedStatus) {
            Write-Host "  x  $Label -> expected HTTP $ExpectedStatus, got $code" -ForegroundColor Red
            $script:Fail++
            return $null
        }
        if ($BodyContains -and -not $resp.Content.Contains($BodyContains)) {
            Write-Host "  x  $Label -> body missing '$BodyContains'" -ForegroundColor Red
            $script:Fail++
            return $null
        }
        Write-Host "  v  $Label -> HTTP $code" -ForegroundColor Green
        $script:Pass++
        return $resp
    }
    catch {
        $code = 0
        if ($_.Exception -and $_.Exception.Response -and $_.Exception.Response.StatusCode) {
            $code = [int]$_.Exception.Response.StatusCode
        }
        if ($ExpectedStatus -eq $code) {
            Write-Host "  v  $Label -> HTTP $code" -ForegroundColor Green
            $script:Pass++
        } else {
            Write-Host "  x  $Label -> $($_.Exception.Message)" -ForegroundColor Red
            $script:Fail++
        }
        return $null
    }
}

function Test-JsonField {
    param([string]$Label, [string]$Url, [string]$Field, [string]$Expected)
    $script:Total++
    try {
        $json = Invoke-RestMethod -Uri $Url -UseBasicParsing -TimeoutSec 15 -ErrorAction Stop
        $actual = $json.$Field
        if ("$actual" -eq $Expected) {
            Write-Host "  v  $Label -> .$Field = '$actual'" -ForegroundColor Green
            $script:Pass++
        } else {
            Write-Host "  x  $Label -> .$Field expected '$Expected', got '$actual'" -ForegroundColor Red
            $script:Fail++
        }
    }
    catch {
        Write-Host "  x  $Label -> $($_.Exception.Message)" -ForegroundColor Red
        $script:Fail++
    }
}

Write-Host ""
Write-Host "======================================================" -ForegroundColor Cyan
Write-Host "  WasmOS Railway Integration Verification" -ForegroundColor Cyan
Write-Host "  Target: $Base" -ForegroundColor Cyan
Write-Host "======================================================" -ForegroundColor Cyan
Write-Host ""

# -- 1. Health -----------------------------------------------------------------
Write-Host "-- 1. Health" -ForegroundColor Yellow
Invoke-Check "GET /health/live"  "$Base/health/live"  200 | Out-Null
Test-JsonField "  .status field"  "$Base/health/live"  "status" "ok"
Invoke-Check "GET /health/ready" "$Base/health/ready" 200 | Out-Null
Test-JsonField "  .database field" "$Base/health/ready" "database" "connected"

# -- 2. Metrics ----------------------------------------------------------------
Write-Host ""
Write-Host "-- 2. Metrics (Prometheus)" -ForegroundColor Yellow
Invoke-Check "GET /metrics"           "$Base/metrics" 200 "# HELP" | Out-Null
Invoke-Check "  wasmos_ prefix check" "$Base/metrics" 200 "wasmos_" | Out-Null

# -- 3. API Endpoints ----------------------------------------------------------
Write-Host ""
Write-Host "-- 3. API - Core Endpoints" -ForegroundColor Yellow
Invoke-Check "GET /v1/tasks"            "$Base/v1/tasks"            200 | Out-Null
Invoke-Check "GET /v1/stats"            "$Base/v1/stats"            200 | Out-Null
Invoke-Check "GET /v1/tenants"          "$Base/v1/tenants"          200 | Out-Null
Invoke-Check "GET /v1/audit"            "$Base/v1/audit"            200 | Out-Null
Invoke-Check "GET /v1/scheduler/status" "$Base/v1/scheduler/status" 200 | Out-Null
Invoke-Check "GET /v1/traces"           "$Base/v1/traces"           200 | Out-Null

# -- 4. Frontend Pages ---------------------------------------------------------
Write-Host ""
Write-Host "-- 4. Frontend Pages" -ForegroundColor Yellow
$pages = @("/", "/tasks/", "/terminal/", "/metrics/", "/analytics/", "/audit/", "/batch/", "/command-center/", "/inspect/", "/execute/", "/execution/report/", "/monitor/", "/rbac/", "/security/", "/snapshots/", "/tokens/", "/traces/", "/tests/", "/demo/")
foreach ($page in $pages) {
    Invoke-Check "GET $page" "$Base$page" 200 | Out-Null
}

# -- 5. WASM Upload (smoke test with minimal WASM) -----------------------------
Write-Host ""
Write-Host "-- 5. WASM Upload Endpoint" -ForegroundColor Yellow
$Total++
# Backend expects JSON: { name: string, wasm_data: number[] }
$minWasm = @(0, 97, 115, 109, 1, 0, 0, 0)  # \0asm + version
try {
    $payload = @{ name = "smoke-test.wasm"; wasm_data = $minWasm } | ConvertTo-Json -Compress
    $uploadResp = Invoke-WebRequest -Uri "$Base/v1/tasks" -Method POST `
        -ContentType "application/json" `
        -Body $payload -UseBasicParsing -TimeoutSec 15 -ErrorAction Stop
    $code = $uploadResp.StatusCode
    if ($code -in 200,201,202,400,422) {
        Write-Host "  v  POST /v1/tasks (upload) -> HTTP $code (endpoint reachable)" -ForegroundColor Green
        $Pass++
    } else {
        Write-Host "  x  POST /v1/tasks (upload) -> unexpected HTTP $code" -ForegroundColor Red
        $Fail++
    }
} catch {
    $code = 0
    if ($_.Exception -and $_.Exception.Response -and $_.Exception.Response.StatusCode) {
        $code = [int]$_.Exception.Response.StatusCode
    }
    if ($code -in 400,422) {
        Write-Host "  v  POST /v1/tasks (upload) -> HTTP $code (endpoint reachable, rejected minimal WASM)" -ForegroundColor Green
        $Pass++
    } else {
        Write-Host "  x  POST /v1/tasks (upload) -> $($_.Exception.Message)" -ForegroundColor Red
        $Fail++
    }
}

# -- Summary -------------------------------------------------------------------
Write-Host ""
Write-Host "======================================================" -ForegroundColor Cyan
$color = if ($Fail -eq 0) { "Green" } else { "Red" }
Write-Host "  Results: $Pass passed / $Fail failed / $Total total" -ForegroundColor $color
Write-Host "======================================================" -ForegroundColor Cyan
Write-Host ""

exit $(if ($Fail -eq 0) { 0 } else { 1 })

