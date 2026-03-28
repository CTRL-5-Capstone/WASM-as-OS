#!/usr/bin/env bash
# verify-railway.sh — Smoke-test a live WasmOS Railway deployment
#
# Usage:
#   ./scripts/verify-railway.sh https://wasmos-production.up.railway.app
#   BASE_URL=https://... ./scripts/verify-railway.sh
#
# Requirements: curl, jq (both available in Railway shell and most CI runners)
# Exit code: 0 = all checks passed, 1 = one or more checks failed

set -euo pipefail

BASE="${1:-${BASE_URL:-}}"
if [[ -z "$BASE" ]]; then
  echo "Usage: $0 <base-url>"
  echo "  e.g. $0 https://wasmos-production.up.railway.app"
  exit 1
fi
BASE="${BASE%/}"   # strip trailing slash

PASS=0
FAIL=0
TOTAL=0

# ── helpers ──────────────────────────────────────────────────────────────────
ok()   { echo "  ✓  $*"; ((PASS++)); ((TOTAL++)); }
fail() { echo "  ✗  $*"; ((FAIL++)); ((TOTAL++)); }

check_status() {
  local label="$1" url="$2" expected="$3"
  local actual
  actual=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 "$url")
  if [[ "$actual" == "$expected" ]]; then
    ok "$label → HTTP $actual"
  else
    fail "$label → expected HTTP $expected, got $actual  ($url)"
  fi
}

check_body_contains() {
  local label="$1" url="$2" pattern="$3"
  local body
  body=$(curl -s --max-time 10 "$url")
  if echo "$body" | grep -q "$pattern"; then
    ok "$label → body contains '$pattern'"
  else
    fail "$label → body missing '$pattern'  ($url)"
    echo "     response: $(echo "$body" | head -c 200)"
  fi
}

check_json_field() {
  local label="$1" url="$2" jq_expr="$3" expected="$4"
  local actual
  actual=$(curl -s --max-time 10 "$url" | jq -r "$jq_expr" 2>/dev/null || echo "__jq_error__")
  if [[ "$actual" == "$expected" ]]; then
    ok "$label → $jq_expr = '$actual'"
  else
    fail "$label → $jq_expr expected '$expected', got '$actual'"
  fi
}

# ── test suite ────────────────────────────────────────────────────────────────
echo ""
echo "══════════════════════════════════════════════════════"
echo "  WasmOS Railway Integration Verification"
echo "  Target: $BASE"
echo "══════════════════════════════════════════════════════"
echo ""

echo "── 1. Health ────────────────────────────────────────"
check_status      "GET /health/live"  "$BASE/health/live"  "200"
check_json_field  "  .status field"   "$BASE/health/live"  ".status" "ok"
check_status      "GET /health/ready" "$BASE/health/ready" "200"
check_json_field  "  .database field" "$BASE/health/ready" ".database" "connected"

echo ""
echo "── 2. Metrics (Prometheus) ──────────────────────────"
check_status       "GET /metrics"       "$BASE/metrics" "200"
check_body_contains "  Prometheus format" "$BASE/metrics" "# HELP"
check_body_contains "  wasmos metrics"    "$BASE/metrics" "wasmos_"

echo ""
echo "── 3. API — Tasks ───────────────────────────────────"
check_status "GET /v1/tasks"       "$BASE/v1/tasks"       "200"
check_status "GET /v1/stats"       "$BASE/v1/stats"       "200"
check_status "GET /v1/tenants"     "$BASE/v1/tenants"     "200"
check_status "GET /v1/audit"       "$BASE/v1/audit"       "200"
check_status "GET /v1/scheduler/tasks" "$BASE/v1/scheduler/tasks" "200"

echo ""
echo "── 4. API — Capability Tokens ───────────────────────"
check_status "GET /v1/capabilities" "$BASE/v1/capabilities" "200"

echo ""
echo "── 5. Frontend Pages ────────────────────────────────"
for page in "/" "/tasks/" "/terminal/" "/metrics/" "/analytics/" "/audit/" "/batch/" "/command-center/" "/inspect/" "/execute/" "/execution/" "/monitor/" "/rbac/" "/security/" "/snapshots/" "/tokens/" "/traces/" "/tests/" "/demo/"; do
  check_status "GET $page" "$BASE$page" "200"
done

echo ""
echo "── 6. Static Assets ─────────────────────────────────"
# Next.js static export emits /_next/static/... references in HTML
check_body_contains "HTML references /_next/static" "$BASE/" "/_next/static/"
# 404 for a made-up path (actix-files should still return frontend index for SPA)
actual_404=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 "$BASE/definitely-does-not-exist-xyz")
if [[ "$actual_404" == "200" || "$actual_404" == "404" ]]; then
  ok "Non-existent path → HTTP $actual_404 (acceptable)"
else
  fail "Non-existent path → unexpected HTTP $actual_404"
fi

echo ""
echo "── 7. WebSocket Handshake ───────────────────────────"
WS_URL="${BASE/https:/wss:}/ws"
WS_URL="${WS_URL/http:/ws:}"
ws_code=$(curl -s -o /dev/null -w "%{http_code}" --max-time 5 \
  -H "Upgrade: websocket" \
  -H "Connection: Upgrade" \
  -H "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==" \
  -H "Sec-WebSocket-Version: 13" \
  "$BASE/ws" || echo "000")
if [[ "$ws_code" == "101" || "$ws_code" == "200" ]]; then
  ok "WebSocket /ws → HTTP $ws_code (upgrade accepted)"
else
  fail "WebSocket /ws → HTTP $ws_code (expected 101)"
fi

echo ""
echo "══════════════════════════════════════════════════════"
echo "  Results: $PASS passed / $FAIL failed / $TOTAL total"
echo "══════════════════════════════════════════════════════"

[[ $FAIL -eq 0 ]]
