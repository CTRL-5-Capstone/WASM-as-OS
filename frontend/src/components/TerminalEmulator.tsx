"use client";

import { useState, useRef, useEffect, useCallback } from "react";
import {
  getTasks,
  getTask,
  startTask,
  stopTask,
  deleteTask,
  getStats,
  checkHealth,
  checkReady,
  uploadTask,
  readFileAsBytes,
  executeBatch,
  getPrometheusMetrics,
  getAdvancedMetrics,
  getExecutionReport,
  getImportStats,
  executeAdvanced,
  comparePerformance,
  getTestFiles,
  runTestFile,
  runAllTestFiles,
  getTaskSecurity,
  getTaskLogs,
  type Task,
  type TaskDetail,
} from "@/lib/api";
import { formatBytes, formatDuration, formatNumber } from "@/lib/utils";
import { Terminal as TermIcon, Maximize2, Minimize2, Copy } from "lucide-react";
import { useTerminal } from "@/lib/terminal-context";

// ─── Types ──────────────────────────────────────────────────────────

interface Line {
  type: "input" | "output" | "error" | "system" | "table";
  text: string;
}

// ─── Help text ──────────────────────────────────────────────────────

const HELP = `
╔══════════════════════════════════════════════════════════════════╗
║                    WASM-OS Terminal v3.0                        ║
╚══════════════════════════════════════════════════════════════════╝

TASK MANAGEMENT
  ls / list                 List all tasks (tabular)
  info <id|name>            Show full task detail + metrics + history
  upload <name>             Upload .wasm binary (opens file picker)
  delete / rm <id>          Delete a task permanently

EXECUTION
  start <id>                Execute a task via v1 API
  execute <id>              Same as start
  run <id|name>             Find by name or id and execute
  advanced <id>             Execute via v2 /execute/advanced (full metrics)
  batch <id1> <id2> ...     Batch-execute multiple tasks by id/name
  compare <id>              Run performance comparison via v2

SCHEDULING & CONTROL
  stop <id>                 Stop a running task
  status                    Show task counts by status
  schedule <id> [pri]       Schedule task with optional priority (0-255)
  queue                     Show scheduled task queue
  runqueue                  Execute all scheduled tasks in priority order

IMPORT MODULES
  imports                   Show registered import module statistics
  modules                   List available host import modules

INSPECTION & DATA
  inspect <id>              Deep inspection: metrics, history, analysis
  security <id>             Static binary security analysis (imports/exports/capabilities)
  logs <id>                 Show last execution log for a task
  opcode <id>               Show execution hotspots / opcode breakdown
  memory <id>               Show memory usage and peak stats
  report <exec-id>          Fetch v2 execution report
  metrics [id]              Prometheus metrics or per-task advanced metrics
  stdout <id>               Show stdout log from last execution

MONITORING
  stats                     Show system-wide statistics
  health                    Check backend liveness & readiness
  live                      Start live polling (updates every 3s)
  stoplive                  Stop live polling
  watch <id>                Watch a single task until completed

UTILITY
  export [json|csv]         Export task list to clipboard
  clear / cls               Clear terminal output
  history                   Show command history
  help                      Show this help message
  version                   Print version info

TEST SUITE
  testfiles                 List all available test .wasm files
  testrun <filename>        Run a single test file by name
  testall [category]        Run all test files (optionally filter by category)
`.trim();

// ─── Command aliases ────────────────────────────────────────────────

const ALIASES: Record<string, string> = {
  ls: "list",
  rm: "delete",
  exec: "execute",
  run: "execute",
  mod: "modules",
  mem: "memory",
  hist: "history",
  ver: "version",
  cls: "clear",
  tf: "testfiles",
  tr: "testrun",
  ta: "testall",
  sec: "security",
  log: "logs",
};

// ─── Component ──────────────────────────────────────────────────────

