# Trace Test Cases — Frontend Testing Guide

This directory contains everything you need to test the **Traces** page (`/traces`)
on the WasmOS frontend. There are three ways to populate traces, from easiest to
most thorough.

---

## Quick Start (30 seconds)

If the backend is running on `localhost:8080`, just seed synthetic traces:

```powershell
# From the project root
.\WasmOSTest\trace_tests\seed_and_test_traces.ps1

# Or with curl
curl -X POST http://localhost:8080/v1/traces/seed -H "Content-Type: application/json" -d '{"count": 50}'
```

Then open **http://localhost:3001/traces** and you'll see traces immediately.

---

## Full Test (upload + execute + seed)

This uploads all 10 WAT modules, executes each one (generating real traces),
then seeds additional synthetic data for volume testing.

```powershell
cd frontend
npx tsx scripts/trace_test_runner.ts
```

Set `SEED_COUNT=100` to seed more traces, or `API_URL` to point at a different backend.

---

## WAT Test Files

Each `.wat` file is designed to exercise a specific trace scenario on the frontend.

| File | Duration | Success | What It Tests on `/traces` |
|------|----------|---------|----------------------------|
| `fast_add.wat` | < 1ms | Yes | Green "OK" row, tiny waterfall bar, valid TTFS badge |
| `slow_fibonacci.wat` | 200-700ms | Yes | Amber "Warning" heatmap cell, wide waterfall, p95/p99 impact |
| `divide_by_zero.wat` | < 0.1ms | No | Red heatmap cell, error string in detail, "FAIL" badge |
| `memory_heavy.wat` | 5-50ms | Yes | Memory tags in spans, env heatmap data |
| `nested_loops.wat` | 50-200ms | Yes | Mid-range waterfall bars, instruction count |
| `unreachable_trap.wat` | < 0.1ms | No | Different error string, Policy Violation badge |
| `stack_overflow.wat` | varies | No | Stack error type, forensic snapshot button |
| `multi_function.wat` | 3-20ms | Yes | Multiple waterfall spans, span ordering |
| `global_counter.wat` | 1-10ms | Yes | Validation span, medium duration |
| `bubble_sort_large.wat` | 100-500ms | Yes | High instructions, amber warning cells |

---

## What to Look For on the Traces Page

After seeding or executing, check each of these frontend features:

### 1. Live Metrics Bar
- **Success rate** should be ~70% (3 of 10 modules always fail)
- **p50** should be in the low-ms range
- **p95/p99** should be pulled up by `slow_fibonacci` and `bubble_sort_large`
- **Throughput** should show recent activity

### 2. Summary Cards
- **Total**: matches the number of traces you seeded
- **Success / Failed**: roughly 70/30 split
- **Scenarios**: generated deterministically from trace IDs
- **Violations**: subset of failed traces get "violation" scenario
- **Forensic Snaps**: 0 until you click the camera icon on a failed trace

### 3. Global Health Heatmap
- **Green cells**: fast successful traces (fast_add, global_counter)
- **Amber cells**: slow successful traces (slow_fibonacci, bubble_sort_large)
- **Red cells**: failed traces (divide_by_zero, unreachable_trap, stack_overflow)
- **Orange cells**: assertion failures (deterministic from trace ID)
- **Purple cells**: policy violations (deterministic from trace ID)
- Hover to see environment tooltips (sensors, env vars, vFS files)

### 4. Filter Pills
- **All**: shows everything
- **OK**: only green/amber traces
- **Failed**: only red traces
- **Assertion Fail**: only orange-badged traces
- **Policy Violation**: only purple-badged traces
- Counts should update next to each pill

### 5. Trace Row Details (click to expand)
- **Waterfall chart**: bars for Root, Load, Validate, Execute, Persist spans
- **+ env spans**: ABI read_sensor, vFS:read, assert: spans
- **Span details**: each span with timing, success icon, duration bar
- **Environment panel**: Mock Sensors, vFS Files, Env Vars sections
- **Scenario assertions**: check/expected/actual with PASSED/FAILED badges

### 6. Forensic Snapshots
- Click the **camera icon** on any failed trace (violation or assertion fail)
- Should trigger a toast "Forensic snapshot captured"
- Forensic Snapshots card appears below the heatmap

### 7. Clone to Test
- Expand any trace row, then click **"Clone to Test"**
- Edit the test name, then click **"Save Test Case"**
- Test Suite button appears in the header with count
- Toggle the suite panel to see all saved regression tests

### 8. Search & Pagination
- Search by trace ID (partial match), task name, scenario name, env var
- Page through results when > 50 traces

---

## Environment Pattern Analysis

The **Environment Pattern Analysis** card correlates env vars with outcomes:
- `LOG_LEVEL=DEBUG` vs `LOG_LEVEL=INFO` — check failure rate difference
- `ENABLE_ALERTS=true` — appears on ~40% of traces
- Bar = average duration, red overlay = failure rate
- More traces = more meaningful patterns

---

## Troubleshooting

| Problem | Fix |
|---------|-----|
| No traces appear | Backend not running, or seed endpoint wasn't called |
| "No traces found" with filter | Switch filter back to "All" |
| Heatmap shows all gray | Only 0 traces loaded — refresh or seed more |
| Live Metrics all zero | Need at least 1 trace — seed some first |
| WAT upload fails | Check that the backend has WAT compilation support (`wat` crate) |
| Seed returns 404 | Backend needs the `seed_traces` endpoint — rebuild after pulling latest |
