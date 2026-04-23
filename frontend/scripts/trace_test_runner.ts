/**
 * trace_test_runner.ts — Upload WAT files, execute them, and seed traces
 *
 * Run from the frontend directory:
 *   npx tsx scripts/trace_test_runner.ts
 *
 * Or with Node (after compiling):
 *   npx tsc --esModuleInterop --module commonjs scripts/trace_test_runner.ts
 *   node scripts/trace_test_runner.js
 *
 * What it does:
 *   1. Reads each .wat file from WasmOSTest/trace_tests/
 *   2. Uploads each as a task via POST /v1/tasks
 *   3. Starts each task via POST /v1/tasks/{id}/start (generates real traces)
 *   4. Optionally seeds extra synthetic traces via POST /v1/traces/seed
 *   5. Fetches traces and live metrics to verify everything landed
 *
 * Prerequisites:
 *   - Backend running on localhost:8080
 *   - WAT files present in WasmOSTest/trace_tests/
 */

const API_BASE = process.env.API_URL || "http://localhost:8080";

const TEST_FILES = [
  "fast_add.wat",
  "slow_fibonacci.wat",
  "divide_by_zero.wat",
  "memory_heavy.wat",
  "nested_loops.wat",
  "unreachable_trap.wat",
  "stack_overflow.wat",
  "multi_function.wat",
  "global_counter.wat",
  "bubble_sort_large.wat",
];

// ─── Helpers ────────────────────────────────────────────────────────────────

async function api<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`, {
    ...init,
    headers: { "Content-Type": "application/json", ...init?.headers },
  });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(`${res.status} ${res.statusText}: ${text}`);
  }
  return res.json() as Promise<T>;
}

function watToBytes(watText: string): number[] {
  return Array.from(new TextEncoder().encode(watText));
}

// ─── Main ───────────────────────────────────────────────────────────────────

async function main() {
  const fs = await import("fs");
  const path = await import("path");

  const traceTestDir = path.resolve(__dirname, "../../WasmOSTest/trace_tests");

  console.log("╔══════════════════════════════════════════════════╗");
  console.log("║   WasmOS Trace Test Runner                      ║");
  console.log("╚══════════════════════════════════════════════════╝");
  console.log(`\nAPI: ${API_BASE}`);
  console.log(`WAT dir: ${traceTestDir}\n`);

  // Step 0: Health check
  try {
    const health = await api<{ status: string }>("/health/live");
    console.log(`✅ Backend healthy: ${health.status}\n`);
  } catch (e) {
    console.error(`❌ Backend not reachable at ${API_BASE}`);
    console.error("   Start the backend first: cargo run (from wasmos/)");
    process.exit(1);
  }

  // Step 1: Upload and execute each WAT file
  console.log("── Step 1: Upload & Execute WAT Files ─────────────\n");

  const results: { name: string; taskId: string; success: boolean; error?: string }[] = [];

  for (const filename of TEST_FILES) {
    const filePath = path.join(traceTestDir, filename);
    if (!fs.existsSync(filePath)) {
      console.log(`  ⚠️  Skipping ${filename} — file not found`);
      continue;
    }

    const watText = fs.readFileSync(filePath, "utf-8");
    const taskName = filename.replace(".wat", "");

    try {
      // Upload
      const task = await api<{ id: string; name: string }>("/v1/tasks", {
        method: "POST",
        body: JSON.stringify({
          name: `trace-test-${taskName}`,
          wasm_data: watToBytes(watText),
        }),
      });
      console.log(`  📦 Uploaded: ${task.name} (${task.id.slice(0, 8)}…)`);

      // Execute
      try {
        const exec = await api<{ success: boolean; error?: string; duration_us: number }>(
          `/v1/tasks/${task.id}/start`,
          { method: "POST" }
        );
        const status = exec.success ? "✅" : "❌";
        console.log(`  ${status} Executed: ${taskName} — ${(exec.duration_us / 1000).toFixed(1)}ms${exec.error ? ` (${exec.error.slice(0, 60)})` : ""}`);
        results.push({ name: taskName, taskId: task.id, success: exec.success, error: exec.error });
      } catch (execErr: any) {
        console.log(`  ❌ Exec failed: ${taskName} — ${execErr.message.slice(0, 80)}`);
        results.push({ name: taskName, taskId: task.id, success: false, error: execErr.message });
      }
    } catch (uploadErr: any) {
      console.log(`  ❌ Upload failed: ${taskName} — ${uploadErr.message.slice(0, 80)}`);
    }

    // Small delay to avoid hammering the server
    await new Promise(r => setTimeout(r, 200));
  }

  // Step 2: Seed additional synthetic traces
  console.log("\n── Step 2: Seed Synthetic Traces ──────────────────\n");

  const seedCount = parseInt(process.env.SEED_COUNT || "50", 10);
  try {
    const seed = await api<{ seeded: number; message: string }>("/v1/traces/seed", {
      method: "POST",
      body: JSON.stringify({ count: seedCount }),
    });
    console.log(`  🌱 ${seed.message}`);
  } catch (e: any) {
    console.log(`  ⚠️  Seed failed: ${e.message}`);
  }

  // Step 3: Verify traces
  console.log("\n── Step 3: Verify Traces ──────────────────────────\n");

  try {
    const traces = await api<any[]>("/v1/traces");
    console.log(`  📊 Total traces in store: ${traces.length}`);

    const successful = traces.filter((t: any) => t.success).length;
    const failed = traces.filter((t: any) => !t.success).length;
    console.log(`     ✅ Successful: ${successful}`);
    console.log(`     ❌ Failed: ${failed}`);

    const metrics = await api<any>("/v1/traces/metrics/live");
    console.log(`\n  📈 Live Metrics:`);
    console.log(`     Success rate: ${(metrics.success_rate * 100).toFixed(1)}%`);
    console.log(`     Error rate:   ${(metrics.error_rate * 100).toFixed(1)}%`);
    console.log(`     p50: ${(metrics.p50_us / 1000).toFixed(1)}ms`);
    console.log(`     p95: ${(metrics.p95_us / 1000).toFixed(1)}ms`);
    console.log(`     p99: ${(metrics.p99_us / 1000).toFixed(1)}ms`);
    console.log(`     Throughput: ${metrics.throughput_per_min.toFixed(1)}/min`);
  } catch (e: any) {
    console.log(`  ⚠️  Trace verification failed: ${e.message}`);
  }

  // Summary
  console.log("\n── Summary ────────────────────────────────────────\n");
  const passed = results.filter(r => r.success).length;
  const failed = results.filter(r => !r.success).length;
  console.log(`  Executed: ${results.length} WAT modules`);
  console.log(`  Passed:   ${passed}`);
  console.log(`  Failed:   ${failed} (expected for divide_by_zero, unreachable_trap, stack_overflow)`);
  console.log(`\n  👉 Open http://localhost:3001/traces to see the results\n`);
}

main().catch(console.error);