export default function TerminalEmulator() {
  const [lines, setLines] = useState<Line[]>([
    { type: "system", text: "WASM-OS Terminal v3.0  —  type 'help' for commands" },
  ]);
  const [input, setInput] = useState("");
  const [history, setHistory] = useState<string[]>([]);
  const [histIdx, setHistIdx] = useState(-1);
  const [busy, setBusy] = useState(false);
  const [fullscreen, setFullscreen] = useState(false);
  const [liveId, setLiveId] = useState<ReturnType<typeof setInterval> | null>(null);
  const bottomRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const fileRef = useRef<HTMLInputElement>(null);
  const pendingUploadName = useRef("");
  const scheduleQueue = useRef<{ taskId: string; priority: number; name: string }[]>([]);

  // Broadcast bridge: every push also sends to the global context
  const { push: ctxPush } = useTerminal();

  const scrollDown = useCallback(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, []);

  useEffect(scrollDown, [lines, scrollDown]);

  // Cleanup live polling on unmount
  useEffect(() => {
    return () => {
      if (liveId) clearInterval(liveId);
    };
  }, [liveId]);

  const push = useCallback(
    (type: Line["type"], text: string) => {
      setLines((prev) => [...prev, { type, text }]);
      // Broadcast to context subscribers (Monitor, Dashboard, etc.)
      ctxPush(type, text);
    },
    [ctxPush]
  );

  const pushTable = useCallback(
    (headers: string[], rows: string[][]) => {
      const widths = headers.map((h, i) =>
        Math.max(h.length, ...rows.map((r) => (r[i] || "").length))
      );
      const hr = widths.map((w) => "─".repeat(w + 2)).join("┼");
      const fmt = (cells: string[]) =>
        cells.map((c, i) => ` ${(c || "").padEnd(widths[i])} `).join("│");
      const out = [fmt(headers), hr, ...rows.map(fmt)].join("\n");
      push("table", out);
    },
    [push]
  );

  // ── Resolve task ID from id, partial id, or name ──

  const resolveTaskId = useCallback(async (query: string): Promise<string | null> => {
    const tasks = await getTasks();
    const t = tasks.find(
      (x) =>
        x.id === query ||
        x.id.startsWith(query) ||
        x.name.toLowerCase() === query.toLowerCase()
    );
    return t?.id ?? null;
  }, []);

  const findTask = useCallback(async (query: string): Promise<TaskDetail | null> => {
    const id = await resolveTaskId(query);
    if (!id) return null;
    try {
      return await getTask(id);
    } catch {
      return null;
    }
  }, [resolveTaskId]);

  // ── Execute command ──

  const exec = useCallback(
    async (raw: string) => {
      const trimmed = raw.trim();
      if (!trimmed) return;

      push("input", `$ ${trimmed}`);
      setHistory((h) => [trimmed, ...h.slice(0, 199)]);
      setHistIdx(-1);
      setBusy(true);

      const parts = trimmed.match(/(?:[^\s"]+|"[^"]*")+/g) || [];
      const rawCmd = (parts[0] || "").toLowerCase();
      const cmd = ALIASES[rawCmd] || rawCmd;
      const args = parts.slice(1).map((a) => a.replace(/^"|"$/g, ""));

      try {
        switch (cmd) {
          // ── HELP ──
          case "help":
            push("output", HELP);
            break;

          // ── CLEAR ──
          case "clear":
            setLines([]);
            break;

          // ── VERSION ──
          case "version":
            push("output", [
              "WASM-OS Terminal v3.0",
              "Frontend : Next.js 14 + React 18 + TypeScript 5",
              "Backend  : Rust Actix-web + Custom WASM Interpreter",
              "Database : PostgreSQL",
              "Runtime  : Custom bytecode engine with import modules",
            ].join("\n"));
            break;

          // ── LIST ──
          case "list": {
            const tasks: Task[] = await getTasks();
            if (tasks.length === 0) {
              push("system", "No tasks found. Use 'upload <name>' to add one.");
            } else {
              pushTable(
                ["ID", "Name", "Status", "Size", "Created"],
                tasks.map((t) => [
                  t.id.slice(0, 8),
                  t.name.length > 22 ? t.name.slice(0, 22) + "…" : t.name,
                  t.status,
                  formatBytes(t.file_size_bytes),
                  new Date(t.created_at).toLocaleDateString(),
                ])
              );
              push("system", `${tasks.length} task(s) total`);
            }
            break;
          }

          // ── INFO ──
          case "info": {
            if (!args[0]) { push("error", "Usage: info <task-id | task-name>"); break; }
            const detail = await findTask(args[0]);
            if (!detail) { push("error", `Task not found: ${args[0]}`); break; }
            const t = detail.task;
            push("output", [
              `Name      : ${t.name}`,
              `ID        : ${t.id}`,
              `Status    : ${t.status}`,
              `Path      : ${t.path}`,
              `Size      : ${formatBytes(t.file_size_bytes)}`,
              `Created   : ${new Date(t.created_at).toLocaleString()}`,
            ].join("\n"));
            if (detail.metrics) {
              const m = detail.metrics;
              push("output", [
                ``,
                `── Metrics ──`,
                `Total Runs    : ${m.total_runs}`,
                `Success       : ${m.successful_runs}`,
                `Failed        : ${m.failed_runs}`,
                `Success Rate  : ${m.total_runs > 0 ? Math.round(m.successful_runs / m.total_runs * 100) : 0}%`,
                `Avg Duration  : ${formatDuration(m.avg_duration_us)}`,
                `Instructions  : ${formatNumber(m.total_instructions)}`,
                `Syscalls      : ${formatNumber(m.total_syscalls)}`,
              ].join("\n"));
            }
            if (detail.recent_executions?.length) {
              push("system", `\n── Last ${detail.recent_executions.length} Execution(s) ──`);
              pushTable(
                ["Time", "OK?", "Duration", "Instr", "Syscalls", "Memory"],
                detail.recent_executions.map((h) => [
                  new Date(h.started_at).toLocaleTimeString(),
                  h.success ? "✓" : "✗",
                  h.duration_us ? formatDuration(h.duration_us) : "—",
                  formatNumber(h.instructions_executed),
                  formatNumber(h.syscalls_executed),
                  formatBytes(h.memory_used_bytes),
                ])
              );
            }
            break;
          }

          // ── INSPECT (deep analysis) ──
          case "inspect": {
            if (!args[0]) { push("error", "Usage: inspect <task-id>"); break; }
            push("system", `Inspecting ${args[0]}…`);
            const d = await findTask(args[0]);
            if (!d) { push("error", `Task not found: ${args[0]}`); break; }
            const t = d.task;
            const findings: string[] = [];
            // Size check
            if (t.file_size_bytes > 10_000_000) findings.push("⚠  Large binary (>10 MB)");
            else if (t.file_size_bytes > 1_000_000) findings.push("⚠  Medium binary (>1 MB)");
            else findings.push("✓  File size OK");
            // Failure rate
            if (d.metrics) {
              const rate = d.metrics.total_runs > 0 ? d.metrics.failed_runs / d.metrics.total_runs : 0;
              if (rate > 0.5) findings.push(`🔴 High failure rate (${Math.round(rate * 100)}%)`);
              else if (rate > 0.2) findings.push(`⚠  Elevated failure rate (${Math.round(rate * 100)}%)`);
              else findings.push("✓  Failure rate acceptable");
              // Instruction density
              if (d.metrics.total_instructions > 100_000_000) findings.push("⚠  Very high instruction count");
              else findings.push("✓  Instruction count normal");
              // Syscalls
              if (d.metrics.total_syscalls > 10_000) findings.push("⚠  High syscall usage");
              else findings.push("✓  Syscall usage normal");
              // Average duration
              if (d.metrics.avg_duration_us > 5_000_000) findings.push("⚠  Slow average execution (>5s)");
              else findings.push("✓  Execution speed OK");
            } else {
              findings.push("ℹ  No execution history available");
            }
            push("output", `── Security & Performance Analysis ──\nTask: "${t.name}" (${t.id.slice(0, 8)})\n\n${findings.join("\n")}`);
            break;
          }

          // ── UPLOAD ──
          case "upload": {
            const uploadName = args.join(" ") || "untitled";
            pendingUploadName.current = uploadName;
            push("system", `Select a .wasm or .wat file for "${uploadName}"…`);
            fileRef.current?.click();
            break;
          }

          // ── START / EXECUTE ──
          case "start":
          case "execute": {
            if (!args[0]) { push("error", `Usage: ${rawCmd} <task-id | task-name>`); break; }
            const id = await resolveTaskId(args[0]);
            if (!id) { push("error", `Task not found: ${args[0]}`); break; }
            push("system", `Executing task ${id.slice(0, 8)}…`);
            const result = await startTask(id);
            const out = [
              `Status       : ${result.success ? "✓ Success" : "✗ Failed"}`,
              `Duration     : ${formatDuration(result.duration_us)}`,
              `Instructions : ${formatNumber(result.instructions_executed)}`,
              `Syscalls     : ${formatNumber(result.syscalls_executed)}`,
              `Memory       : ${formatBytes(result.memory_used_bytes)}`,
            ];
            if (result.error) out.push(`Error        : ${result.error}`);
            if (result.return_value != null) out.push(`Return       : ${result.return_value}`);
            if (result.stdout_log?.length) out.push(`\n── stdout ──\n${result.stdout_log.join("\n")}`);
            push(result.success ? "output" : "error", out.join("\n"));
            break;
          }

          // ── ADVANCED EXECUTE (v2 API) ──
          case "advanced": {
            if (!args[0]) { push("error", "Usage: advanced <task-id>"); break; }
            const detail = await findTask(args[0]);
            if (!detail) { push("error", `Task not found: ${args[0]}`); break; }
            push("system", `Advanced execution of "${detail.task.name}" via v2 API…`);
            const adv = await executeAdvanced({ wasm_path: detail.task.path });
            const keys = ["execution_id", "success", "total_instructions", "total_syscalls",
              "duration_ms", "peak_memory_mb", "instructions_per_second"];
            const out: string[] = [];
            for (const k of keys) {
              if (adv[k] !== undefined) out.push(`${k.padEnd(24)}: ${adv[k]}`);
            }
            const hotspots = (adv.hotspots || []) as Array<Record<string, unknown>>;
            if (hotspots.length) {
              out.push(`\n── Hotspots ──`);
              hotspots.forEach((h) => out.push(`  ${h.opcode}  ${h.percentage}%`));
            }
            const anomalies = adv.performance_anomalies;
            if (Array.isArray(anomalies) && anomalies.length) {
              out.push(`\n── Performance Anomalies ──`);
              anomalies.forEach((a: string) => out.push(`  ⚠ ${a}`));
            }
            push("output", out.join("\n") || JSON.stringify(adv, null, 2));
            break;
          }

          // ── COMPARE (v2 performance comparison) ──
          case "compare": {
            if (args.length < 2) { push("error", "Usage: compare <baseline-id> <current-id>"); break; }
            const baseline = await findTask(args[0]);
            const current  = await findTask(args[1]);
            if (!baseline) { push("error", `Baseline task not found: ${args[0]}`); break; }
            if (!current)  { push("error", `Current task not found: ${args[1]}`); break; }
            push("system", `Comparing "${baseline.task.name}" vs "${current.task.name}"…`);
            const cmp = await comparePerformance({
              baseline_path: baseline.task.path,
              current_path: current.task.path,
            });
            push("output", JSON.stringify(cmp, null, 2));
            break;
          }

          // ── BATCH ──
          case "batch": {
            if (args.length === 0) { push("error", "Usage: batch <id1> <id2> …"); break; }
            const allT: Task[] = await getTasks();
            const paths = args.map((a) => {
              const found = allT.find(
                (t) => t.id === a || t.id.startsWith(a) || t.name.toLowerCase() === a.toLowerCase()
              );
              return found ? found.path : a;
            });
            push("system", `Executing batch of ${paths.length} module(s)…`);
            const br = await executeBatch({ wasm_paths: paths, continue_on_error: true });
            const rows = [
              ...br.results.map((r, i) => [
                String(i + 1),
                r.execution_id,
                "✓",
                r.duration_ms.toFixed(2) + "ms",
                formatNumber(r.instructions),
              ]),
              ...(br.errors ?? []).map((e, i) => [
                String(br.results.length + i + 1),
                e.path.split("/").pop() || e.path,
                "✗",
                "—",
                e.error,
              ]),
            ];
            pushTable(["#", "ID / Path", "Result", "Duration", "Instructions / Error"], rows);
            push("system", `Total: ${br.total_files}  Passed: ${br.successful}  Failed: ${br.failed}`);
            break;
          }

          // ── STOP ──
          case "stop": {
            if (!args[0]) { push("error", "Usage: stop <task-id>"); break; }
            const id = await resolveTaskId(args[0]);
            if (!id) { push("error", `Task not found: ${args[0]}`); break; }
            await stopTask(id);
            push("output", `Task ${id.slice(0, 8)} stopped.`);
            break;
          }

          // ── DELETE ──
          case "delete": {
            if (!args[0]) { push("error", "Usage: delete <task-id>"); break; }
            const id = await resolveTaskId(args[0]);
            if (!id) { push("error", `Task not found: ${args[0]}`); break; }
            await deleteTask(id);
            push("output", `Task ${id.slice(0, 8)} deleted.`);
            break;
          }

          // ── STATUS ──
          case "status": {
            const allTasks: Task[] = await getTasks();
            const counts: Record<string, number> = {};
            allTasks.forEach((t) => { counts[t.status] = (counts[t.status] || 0) + 1; });
            push("output", [
              `Total Tasks: ${allTasks.length}`,
              ...Object.entries(counts).map(([k, v]) => `  ${k.padEnd(14)} ${v}`),
            ].join("\n"));
            break;
          }

          // ── SCHEDULE ──
          case "schedule": {
            if (!args[0]) { push("error", "Usage: schedule <task-id> [priority 0-255]"); break; }
            const id = await resolveTaskId(args[0]);
            if (!id) { push("error", `Task not found: ${args[0]}`); break; }
            const pri = Math.min(255, Math.max(0, parseInt(args[1]) || 100));
            // Get name for display
            const tasks = await getTasks();
            const taskName = tasks.find((t) => t.id === id)?.name || id.slice(0, 8);
            scheduleQueue.current.push({ taskId: id, priority: pri, name: taskName });
            scheduleQueue.current.sort((a, b) => b.priority - a.priority);
            push("output", `Scheduled "${taskName}" with priority ${pri}. Queue size: ${scheduleQueue.current.length}`);
            break;
          }

          // ── QUEUE ──
          case "queue": {
            if (scheduleQueue.current.length === 0) {
              push("system", "Schedule queue is empty. Use 'schedule <id> [priority]' to add tasks.");
            } else {
              pushTable(
                ["#", "Task", "ID (short)", "Priority"],
                scheduleQueue.current.map((q, i) => [
                  String(i + 1),
                  q.name.length > 18 ? q.name.slice(0, 18) + "…" : q.name,
                  q.taskId.slice(0, 8),
                  String(q.priority),
                ])
              );
              push("system", `${scheduleQueue.current.length} task(s) in queue. Use 'runqueue' to execute.`);
            }
            break;
          }

          // ── RUNQUEUE — execute all scheduled tasks in priority order ──
          case "runqueue": {
            const q = scheduleQueue.current;
            if (q.length === 0) { push("system", "Queue is empty."); break; }
            push("system", `Executing ${q.length} scheduled task(s) in priority order…`);
            const results: string[] = [];
            let ok = 0;
            let fail = 0;
            for (const item of q) {
              try {
                const res = await startTask(item.taskId);
                if (res.success) { ok++; results.push(`  ✓ ${item.name} — ${formatDuration(res.duration_us)}`); }
                else { fail++; results.push(`  ✗ ${item.name} — ${res.error || "failed"}`); }
              } catch (e) {
                fail++;
                results.push(`  ✗ ${item.name} — ${e instanceof Error ? e.message : e}`);
              }
            }
            scheduleQueue.current = [];
            push("output", [
              `── Queue Execution Complete ──`,
              `Passed: ${ok}  Failed: ${fail}`,
              ...results,
            ].join("\n"));
            break;
          }

          // ── STATS ──
          case "stats": {
            const st = await getStats();
            push("output", [
              `── System Statistics ──`,
              `Total Tasks      : ${st.total_tasks}`,
              `Running          : ${st.running_tasks}`,
              `Failed           : ${st.failed_tasks}`,
              `Total Instr      : ${formatNumber(st.total_instructions)}`,
              `Total Syscalls   : ${formatNumber(st.total_syscalls)}`,
            ].join("\n"));
            break;
          }

          // ── HEALTH ──
          case "health": {
            const live = await checkHealth();
            const ready = await checkReady();
            push("output", [
              `Liveness  : ${live.status}   (${live.timestamp})`,
              `Readiness : ${ready.status}  — DB: ${ready.database || "unknown"}  (${ready.timestamp})`,
            ].join("\n"));
            break;
          }

          // ── METRICS ──
          case "metrics": {
            if (args[0]) {
              push("system", `Fetching advanced metrics for ${args[0]}…`);
              const id = await resolveTaskId(args[0]);
              if (!id) { push("error", `Task not found: ${args[0]}`); break; }
              const m = await getAdvancedMetrics(id);
              push("output", JSON.stringify(m, null, 2));
            } else {
              push("system", "Fetching Prometheus metrics…");
              const raw = await getPrometheusMetrics();
              const metricLines = raw.split("\n").filter((l: string) => !l.startsWith("#") && l.trim());
              if (metricLines.length > 0) {
                pushTable(
                  ["Metric", "Value"],
                  metricLines.map((l: string) => {
                    const p = l.split(/\s+/);
                    return [p[0] || l, p[1] || ""];
                  })
                );
              } else {
                push("system", "No metrics available yet.");
              }
            }
            break;
          }

          // ── REPORT ──
          case "report": {
            if (!args[0]) { push("error", "Usage: report <execution-id>"); break; }
            push("system", `Fetching report for ${args[0]}…`);
            const rpt = await getExecutionReport(args[0]);
            push("output", rpt ? JSON.stringify(rpt, null, 2) : "No report found.");
            break;
          }

          // ── IMPORTS ──
          case "imports": {
            push("system", "Fetching import statistics…");
            const imp = await getImportStats();
            push("output", JSON.stringify(imp, null, 2));
            break;
          }

          // ── MODULES ──
          case "modules": {
            push("output", [
              "── Available Host Import Modules ──",
              "",
              "  Module            Description",
              "  ────────────────  ──────────────────────────────────────",
              "  math              Arithmetic, trig, rounding, constants",
              "  string            Manipulation, encoding, parsing",
              "  array             Buffer operations, sorting, search",
              "  file              Sandboxed file I/O, path operations",
              "  serialization     JSON/binary encode & decode",
              "  host_log          Guest → host stdout logging",
              "  read_sensor       Sensor data input (stub)",
              "  send_alert        Alert dispatch (stub)",
            ].join("\n"));
            break;
          }

          // ── OPCODE / HOTSPOTS ──
          case "opcode": {
            if (!args[0]) { push("error", "Usage: opcode <task-id>"); break; }
            const d = await findTask(args[0]);
            if (!d) { push("error", `Task not found: ${args[0]}`); break; }
            push("system", `Running advanced execution for opcode analysis on "${d.task.name}"…`);
            try {
              const adv = await executeAdvanced({ wasm_path: d.task.path });
              const hotspots = (adv.hotspots || []) as Array<{ opcode: string; percentage: number }>;
              if (hotspots.length) {
                pushTable(
                  ["Opcode", "% of Total"],
                  hotspots.map((h) => [h.opcode, h.percentage.toFixed(2) + "%"])
                );
              } else {
                push("system", "No hotspot data available (module may be too small).");
              }
              push("output", `Total instructions: ${adv.total_instructions ?? "N/A"}\nDuration: ${adv.duration_ms ?? "N/A"}ms`);
            } catch (e) {
              push("error", `Opcode analysis failed: ${e instanceof Error ? e.message : e}`);
            }
            break;
          }

          // ── MEMORY ──
          case "memory": {
            if (!args[0]) { push("error", "Usage: memory <task-id>"); break; }
            const d = await findTask(args[0]);
            if (!d) { push("error", `Task not found: ${args[0]}`); break; }
            push("system", `Analyzing memory for "${d.task.name}"…`);
            try {
              const adv = await executeAdvanced({ wasm_path: d.task.path });
              push("output", [
                `── Memory Analysis ──`,
                `Peak Memory    : ${typeof adv.peak_memory_mb === "number" ? (adv.peak_memory_mb as number).toFixed(2) + " MB" : "N/A"}`,
                `Instructions   : ${formatNumber(Number(adv.total_instructions || 0))}`,
                `File Size      : ${formatBytes(d.task.file_size_bytes)}`,
              ].join("\n"));
            } catch (e) {
              push("error", `Memory analysis failed: ${e instanceof Error ? e.message : e}`);
            }
            break;
          }

          // ── STDOUT ──
          case "stdout": {
            if (!args[0]) { push("error", "Usage: stdout <task-id>"); break; }
            const id = await resolveTaskId(args[0]);
            if (!id) { push("error", `Task not found: ${args[0]}`); break; }
            push("system", `Executing task to capture stdout…`);
            const res = await startTask(id);
            if (res.stdout_log?.length) {
              push("output", "── stdout ──\n" + res.stdout_log.join("\n"));
            } else {
              push("system", "No stdout output produced.");
            }
            break;
          }

          // ── LIVE POLLING ──
          case "live": {
            if (liveId) { push("system", "Live polling already active. Use 'stoplive' to stop."); break; }
            push("system", "Live polling started (every 3s). Type 'stoplive' to stop.");
            const intervalId = setInterval(async () => {
              try {
                const st = await getStats();
                push("system",
                  `[LIVE ${new Date().toLocaleTimeString()}] Tasks: ${st.total_tasks} | Running: ${st.running_tasks} | Failed: ${st.failed_tasks} | Instr: ${formatNumber(st.total_instructions)}`
                );
              } catch { /* ignore polling errors */ }
            }, 3000);
            setLiveId(intervalId);
            break;
          }

          case "stoplive": {
            if (liveId) {
              clearInterval(liveId);
              setLiveId(null);
              push("system", "Live polling stopped.");
            } else {
              push("system", "No live polling active.");
            }
            break;
          }

          // ── WATCH ──
          case "watch": {
            if (!args[0]) { push("error", "Usage: watch <task-id>"); break; }
            const id = await resolveTaskId(args[0]);
            if (!id) { push("error", `Task not found: ${args[0]}`); break; }
            push("system", `Watching task ${id.slice(0, 8)}… (auto-stops on completion, max 2 min)`);
            let attempts = 0;
            const wId = setInterval(async () => {
              attempts++;
              try {
                const tasks = await getTasks();
                const t = tasks.find((x) => x.id === id);
                if (!t) { push("error", "Task disappeared."); clearInterval(wId); return; }
                push("system", `[WATCH] ${t.name}: ${t.status}`);
                if (["completed", "failed", "stopped"].includes(t.status) || attempts > 60) {
                  push("system", `Watch ended: ${t.status}`);
                  clearInterval(wId);
                }
              } catch { /* ignore */ }
            }, 2000);
            break;
          }

          // ── EXPORT ──
          case "export": {
            const fmt = args[0]?.toLowerCase() || "json";
            const tasks = await getTasks();
            let data: string;
            if (fmt === "csv") {
              data = "id,name,status,file_size_bytes,created_at\n" +
                tasks.map((t) => `${t.id},${t.name},${t.status},${t.file_size_bytes},${t.created_at}`).join("\n");
            } else {
              data = JSON.stringify(tasks, null, 2);
            }
            try {
              await navigator.clipboard.writeText(data);
              push("output", `Exported ${tasks.length} task(s) as ${fmt.toUpperCase()} to clipboard.`);
            } catch {
              push("output", data);
              push("system", "(Could not copy to clipboard — output printed above)");
            }
            break;
          }

          // ── HISTORY ──
          case "history": {
            if (history.length === 0) {
              push("system", "No command history.");
            } else {
              push("output", history.slice(0, 50).map((h, i) => `  ${String(i + 1).padStart(3)}. ${h}`).join("\n"));
            }
            break;
          }

          // ── TEST SUITE ──
          case "testfiles": {
            push("system", "Discovering test files...");
            const data = await getTestFiles();
            if (data.files.length === 0) {
              push("system", "No test files found in WasmOSTest or wasm_files directories.");
            } else {
              pushTable(
                ["File", "Category", "Source", "Size"],
                data.files.map((f) => [f.name, f.category, f.source, formatBytes(f.size_bytes)])
              );
              push("system", `${data.total} test file(s) discovered.`);
            }
            break;
          }

          case "testrun": {
            if (args.length === 0) { push("error", "Usage: testrun <filename>"); break; }
            const filename = args[0];
            push("system", `Running test file: ${filename}...`);
            const r = await runTestFile(filename);
            if (r.success) {
              push("output", [
                `✓ PASS: ${r.file}`,
                `  Duration: ${formatDuration(r.duration_us)}`,
                `  Instructions: ${formatNumber(r.instructions_executed)}`,
                `  Syscalls: ${formatNumber(r.syscalls_executed)}`,
                `  Memory: ${formatBytes(r.memory_used_bytes)}`,
                ...(r.return_value != null ? [`  Return: ${r.return_value}`] : []),
                ...(r.stdout_log.length > 0 ? [`  Stdout: ${r.stdout_log.join(", ")}`] : []),
              ].join("\n"));
            } else {
              push("error", [
                `✗ FAIL: ${r.file}`,
                `  Error: ${r.error || "Unknown"}`,
                `  Duration: ${formatDuration(r.duration_us)}`,
              ].join("\n"));
            }
            break;
          }

          case "testall": {
            const category = args[0] || undefined;
            push("system", `Running all test files${category ? ` (category: ${category})` : ""}...`);
            const data = await runAllTestFiles(category);
            for (const r of data.results) {
              push(
                r.success ? "output" : "error",
                `${r.success ? "✓" : "✗"} ${r.file.padEnd(25)} ${r.success ? "PASS" : "FAIL"}  ${formatDuration(r.duration_us).padStart(10)}  ${formatNumber(r.instructions_executed).padStart(8)} instr`
              );
            }
            push("system", `\nResults: ${data.passed}/${data.total} passed, ${data.failed} failed — ${formatDuration(data.total_duration_us)} total`);
            break;
          }

          // ── SECURITY ANALYSIS (static binary) ──
          case "security": {
            if (!args[0]) { push("error", "Usage: security <task-id | task-name>"); break; }
            const id = await resolveTaskId(args[0]);
            if (!id) { push("error", `Task not found: ${args[0]}`); break; }
            push("system", `Running static security analysis on ${id.slice(0, 8)}…`);
            try {
              const sec = await getTaskSecurity(id);
              const riskEmoji = sec.risk_level === "high" ? "🔴" : sec.risk_level === "medium" ? "🟡" : "🟢";
              const lines: string[] = [
                `── Security Report: ${sec.task_name} ──`,
                `Risk Level   : ${riskEmoji} ${sec.risk_level.toUpperCase()}`,
                `Summary      : ${sec.summary}`,
                ``,
                `── Capabilities ──`,
                ...sec.capabilities.map((c) => {
                  const icon = c.level === "severe" ? "🔴" : c.level === "warn" ? "⚠️" : "✅";
                  return `  ${icon} ${c.name}: ${c.description}`;
                }),
              ];
              if (sec.imports.length) {
                lines.push(``, `── Host Imports (${sec.imports.length}) ──`);
                sec.imports.forEach((i) => lines.push(`  ${i}`));
              }
              if (sec.exports.length) {
                lines.push(``, `── Exports (${sec.exports.length}) ──`);
                sec.exports.forEach((e) => lines.push(`  ${e}`));
              }
              push(sec.risk_level === "high" ? "error" : sec.risk_level === "medium" ? "output" : "output", lines.join("\n"));
            } catch (e) {
              push("error", `Security analysis failed: ${e instanceof Error ? e.message : e}`);
            }
            break;
          }

          // ── EXECUTION LOGS ──
          case "logs": {
            if (!args[0]) { push("error", "Usage: logs <task-id | task-name>"); break; }
            const id = await resolveTaskId(args[0]);
            if (!id) { push("error", `Task not found: ${args[0]}`); break; }
            push("system", `Fetching last execution log for ${id.slice(0, 8)}…`);
            try {
              const log = await getTaskLogs(id);
              if (!log.started_at) {
                push("system", `No executions recorded for "${log.task_name}" yet.`);
                break;
              }
              const out: string[] = [
                `── Last Execution Log: ${log.task_name} ──`,
                `Status       : ${log.success ? "✓ Success" : "✗ Failed"}`,
                `Started      : ${new Date(log.started_at).toLocaleString()}`,
                `Duration     : ${log.duration_us ? formatDuration(log.duration_us) : "—"}`,
                `Instructions : ${formatNumber(log.instructions_executed)}`,
                `Syscalls     : ${formatNumber(log.syscalls_executed)}`,
                `Memory       : ${formatBytes(log.memory_used_bytes)}`,
              ];
              if (log.error) out.push(`Error        : ${log.error}`);
              if (log.stdout_log?.length) {
                out.push(``, `── stdout ──`);
                log.stdout_log.forEach((l) => out.push(`  ${l}`));
              } else {
                out.push(``, `(no stdout output captured)`);
              }
              push(log.success ? "output" : "error", out.join("\n"));
            } catch (e) {
              push("error", `Failed to fetch logs: ${e instanceof Error ? e.message : e}`);
            }
            break;
          }

          // ── SECURITY ANALYSIS ──
          case "security": {
            if (!args[0]) { push("error", "Usage: security <task-id | task-name>"); break; }
            const id = await resolveTaskId(args[0]);
            if (!id) { push("error", `Task not found: ${args[0]}`); break; }
            push("system", `Running static security analysis on ${id.slice(0, 8)}\u2026`);
            try {
              const sec = await getTaskSecurity(id);
              const riskEmoji = sec.risk_level === "high" ? "\uD83D\uDD34" : sec.risk_level === "medium" ? "\uD83D\uDFE1" : "\uD83D\uDFE2";
              const lines: string[] = [
                `\u2500\u2500 Security Report: ${sec.task_name} \u2500\u2500`,
                `Risk Level   : ${riskEmoji} ${sec.risk_level.toUpperCase()}`,
                `Summary      : ${sec.summary}`,
                ``,
                `\u2500\u2500 Capabilities \u2500\u2500`,
                ...sec.capabilities.map((c) => {
                  const icon = c.level === "severe" ? "\uD83D\uDD34" : c.level === "warn" ? "\u26A0\uFE0F" : "\u2705";
                  return `  ${icon} ${c.name}: ${c.description}`;
                }),
              ];
              if (sec.imports.length) {
                lines.push(``, `\u2500\u2500 Host Imports (${sec.imports.length}) \u2500\u2500`);
                sec.imports.slice(0, 20).forEach((imp) => lines.push(`  ${imp}`));
                if (sec.imports.length > 20) lines.push(`  ... and ${sec.imports.length - 20} more`);
              }
              if (sec.exports.length) {
                lines.push(``, `\u2500\u2500 Exports (${sec.exports.length}) \u2500\u2500`);
                sec.exports.forEach((e) => lines.push(`  ${e}`));
              }
              push("output", lines.join("\n"));
            } catch (e) {
              push("error", `Security analysis failed: ${e instanceof Error ? e.message : e}`);
            }
            break;
          }

          // ── EXECUTION LOGS ──
          case "logs": {
            if (!args[0]) { push("error", "Usage: logs <task-id | task-name>"); break; }
            const id = await resolveTaskId(args[0]);
            if (!id) { push("error", `Task not found: ${args[0]}`); break; }
            push("system", `Fetching last execution log for ${id.slice(0, 8)}\u2026`);
            try {
              const log = await getTaskLogs(id);
              if (!log.started_at) {
                push("system", `No executions recorded for "${log.task_name}" yet.`);
                break;
              }
              const out: string[] = [
                `\u2500\u2500 Last Execution Log: ${log.task_name} \u2500\u2500`,
                `Status       : ${log.success ? "\u2713 Success" : "\u2717 Failed"}`,
                `Started      : ${new Date(log.started_at).toLocaleString()}`,
                `Duration     : ${log.duration_us ? formatDuration(log.duration_us) : "\u2014"}`,
                `Instructions : ${formatNumber(log.instructions_executed)}`,
                `Syscalls     : ${formatNumber(log.syscalls_executed)}`,
                `Memory       : ${formatBytes(log.memory_used_bytes)}`,
              ];
              if (log.error) out.push(`Error        : ${log.error}`);
              if (log.stdout_log?.length) {
                out.push(``, `\u2500\u2500 stdout \u2500\u2500`);
                log.stdout_log.forEach((l) => out.push(`  ${l}`));
              } else {
                out.push(``, `(no stdout output captured)`);
              }
              push(log.success ? "output" : "error", out.join("\n"));
            } catch (e) {
              push("error", `Failed to fetch logs: ${e instanceof Error ? e.message : e}`);
            }
            break;
          }

          // ── SECURITY ANALYSIS ──
          case "security": {
            if (!args[0]) { push("error", "Usage: security <task-id | task-name>"); break; }
            const id = await resolveTaskId(args[0]);
            if (!id) { push("error", `Task not found: ${args[0]}`); break; }
            push("system", `Running static security analysis on ${id.slice(0, 8)}…`);
            try {
              const sec = await getTaskSecurity(id);
              const riskEmoji = sec.risk_level === "high" ? "🔴" : sec.risk_level === "medium" ? "🟡" : "🟢";
              const lines: string[] = [
                `── Security Report: ${sec.task_name} ──`,
                `Risk Level   : ${riskEmoji} ${sec.risk_level.toUpperCase()}`,
                `Summary      : ${sec.summary}`,
                ``,
                `── Capabilities ──`,
                ...sec.capabilities.map((c) => {
                  const icon = c.level === "severe" ? "🔴" : c.level === "warn" ? "⚠️" : "✅";
                  return `  ${icon} ${c.name}: ${c.description}`;
                }),
              ];
              if (sec.imports.length) {
                lines.push(``, `── Host Imports (${sec.imports.length}) ──`);
                sec.imports.slice(0, 20).forEach((imp) => lines.push(`  ${imp}`));
                if (sec.imports.length > 20) lines.push(`  ... and ${sec.imports.length - 20} more`);
              }
              if (sec.exports.length) {
                lines.push(``, `── Exports (${sec.exports.length}) ──`);
                sec.exports.forEach((e) => lines.push(`  ${e}`));
              }
              push("output", lines.join("\n"));
            } catch (e) {
              push("error", `Security analysis failed: ${e instanceof Error ? e.message : e}`);
            }
            break;
          }

          // ── EXECUTION LOGS ──
          case "logs": {
            if (!args[0]) { push("error", "Usage: logs <task-id | task-name>"); break; }
            const id = await resolveTaskId(args[0]);
            if (!id) { push("error", `Task not found: ${args[0]}`); break; }
            push("system", `Fetching last execution log for ${id.slice(0, 8)}…`);
            try {
              const log = await getTaskLogs(id);
              if (!log.started_at) {
                push("system", `No executions recorded for "${log.task_name}" yet.`);
                break;
              }
              const out: string[] = [
                `── Last Execution Log: ${log.task_name} ──`,
                `Status       : ${log.success ? "✓ Success" : "✗ Failed"}`,
                `Started      : ${new Date(log.started_at).toLocaleString()}`,
                `Duration     : ${log.duration_us ? formatDuration(log.duration_us) : "—"}`,
                `Instructions : ${formatNumber(log.instructions_executed)}`,
                `Syscalls     : ${formatNumber(log.syscalls_executed)}`,
                `Memory       : ${formatBytes(log.memory_used_bytes)}`,
              ];
              if (log.error) out.push(`Error        : ${log.error}`);
              if (log.stdout_log?.length) {
                out.push(``, `── stdout ──`);
                log.stdout_log.forEach((l) => out.push(`  ${l}`));
              } else {
                out.push(``, `(no stdout output captured)`);
              }
              push(log.success ? "output" : "error", out.join("\n"));
            } catch (e) {
              push("error", `Failed to fetch logs: ${e instanceof Error ? e.message : e}`);
            }
            break;
          }

          // ── UNKNOWN ──
          default:
            push("error", `Unknown command: ${rawCmd}. Type 'help' for available commands.`);
        }
      } catch (err) {
        push("error", `Error: ${err instanceof Error ? err.message : String(err)}`);
      } finally {
        setBusy(false);
      }
    },
    [push, pushTable, history, liveId, resolveTaskId, findTask]
  );

  // ── File upload handler ──

  const handleFileUpload = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const files = e.target.files;
    if (!files || files.length === 0) return;
    setBusy(true);
    try {
      for (let i = 0; i < files.length; i++) {
        const f = files[i];
        const name = files.length === 1
          ? (pendingUploadName.current || f.name.replace(/\.(wasm|wat)$/, ""))
          : f.name.replace(/\.(wasm|wat)$/, "");
        push("system", `Uploading "${name}" (${formatBytes(f.size)})…`);
        const bytes = await readFileAsBytes(f);
        const task = await uploadTask(name, bytes);
        push("output", `✓ Uploaded: ${task.name} (${task.id.slice(0, 8)}…) — ${formatBytes(f.size)}`);
      }
    } catch (err) {
      push("error", `Upload failed: ${err instanceof Error ? err.message : err}`);
    } finally {
      setBusy(false);
      e.target.value = "";
    }
  };

  // ── Keyboard handler ──

  const onKey = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter" && !busy) {
      exec(input);
      setInput("");
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      if (history.length > 0) {
        const idx = Math.min(histIdx + 1, history.length - 1);
        setHistIdx(idx);
        setInput(history[idx]);
      }
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      if (histIdx > 0) {
        const idx = histIdx - 1;
        setHistIdx(idx);
        setInput(history[idx]);
      } else {
        setHistIdx(-1);
        setInput("");
      }
    } else if (e.key === "Tab") {
      e.preventDefault();
      const cmds = [
        "list", "info", "upload", "delete", "start", "execute", "run", "advanced",
        "batch", "compare", "stop", "status", "schedule", "queue", "runqueue",
        "stats", "health", "metrics", "report", "imports", "modules", "inspect",
        "opcode", "memory", "stdout", "live", "stoplive", "watch", "export",
        "clear", "history", "help", "version",
      ];
      const partial = input.toLowerCase().trim();
      if (!partial) {
        push("system", "  " + cmds.join("  "));
      } else {
        const match = cmds.filter((c) => c.startsWith(partial));
        if (match.length === 1) setInput(match[0] + " ");
        else if (match.length > 1) push("system", "  " + match.join("  "));
      }
    } else if (e.key === "l" && e.ctrlKey) {
      e.preventDefault();
      setLines([]);
    }
  };

  // ── Copy all output ──

  const copyOutput = () => {
    const text = lines.map((l) => (l.type === "input" ? l.text : "  " + l.text)).join("\n");
    navigator.clipboard.writeText(text).catch(() => {});
  };

  // ── Line colors ──

  const lineColor = (type: Line["type"]) => {
    switch (type) {
      case "input":  return "text-sky-400";
      case "error":  return "text-red-400";
      case "system": return "text-yellow-400";
      case "table":  return "text-emerald-300 font-medium";
      default:       return "text-slate-300";
    }
  };

  // ── Render ──

  return (
    <div
      className={`flex flex-col rounded-xl border border-slate-700/50 bg-slate-950 overflow-hidden transition-all ${
        fullscreen ? "fixed inset-0 z-50 rounded-none" : "h-full"
      }`}
    >
      {/* Header bar */}
      <div className="flex items-center gap-2 px-4 py-2 bg-slate-900 border-b border-slate-800">
        <TermIcon size={14} className="text-green-400" />
        <span className="text-xs text-slate-400 font-mono">wasm-os@shell</span>
        {busy && (
          <span className="ml-2 text-xs text-yellow-400 animate-pulse">⏳ working…</span>
        )}
        <div className="ml-auto flex items-center gap-2">
          <button onClick={copyOutput} className="text-slate-500 hover:text-slate-300 transition-colors" title="Copy output">
            <Copy size={13} />
          </button>
          <button onClick={() => setFullscreen(!fullscreen)} className="text-slate-500 hover:text-slate-300 transition-colors" title="Toggle fullscreen">
            {fullscreen ? <Minimize2 size={13} /> : <Maximize2 size={13} />}
          </button>
          <div className="flex gap-1.5 ml-2">
            <span className="w-3 h-3 rounded-full bg-red-500/60" />
            <span className="w-3 h-3 rounded-full bg-yellow-500/60" />
            <span className="w-3 h-3 rounded-full bg-green-500/60" />
          </div>
        </div>
      </div>

      {/* Output area */}
      <div
        className="flex-1 overflow-y-auto px-4 py-3 font-mono text-[13px] leading-relaxed"
        onClick={() => inputRef.current?.focus()}
      >
        {lines.map((line, i) => (
          <pre key={i} className={`whitespace-pre-wrap ${lineColor(line.type)}`}>
            {line.text}
          </pre>
        ))}
        <div ref={bottomRef} />
      </div>

      {/* Input */}
      <div className="flex items-center gap-2 px-4 py-2.5 bg-slate-900/80 border-t border-slate-800">
        <span className="text-green-400 text-sm font-mono shrink-0">
          {busy ? "⏳" : "$"}
        </span>
        <input
          ref={inputRef}
          type="text"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={onKey}
          placeholder={busy ? "Running…" : "Type a command… (Tab to autocomplete)"}
          disabled={busy}
          autoFocus
          spellCheck={false}
          className="flex-1 bg-transparent text-sm text-slate-200 font-mono focus:outline-none placeholder:text-slate-600 disabled:opacity-50"
        />
      </div>

      {/* Hidden file input for upload (multi-file) */}
      <input
        ref={fileRef}
        type="file"
        accept=".wasm,.wat"
        multiple
        className="hidden"
        onChange={handleFileUpload}
      />
    </div>
  );
}
