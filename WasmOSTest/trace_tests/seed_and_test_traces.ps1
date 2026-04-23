#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Quick trace test — seed traces and verify the frontend can load them.

.DESCRIPTION
    This script seeds synthetic traces into the running backend and then
    opens the Traces page in your browser. No WAT compilation needed.

.EXAMPLE
    .\seed_and_test_traces.ps1
    .\seed_and_test_traces.ps1 -Count 100
    .\seed_and_test_traces.ps1 -ApiUrl "http://localhost:8080"
#>
param(
    [int]$Count = 50,
    [string]$ApiUrl = "http://localhost:8080"
)

Write-Host "`n╔══════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║   WasmOS Trace Seeder & Test                     ║" -ForegroundColor Cyan
Write-Host "╚══════════════════════════════════════════════════╝" -ForegroundColor Cyan

# Health check
Write-Host "`n🔍 Checking backend at $ApiUrl..." -ForegroundColor Yellow
try {
    $health = Invoke-RestMethod -Uri "$ApiUrl/health/live" -Method Get -ErrorAction Stop
    Write-Host "✅ Backend is $($health.status)" -ForegroundColor Green
} catch {
    Write-Host "❌ Backend not reachable at $ApiUrl" -ForegroundColor Red
    Write-Host "   Start it first: cd wasmos; cargo run" -ForegroundColor Gray
    exit 1
}

# Seed traces
Write-Host "`n🌱 Seeding $Count synthetic traces..." -ForegroundColor Yellow
try {
    $body = @{ count = $Count } | ConvertTo-Json
    $result = Invoke-RestMethod -Uri "$ApiUrl/v1/traces/seed" `
        -Method Post `
        -ContentType "application/json" `
        -Body $body `
        -ErrorAction Stop
    Write-Host "✅ $($result.message)" -ForegroundColor Green
} catch {
    Write-Host "❌ Seed failed: $_" -ForegroundColor Red
    exit 1
}

# Verify traces
Write-Host "`n📊 Verifying traces..." -ForegroundColor Yellow
try {
    $traces = Invoke-RestMethod -Uri "$ApiUrl/v1/traces" -Method Get -ErrorAction Stop
    $total = $traces.Count
    $ok = ($traces | Where-Object { $_.success -eq $true }).Count
    $fail = ($traces | Where-Object { $_.success -eq $false }).Count

    Write-Host "   Total traces: $total" -ForegroundColor White
    Write-Host "   ✅ Successful: $ok" -ForegroundColor Green
    Write-Host "   ❌ Failed: $fail" -ForegroundColor Red
} catch {
    Write-Host "⚠️  Could not verify: $_" -ForegroundColor Yellow
}

# Live metrics
Write-Host "`n📈 Live Metrics:" -ForegroundColor Yellow
try {
    $m = Invoke-RestMethod -Uri "$ApiUrl/v1/traces/metrics/live" -Method Get -ErrorAction Stop
    Write-Host "   Success rate: $([math]::Round($m.success_rate * 100, 1))%" -ForegroundColor White
    Write-Host "   Error rate:   $([math]::Round($m.error_rate * 100, 1))%" -ForegroundColor White
    Write-Host "   p50: $([math]::Round($m.p50_us / 1000, 1))ms" -ForegroundColor White
    Write-Host "   p95: $([math]::Round($m.p95_us / 1000, 1))ms" -ForegroundColor White
    Write-Host "   p99: $([math]::Round($m.p99_us / 1000, 1))ms" -ForegroundColor White
    Write-Host "   Throughput: $([math]::Round($m.throughput_per_min, 1))/min" -ForegroundColor White
} catch {
    Write-Host "⚠️  Could not fetch metrics: $_" -ForegroundColor Yellow
}

Write-Host "`n👉 Open http://localhost:3001/traces to see the results" -ForegroundColor Cyan
Write-Host ""
