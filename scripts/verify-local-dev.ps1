# verify-local-dev.ps1 - End-to-end local integration smoke test (Windows PowerShell)
#
# Validates:
# - Backend health/metrics/API (direct)
# - Frontend pages (Next dev)
# - Frontend -> backend rewrites (proxy)
# - DB write/read via task creation
# - WebSocket handshake to backend (/ws)
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
    } catch {
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
    } catch {
        Write-Host "  x  $Label -> $($_.Exception.Message)" -ForegroundColor Red
        $script:Fail++
    }
}

function Test-WebSocket {
    param([string]$Label, [string]$WsUrl)
    $script:Total++
    try {
        Add-Type -AssemblyName System.Net.Http | Out-Null
        $uri = [Uri]$WsUrl
        $ws  = New-Object System.Net.WebSockets.ClientWebSocket
        $ws.Options.Proxy = $null

        $ct = New-Object System.Threading.CancellationTokenSource
        $ct.CancelAfter([TimeSpan]::FromSeconds(10))
        $ws.ConnectAsync($uri, $ct.Token).GetAwaiter().GetResult() | Out-Null

        if ($ws.State -ne [System.Net.WebSockets.WebSocketState]::Open) {
            throw "WebSocket not open (state=$($ws.State))"
        }

        $buffer = New-Object byte[] 8192
        $seg    = New-Object 'System.ArraySegment[byte]' (, $buffer)
        $recvCts = New-Object System.Threading.CancellationTokenSource
        $recvCts.CancelAfter([TimeSpan]::FromSeconds(3))
        $res = $ws.ReceiveAsync($seg, $recvCts.Token).GetAwaiter().GetResult()

        if ($res.MessageType -eq [System.Net.WebSockets.WebSocketMessageType]::Close) {
            throw "Server closed immediately"
        }

        $msg = [Text.Encoding]::UTF8.GetString($buffer, 0, $res.Count)
        if (-not $msg) {
            throw "No message received"
        }

        Write-Host "  v  $Label -> connected, first message: $msg" -ForegroundColor Green
        $script:Pass++

        # Some servers close quickly (or skip the close handshake) once we've received
        # the initial message. That's still a successful handshake for this smoke test.
        try {
            if ($ws.State -eq [System.Net.WebSockets.WebSocketState]::Open) {
                $ws.CloseAsync(
                    [System.Net.WebSockets.WebSocketCloseStatus]::NormalClosure,
                    "bye",
                    [Threading.CancellationToken]::None
                ).GetAwaiter().GetResult() | Out-Null
            }
        } catch {
            # Ignore close-handshake errors
        }
        try { $ws.Dispose() } catch {}
        return $true
    } catch {
        Write-Host "  x  $Label -> $($_.Exception.Message)" -ForegroundColor Red
        $script:Fail++
        return $false
    }
}

Write-Host ""
Write-Host "======================================================" -ForegroundColor Cyan
Write-Host "  WasmOS Local Integration Verification" -ForegroundColor Cyan
Write-Host "  Frontend: $frontend" -ForegroundColor Cyan
Write-Host "  Backend:  $backend" -ForegroundColor Cyan
Write-Host "======================================================" -ForegroundColor Cyan
Write-Host ""

# 1) Backend direct
Write-Host "-- 1. Backend (direct)" -ForegroundColor Yellow
Invoke-Check  "GET /health/live"   "$backend/health/live"   200 | Out-Null
Test-JsonField "  .status field"   "$backend/health/live"   "status"   "ok"
Invoke-Check  "GET /health/ready"  "$backend/health/ready"  200 | Out-Null
Test-JsonField "  .database field" "$backend/health/ready"  "database" "connected"
Invoke-Check  "GET /metrics"       "$backend/metrics"       200 "# HELP" | Out-Null
Invoke-Check  "GET /v1/tasks"      "$backend/v1/tasks"      200 | Out-Null
Invoke-Check  "GET /v1/stats"      "$backend/v1/stats"      200 | Out-Null

# 2) Frontend pages
Write-Host ""
Write-Host "-- 2. Frontend (Next dev)" -ForegroundColor Yellow
$pages = @("/", "/tasks/", "/terminal/", "/metrics/", "/analytics/", "/audit/")
foreach ($p in $pages) { Invoke-Check "GET $p" "$frontend$p" 200 | Out-Null }

# 3) Frontend -> backend proxy (rewrites)
Write-Host ""
Write-Host "-- 3. Frontend -> Backend Proxy" -ForegroundColor Yellow
# With trailingSlash:true, Next may 308 /v1/foo -> /v1/foo/. Use slashed URLs here.
Invoke-Check "GET /health/ready/ (via frontend)" "$frontend/health/ready/" 200 | Out-Null
Invoke-Check "GET /v1/tasks/ (via frontend)"     "$frontend/v1/tasks/"     200 | Out-Null

# 4) DB write/read through the frontend origin
Write-Host ""
Write-Host "-- 4. DB Write/Read" -ForegroundColor Yellow
$script:Total++
try {
    $name = "local-e2e-" + (Get-Date -Format "yyyyMMdd-HHmmss") + ".wasm"
    $payload = @{ name = $name; wasm_data = @(0,97,115,109,1,0,0,0) } | ConvertTo-Json -Compress
    $created = Invoke-RestMethod -UseBasicParsing -TimeoutSec 15 -Method Post -Uri "$frontend/v1/tasks/" -ContentType "application/json" -Body $payload
    $taskId  = $created.id

    $list = Invoke-RestMethod -UseBasicParsing -TimeoutSec 15 -Uri "$backend/v1/tasks"
    $found = $list | Where-Object { $_.id -eq $taskId } | Select-Object -First 1
    if (-not $found) { throw "Task id not found in backend list" }

    Write-Host "  v  Created task persisted -> id=$taskId status=$($found.status)" -ForegroundColor Green
    $script:Pass++
} catch {
    Write-Host "  x  DB write/read -> $($_.Exception.Message)" -ForegroundColor Red
    $script:Fail++
}

# 5) WebSocket
Write-Host ""
Write-Host "-- 5. WebSocket" -ForegroundColor Yellow
$wsUrl = $backend.Replace('https://','wss://').Replace('http://','ws://') + "/ws"
Test-WebSocket "WS /ws" $wsUrl | Out-Null

Write-Host ""
Write-Host "======================================================" -ForegroundColor Cyan
$color = if ($Fail -eq 0) { "Green" } else { "Red" }
Write-Host "  Results: $Pass passed / $Fail failed / $Total total" -ForegroundColor $color
Write-Host "======================================================" -ForegroundColor Cyan
Write-Host ""

exit $(if ($Fail -eq 0) { 0 } else { 1 })

