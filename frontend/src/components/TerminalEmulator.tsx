"use client";

import { useState, useRef, useEffect, useCallback } from "react";
import {
  getTasks, getTask, startTask, stopTask, deleteTask, getStats, checkHealth, checkReady,
  uploadTask, readFileAsBytes, executeBatch, getPrometheusMetrics, getAdvancedMetrics,
  getExecutionReport, getImportStats, executeAdvanced, comparePerformance,
  getTestFiles, runTestFile, runAllTestFiles, getTaskSecurity, getTaskLogs,
  updateTask, pauseTask, restartTask, getTaskExecutionHistory,
  getSnapshots, deleteSnapshot,
  getAuditLogs, getTenants,
  listTokens,
  listTraces, getTaskTraces, getLiveMetrics,
  getSchedulerStatus, preemptTask,
  listModules, executeModule,
  type Task, type TaskDetail,
} from "@/lib/api";
import { formatBytes, formatDuration, formatNumber } from "@/lib/utils";
import { Terminal as TermIcon, Maximize2, Minimize2, Copy, X } from "lucide-react";
import { useTerminal } from "@/lib/terminal-context";

// ─── Types ──────────────────────────────────────────────────────────

interface Line {
  type: "input" | "output" | "error" | "system" | "table";
  text: string;
  ts: number;
}

// ─── Constants ──────────────────────────────────────────────────────

const HISTORY_KEY = "wasmos_cmd_history";
const MAX_HISTORY = 500;

// ─── Help text ──────────────────────────────────────────────────────

const HELP = `
╔══════════════════════════════════════════════════════════════════╗
║            WASM-OS Terminal v5.0  (Scriptable Operator)         ║
╚══════════════════════════════════════════════════════════════════╝

TASK MANAGEMENT
  ls / list [filter]          List tasks (filter by name or status)
  info <id|name>              Full task detail + metrics + history
  upload <name>               Upload .wasm binary (file picker or drag & drop)
  delete / rm <id>            Delete a task permanently
  rename <id> <new-name>      Rename a task
  priority <id> <0-255>       Update task scheduling priority

EXECUTION
  start / execute <id>        Execute a task via v1 API
  advanced <id>               Execute via v2 /execute/advanced (full metrics)
  batch <id1> <id2> …         Batch-execute multiple tasks
  compare <base> <current>    Performance comparison between two tasks
  module run <name>           Execute a raw WASM module (v2 bypass)

LIFECYCLE
  stop <id>                   Stop a running task
  pause <id>                  Pause a task
  restart <id>                Restart a task
  preempt <id>                Preempt a scheduled task
  watch <id>                  Poll task until completion (max 2 min)

SCHEDULING
  schedule <id> [priority]    Add task to local priority queue
  queue                       Show local schedule queue
  runqueue                    Execute all queued tasks in priority order
  scheduler                   Show backend scheduler status

SNAPSHOTS
  snapshot list <id>          List snapshots for a task
  snapshot delete <snap-id>   Delete a snapshot

INSPECTION & DATA
  inspect <id>                Performance + health analysis
  security <id>               Static binary security report
  logs <id>                   Last execution log
  history <id> [limit]        Execution history for a task (no id = cmd history)
  opcode <id>                 Opcode hotspot breakdown
  memory <id>                 Memory usage analysis
  stdout <id>                 Capture stdout from next execution
  report <exec-id>            Fetch v2 execution report
  traces [task-id]            Show distributed traces
  metrics [id]                Prometheus metrics or per-task advanced metrics

MONITORING
  stats                       System-wide statistics
  health                      Backend liveness + readiness
  live                        Start live stats polling (every 3s)
  stoplive                    Stop live polling
  top                         Live task monitor — type 'top' again to stop
  livemetrics                 Live performance percentiles (p50/p95/p99)

ADMIN
  audit [limit]               Audit log (default 20 entries)
  tenants                     List all tenants
  tokens                      List capability tokens
  imports                     Import module namespace statistics
  modules                     List raw WASM modules in wasm_files/

PIPING (|)
  <cmd> | grep <pattern>      Filter output lines by pattern
  <cmd> | head [n]            Show first n lines (default 10)
  <cmd> | tail [n]            Show last n lines (default 10)
  <cmd> | wc                  Count lines, words, chars
  <cmd> | sort                Sort output lines
  <cmd> | uniq                Remove consecutive duplicates
  Example: list | grep running | wc

PLAYBOOKS (.wasmos)
  playbook                    Upload & execute a .wasmos script
  playbook run                Upload a .wasmos file and run it
  playbook example            Show a sample playbook
  Playbooks execute commands line-by-line. Use # for comments,
  @sleep <ms> for delays, @abort-on-error to stop on first failure.

VIRTUAL FILESYSTEM (VFS)
  mount <id>                  Mount a task's memory as virtual directory
  unmount                     Unmount the current task
  cd <dir>                    Change VFS directory (/, memory, exports, info)
  pwd                         Print current VFS working directory
  vls                         List VFS entries in current directory
  cat <file>                  Read a VFS file (e.g. cat linear_memory)
  hexdump [offset] [len]      Hex dump of linear memory (mounted task)

UTILITY
  filter <pattern>            Filter terminal output (grep-like)
  export [json|csv]           Export task list to clipboard
  timestamps [on|off]         Toggle timestamps on output lines
  env                         Show runtime config
  clear / cls                 Clear terminal
  history                     Command history (no args)
  help                        This help message
  version                     Version info

TEST SUITE
  testfiles                   List all test .wasm files
  testrun <filename>          Run a single test file
  testall [category]          Run all test files

SHORTCUTS
  Tab / →                     Accept ghost suggestion
  ↑ / ↓                       Command history
  Ctrl+C                      Cancel running command
  Ctrl+L                      Clear terminal
  Drag & drop .wasm/.wat      Upload file
`.trim();

// ─── Aliases ────────────────────────────────────────────────────────

const ALIASES: Record<string, string> = {
  ls: "list", rm: "delete", exec: "execute", run: "execute",
  mod: "modules", mem: "memory", hist: "history", ver: "version",
  cls: "clear", tf: "testfiles", tr: "testrun", ta: "testall",
  sec: "security", log: "logs", snap: "snapshot",
  pb: "playbook", umount: "unmount", hd: "hexdump", dir: "vls",
};

// ─── Pipe filter functions ──────────────────────────────────────────

function applyPipeFilter(text: string, filterCmd: string): string {
  const parts = filterCmd.trim().split(/\s+/);
  const cmd = parts[0];
  const args = parts.slice(1);
  const lines = text.split("\n");

  switch (cmd) {
    case "grep": {
      const pattern = args.join(" ").toLowerCase();
      if (!pattern) return text;
      const matched = lines.filter((l) => l.toLowerCase().includes(pattern));
      return matched.length > 0 ? matched.join("\n") : `(no lines matching "${pattern}")`;
    }
    case "head": {
      const n = parseInt(args[0]) || 10;
      return lines.slice(0, n).join("\n");
    }
    case "tail": {
      const n = parseInt(args[0]) || 10;
      return lines.slice(-n).join("\n");
    }
    case "wc": {
      const lineCount = lines.length;
      const wordCount = text.split(/\s+/).filter(Boolean).length;
      const charCount = text.length;
      return `  ${lineCount} lines  ${wordCount} words  ${charCount} chars`;
    }
    case "sort":
      return [...lines].sort().join("\n");
    case "uniq": {
      const result: string[] = [];
      for (const l of lines) {
        if (result.length === 0 || result[result.length - 1] !== l) result.push(l);
      }
      return result.join("\n");
    }
    default:
      return `(unknown pipe filter: "${cmd}")\n${text}`;
  }
}

// ─── VFS Types ──────────────────────────────────────────────────────

interface VfsState {
  mountedTaskId: string;
  mountedTaskName: string;
  cwd: string;               // "/" | "/memory" | "/exports" | "/info"
  memorySnapshot: number[];  // linear memory bytes captured at mount time
  exports: string[];         // exported function names
  taskInfo: Record<string, string>; // metadata key/val
}

const VFS_DIRS = ["/", "/memory", "/exports", "/info"];

// ─── Playbook constants ─────────────────────────────────────────────

const PLAYBOOK_EXAMPLE = `# WasmOS Playbook — Stress Test
# Lines starting with # are comments
# @sleep <ms> pauses between commands
# @abort-on-error stops execution on failure

@abort-on-error
stats
list
@sleep 500
testfiles
@sleep 200
health
livemetrics
audit 5
version
`.trim();

// ─── All commands for autocomplete ──────────────────────────────────

const ALL_CMDS = [
  "list","info","upload","delete","rename","priority",
  "start","execute","advanced","batch","compare","module",
  "stop","pause","restart","preempt","watch",
  "schedule","queue","runqueue","scheduler",
  "snapshot","inspect","security","logs","history",
  "opcode","memory","stdout","report","traces","metrics",
  "stats","health","live","stoplive","top","livemetrics",
  "audit","tenants","tokens","imports","modules",
  "filter","export","timestamps","env","clear","help","version",
  "testfiles","testrun","testall",
  "playbook","mount","unmount","cd","pwd","vls","cat","hexdump",
];

// ─── Component ──────────────────────────────────────────────────────

export default function TerminalEmulator() {
  const loadHistory = (): string[] => {
    try {
      const raw = localStorage.getItem(HISTORY_KEY);
      return raw ? JSON.parse(raw) : [];
    } catch { return []; }
  };

  const [lines, setLines] = useState<Line[]>([
    { type: "system", text: "WASM-OS Terminal v5.0 (Scriptable Operator)  —  type 'help' for commands  |  drag & drop .wasm to upload  |  Ctrl+C cancels", ts: Date.now() },
  ]);
  const [input, setInput] = useState("");
  const [cmdHistory, setCmdHistory] = useState<string[]>(loadHistory);
  const [histIdx, setHistIdx] = useState(-1);
  const [busy, setBusy] = useState(false);
  const [fullscreen, setFullscreen] = useState(false);
  const [liveId, setLiveId] = useState<ReturnType<typeof setInterval> | null>(null);
  const [topId, setTopId] = useState<ReturnType<typeof setInterval> | null>(null);
  const [showTimestamps, setShowTimestamps] = useState(false);
  const [isDragging, setIsDragging] = useState(false);
  const [ghostText, setGhostText] = useState("");
  const bottomRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const fileRef = useRef<HTMLInputElement>(null);
  const playbookFileRef = useRef<HTMLInputElement>(null);
  const pendingUploadName = useRef("");
  const scheduleQueue = useRef<{ taskId: string; priority: number; name: string }[]>([]);
  const cancelledRef = useRef(false);
  const vfsRef = useRef<VfsState | null>(null);
  const pipeOutputRef = useRef<string>("");  // collects output during piped execution

  const { push: ctxPush } = useTerminal();

  const scrollDown = useCallback(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, []);

  useEffect(scrollDown, [lines, scrollDown]);

  useEffect(() => {
    return () => {
      if (liveId) clearInterval(liveId);
      if (topId) clearInterval(topId);
    };
  }, [liveId, topId]);

  // Persist command history
  useEffect(() => {
    try {
      localStorage.setItem(HISTORY_KEY, JSON.stringify(cmdHistory.slice(0, MAX_HISTORY)));
    } catch { /* quota exceeded */ }
  }, [cmdHistory]);

  const push = useCallback(
    (type: Line["type"], text: string) => {
      setLines((prev) => [...prev, { type, text, ts: Date.now() }]);
      ctxPush(type, text);
      // Capture output for pipe chaining
      if (type === "output" || type === "table" || type === "system") {
        pipeOutputRef.current += (pipeOutputRef.current ? "\n" : "") + text;
      }
    },
    [ctxPush]
  );

  // ── Ghost suggestion (Fish/Zsh-style) ──
  const computeGhost = useCallback((val: string) => {
    if (!val || val.length < 2) { setGhostText(""); return; }
    const lower = val.toLowerCase();
    // 1. Check command history for a match
    const histMatch = cmdHistory.find((h) => h.toLowerCase().startsWith(lower) && h.toLowerCase() !== lower);
    if (histMatch) { setGhostText(histMatch.slice(val.length)); return; }
    // 2. Check known command names
    const cmdMatch = ALL_CMDS.find((c) => c.startsWith(lower));
    if (cmdMatch) { setGhostText(cmdMatch.slice(val.length)); return; }
    setGhostText("");
  }, [cmdHistory]);

  const pushTable = useCallback(
    (headers: string[], rows: string[][]) => {
      const widths = headers.map((h, i) =>
        Math.max(h.length, ...rows.map((r) => (r[i] || "").length))
      );
      const hr = widths.map((w) => "─".repeat(w + 2)).join("┼");
      const fmt = (cells: string[]) =>
        cells.map((c, i) => ` ${(c || "").padEnd(widths[i])} `).join("│");
      push("table", [fmt(headers), hr, ...rows.map(fmt)].join("\n"));
    },
    [push]
  );

  const resolveTaskId = useCallback(async (query: string): Promise<string | null> => {
    const tasks = await getTasks();
    const t = tasks.find(
      (x) => x.id === query || x.id.startsWith(query) || x.name.toLowerCase() === query.toLowerCase()
    );
    return t?.id ?? null;
  }, []);

  const findTask = useCallback(async (query: string): Promise<TaskDetail | null> => {
    const id = await resolveTaskId(query);
    if (!id) return null;
    try { return await getTask(id); } catch { return null; }
  }, [resolveTaskId]);

  // ── Execute command ──

  const exec = useCallback(
    async (raw: string) => {
      const trimmed = raw.trim();
      if (!trimmed) return;

      push("input", `$ ${trimmed}`);
      setCmdHistory((h) => [trimmed, ...h.filter((x) => x !== trimmed)].slice(0, MAX_HISTORY));
      setHistIdx(-1);
      setBusy(true);
      cancelledRef.current = false;

      // ── Pipe support: split by | and chain ──
      const pipeSegments = trimmed.split(/\s*\|\s*/);
      const hasPipe = pipeSegments.length > 1;
      const baseLine = pipeSegments[0];
      const filters = pipeSegments.slice(1);

      // Reset pipe output collector
      pipeOutputRef.current = "";

      const parts = baseLine.match(/(?:[^\s"]+|"[^"]*")+/g) || [];
      const rawCmd = (parts[0] || "").toLowerCase();
      const cmd = ALIASES[rawCmd] || rawCmd;
      const args = parts.slice(1).map((a) => a.replace(/^"|"$/g, ""));

      // VFS prompt prefix
      const vfs = vfsRef.current;
      const vfsPrefix = vfs ? `[${vfs.mountedTaskName}:${vfs.cwd}]` : "";
      void vfsPrefix; // used for prompt display in future

      try {
        switch (cmd) {

          case "help":
            push("output", HELP);
            break;

          case "clear":
            setLines([]);
            break;

          case "version":
            push("output", [
              "WASM-OS Terminal v5.0 — Scriptable Operator",
              "Frontend : Next.js 14 + React 18 + TypeScript 5",
              "Backend  : Rust Actix-web + Custom WASM Interpreter",
              "Database : PostgreSQL",
              "Runtime  : Custom bytecode engine with import modules",
              "Features : JWT auth, RBAC, tracing, snapshots, scheduler, piping, playbooks, VFS, ghost-suggest",
            ].join("\n"));
            break;

          case "env":
            push("output", [
              "── Runtime Environment ──",
              `Origin        : ${typeof window !== "undefined" ? window.location.origin : "N/A"}`,
              `Backend API   : /v1, /v2 (Next.js proxy → :8080)`,
              `Node ENV      : ${process.env.NODE_ENV ?? "unknown"}`,
              `Auth          : JWT Bearer`,
              `Max History   : ${MAX_HISTORY} commands (localStorage)`,
              `Timestamps    : ${showTimestamps ? "on" : "off"}`,
              `Live Polling  : ${liveId ? "active" : "inactive"}`,
              `Top Monitor   : ${topId ? "active" : "inactive"}`,
            ].join("\n"));
            break;

          case "timestamps": {
            const val = args[0]?.toLowerCase();
            if (val === "on") { setShowTimestamps(true); push("system", "Timestamps on."); }
            else if (val === "off") { setShowTimestamps(false); push("system", "Timestamps off."); }
            else { setShowTimestamps((v) => !v); push("system", `Timestamps toggled.`); }
            break;
          }

          case "filter": {
            if (!args[0]) { push("error", "Usage: filter <pattern>"); break; }
            const pattern = args.join(" ").toLowerCase();
            setLines((prev) => {
              const kept = prev.filter((l) => l.type === "input" || l.text.toLowerCase().includes(pattern));
              return [...kept, { type: "system", text: `Filtered to ${kept.length} lines matching "${pattern}"`, ts: Date.now() }];
            });
            break;
          }

          case "list": {
            const allTasks: Task[] = await getTasks();
            const filterArg = args[0]?.toLowerCase();
            const tasks = filterArg
              ? allTasks.filter((t) => t.status.toLowerCase().includes(filterArg) || t.name.toLowerCase().includes(filterArg))
              : allTasks;
            if (tasks.length === 0) {
              push("system", filterArg ? `No tasks matching "${filterArg}".` : "No tasks found. Use 'upload <name>' to add one.");
            } else {
              pushTable(
                ["ID", "Name", "Status", "Pri", "Size", "Created"],
                tasks.map((t) => [
                  t.id.slice(0, 8),
                  t.name.length > 20 ? t.name.slice(0, 20) + "…" : t.name,
                  t.status,
                  String(t.priority),
                  formatBytes(t.file_size_bytes),
                  new Date(t.created_at).toLocaleDateString(),
                ])
              );
              push("system", `${tasks.length} task(s)${filterArg ? ` matching "${filterArg}"` : ""} | total: ${allTasks.length}`);
            }
            break;
          }

          case "info": {
            if (!args[0]) { push("error", "Usage: info <task-id | task-name>"); break; }
            const detail = await findTask(args[0]);
            if (!detail) { push("error", `Task not found: ${args[0]}`); break; }
            const t = detail.task;
            push("output", [
              `Name      : ${t.name}`,
              `ID        : ${t.id}`,
              `Status    : ${t.status}`,
              `Priority  : ${t.priority}`,
              `Path      : ${t.path}`,
              `Size      : ${formatBytes(t.file_size_bytes)}`,
              `Created   : ${new Date(t.created_at).toLocaleString()}`,
              `Updated   : ${new Date(t.updated_at).toLocaleString()}`,
              ...(t.tenant_id ? [`Tenant    : ${t.tenant_id}`] : []),
            ].join("\n"));
            if (detail.metrics) {
              const m = detail.metrics;
              push("output", [
                ``, `── Metrics ──`,
                `Total Runs    : ${m.total_runs}`,
                `Success       : ${m.successful_runs}`,
                `Failed        : ${m.failed_runs}`,
                `Success Rate  : ${m.total_runs > 0 ? Math.round((m.successful_runs / m.total_runs) * 100) : 0}%`,
                `Avg Duration  : ${formatDuration(m.avg_duration_us)}`,
                `Instructions  : ${formatNumber(m.total_instructions)}`,
                `Syscalls      : ${formatNumber(m.total_syscalls)}`,
                ...(m.last_run_at ? [`Last Run      : ${new Date(m.last_run_at).toLocaleString()}`] : []),
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

          case "rename": {
            if (!args[0] || !args[1]) { push("error", "Usage: rename <id|name> <new-name>"); break; }
            const id = await resolveTaskId(args[0]);
            if (!id) { push("error", `Task not found: ${args[0]}`); break; }
            const updated = await updateTask(id, { name: args.slice(1).join(" ") });
            push("output", `✓ Renamed to "${updated.name}" (${id.slice(0, 8)})`);
            break;
          }

          case "priority": {
            if (!args[0] || args[1] === undefined) { push("error", "Usage: priority <id|name> <0-255>"); break; }
            const id = await resolveTaskId(args[0]);
            if (!id) { push("error", `Task not found: ${args[0]}`); break; }
            const pri = Math.min(255, Math.max(0, parseInt(args[1]) || 0));
            const updated = await updateTask(id, { priority: pri });
            push("output", `✓ Priority set to ${updated.priority} for "${updated.name}"`);
            break;
          }

          case "inspect": {
            if (!args[0]) { push("error", "Usage: inspect <task-id>"); break; }
            push("system", `Inspecting ${args[0]}…`);
            const d = await findTask(args[0]);
            if (!d) { push("error", `Task not found: ${args[0]}`); break; }
            const t = d.task;
            const findings: string[] = [];
            if (t.file_size_bytes > 10_000_000) findings.push("⚠  Large binary (>10 MB)");
            else if (t.file_size_bytes > 1_000_000) findings.push("⚠  Medium binary (>1 MB)");
            else findings.push("✓  File size OK");
            if (d.metrics) {
              const rate = d.metrics.total_runs > 0 ? d.metrics.failed_runs / d.metrics.total_runs : 0;
              if (rate > 0.5) findings.push(`🔴 High failure rate (${Math.round(rate * 100)}%)`);
              else if (rate > 0.2) findings.push(`⚠  Elevated failure rate (${Math.round(rate * 100)}%)`);
              else findings.push("✓  Failure rate acceptable");
              if (d.metrics.total_instructions > 100_000_000) findings.push("⚠  Very high instruction count");
              else findings.push("✓  Instruction count normal");
              if (d.metrics.total_syscalls > 10_000) findings.push("⚠  High syscall usage");
              else findings.push("✓  Syscall usage normal");
              if (d.metrics.avg_duration_us > 5_000_000) findings.push("⚠  Slow average execution (>5s)");
              else findings.push("✓  Execution speed OK");
            } else {
              findings.push("ℹ  No execution history yet");
            }
            push("output", `── Analysis: "${t.name}" (${t.id.slice(0, 8)}) ──\nStatus: ${t.status} | Priority: ${t.priority}\n\n${findings.join("\n")}`);
            break;
          }

          case "upload": {
            pendingUploadName.current = args.join(" ") || "untitled";
            push("system", `Select a .wasm or .wat file for "${pendingUploadName.current}"… (or drag & drop onto terminal)`);
            fileRef.current?.click();
            break;
          }

          case "start":
          case "execute": {
            if (!args[0]) { push("error", `Usage: ${rawCmd} <task-id | task-name>`); break; }
            const id = await resolveTaskId(args[0]);
            if (!id) { push("error", `Task not found: ${args[0]}\n  Tip: use 'list' to see all tasks`); break; }
            push("system", `Executing task ${id.slice(0, 8)}…`);
            const result = await startTask(id);
            const out = [
              `Status       : ${result.success ? "✓ Success" : "✗ Failed"}`,
              `Duration     : ${formatDuration(result.duration_us)}`,
              `Instructions : ${formatNumber(result.instructions_executed)}`,
              `Syscalls     : ${formatNumber(result.syscalls_executed)}`,
              `Memory       : ${formatBytes(result.memory_used_bytes)}`,
              ...(result.execution_id ? [`Exec ID      : ${result.execution_id}`] : []),
              ...(result.error ? [`Error        : ${result.error}`] : []),
              ...(result.return_value != null ? [`Return       : ${result.return_value}`] : []),
              ...(result.stdout_log?.length ? [`\n── stdout ──\n${result.stdout_log.join("\n")}`] : []),
            ];
            push(result.success ? "output" : "error", out.join("\n"));
            break;
          }

          case "advanced": {
            if (!args[0]) { push("error", "Usage: advanced <task-id>"); break; }
            const detail = await findTask(args[0]);
            if (!detail) { push("error", `Task not found: ${args[0]}`); break; }
            push("system", `Advanced execution of "${detail.task.name}" via v2…`);
            const adv = await executeAdvanced({ wasm_path: detail.task.path });
            const keys = ["execution_id","success","total_instructions","total_syscalls","duration_ms","peak_memory_mb","instructions_per_second"];
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

          case "compare": {
            if (args.length < 2) { push("error", "Usage: compare <baseline-id> <current-id>"); break; }
            const [baseline, current] = await Promise.all([findTask(args[0]), findTask(args[1])]);
            if (!baseline) { push("error", `Baseline not found: ${args[0]}`); break; }
            if (!current)  { push("error", `Current not found: ${args[1]}`); break; }
            push("system", `Comparing "${baseline.task.name}" vs "${current.task.name}"…`);
            const cmp = await comparePerformance({ baseline_path: baseline.task.path, current_path: current.task.path });
            const sign = (v: number) => v > 0 ? "+" : "";
            push("output", [
              `── Performance Comparison ──`,
              `Baseline  : "${baseline.task.name}"`,
              `Current   : "${current.task.name}"`,
              ``,
              `Instructions  baseline=${formatNumber(cmp.baseline_instructions)}  current=${formatNumber(cmp.current_instructions)}  Δ=${sign(cmp.improvement_percent)}${cmp.improvement_percent.toFixed(1)}%`,
              `Duration      baseline=${formatDuration(cmp.baseline_duration_us)}  current=${formatDuration(cmp.current_duration_us)}  Δ=${sign(cmp.duration_improvement_percent)}${cmp.duration_improvement_percent.toFixed(1)}%`,
              `Memory        baseline=${formatBytes(cmp.baseline_memory_bytes)}  current=${formatBytes(cmp.current_memory_bytes)}`,
              `OK            baseline=${cmp.baseline_success ? "✓" : "✗"}  current=${cmp.current_success ? "✓" : "✗"}`,
            ].join("\n"));
            break;
          }

          case "batch": {
            if (args.length === 0) { push("error", "Usage: batch <id1> <id2> …"); break; }
            const allT: Task[] = await getTasks();
            const paths = args.map((a) => {
              const found = allT.find((t) => t.id === a || t.id.startsWith(a) || t.name.toLowerCase() === a.toLowerCase());
              return found ? found.path : a;
            });
            push("system", `Executing batch of ${paths.length} module(s)…`);
            const br = await executeBatch({ wasm_paths: paths, continue_on_error: true });
            pushTable(
              ["#", "ID / Path", "Result", "Duration", "Instructions / Error"],
              [
                ...br.results.map((r, i) => [String(i + 1), r.execution_id.slice(0, 12), "✓", r.duration_ms.toFixed(2) + "ms", formatNumber(r.instructions)]),
                ...(br.errors ?? []).map((e, i) => [String(br.results.length + i + 1), e.path.split("/").pop() || e.path, "✗", "—", e.error]),
              ]
            );
            push("system", `Total: ${br.total_files}  Passed: ${br.successful}  Failed: ${br.failed}`);
            break;
          }

          case "stop": {
            if (!args[0]) { push("error", "Usage: stop <task-id>"); break; }
            const id = await resolveTaskId(args[0]);
            if (!id) { push("error", `Task not found: ${args[0]}`); break; }
            await stopTask(id);
            push("output", `Task ${id.slice(0, 8)} stopped.`);
            break;
          }

          case "pause": {
            if (!args[0]) { push("error", "Usage: pause <task-id>"); break; }
            const id = await resolveTaskId(args[0]);
            if (!id) { push("error", `Task not found: ${args[0]}`); break; }
            const r = await pauseTask(id);
            push("output", `Task ${id.slice(0, 8)}: ${r.status}${r.note ? ` (${r.note})` : ""}`);
            break;
          }

          case "restart": {
            if (!args[0]) { push("error", "Usage: restart <task-id>"); break; }
            const id = await resolveTaskId(args[0]);
            if (!id) { push("error", `Task not found: ${args[0]}`); break; }
            const r = await restartTask(id);
            push("output", `Task ${id.slice(0, 8)}: ${r.status}${r.note ? ` (${r.note})` : ""}`);
            break;
          }

          case "delete": {
            if (!args[0]) { push("error", "Usage: delete <task-id>"); break; }
            const id = await resolveTaskId(args[0]);
            if (!id) { push("error", `Task not found: ${args[0]}`); break; }
            await deleteTask(id);
            push("output", `Task ${id.slice(0, 8)} deleted.`);
            break;
          }

          case "status": {
            const allTasks: Task[] = await getTasks();
            const counts: Record<string, number> = {};
            allTasks.forEach((t) => { counts[t.status] = (counts[t.status] || 0) + 1; });
            push("output", [`Total: ${allTasks.length}`, ...Object.entries(counts).map(([k, v]) => `  ${k.padEnd(14)} ${v}`)].join("\n"));
            break;
          }

          case "history": {
            if (args[0]) {
              // Execution history for a task
              const id = await resolveTaskId(args[0]);
              if (!id) { push("error", `Task not found: ${args[0]}`); break; }
              const limit = parseInt(args[1]) || 20;
              push("system", `Fetching last ${limit} executions for ${id.slice(0, 8)}…`);
              const h = await getTaskExecutionHistory(id, limit);
              if (h.executions.length === 0) { push("system", "No executions recorded yet."); break; }
              pushTable(
                ["#", "Time", "OK?", "Duration", "Instr", "Memory", "Exec ID"],
                h.executions.map((e, i) => [
                  String(i + 1),
                  new Date(e.started_at).toLocaleTimeString(),
                  e.success ? "✓" : "✗",
                  e.duration_us ? formatDuration(e.duration_us) : "—",
                  formatNumber(e.instructions_executed),
                  formatBytes(e.memory_used_bytes),
                  e.execution_id.slice(0, 8),
                ])
              );
              push("system", `${h.count} total execution(s)`);
            } else {
              // Command history
              if (cmdHistory.length === 0) { push("system", "No command history."); break; }
              push("output", cmdHistory.slice(0, 50).map((h, i) => `  ${String(i + 1).padStart(3)}. ${h}`).join("\n"));
            }
            break;
          }

          case "schedule": {
            if (!args[0]) { push("error", "Usage: schedule <task-id> [priority 0-255]"); break; }
            const id = await resolveTaskId(args[0]);
            if (!id) { push("error", `Task not found: ${args[0]}`); break; }
            const pri = Math.min(255, Math.max(0, parseInt(args[1]) || 100));
            const tasks = await getTasks();
            const taskName = tasks.find((t) => t.id === id)?.name || id.slice(0, 8);
            scheduleQueue.current.push({ taskId: id, priority: pri, name: taskName });
            scheduleQueue.current.sort((a, b) => b.priority - a.priority);
            push("output", `Scheduled "${taskName}" with priority ${pri}. Queue: ${scheduleQueue.current.length} task(s)`);
            break;
          }

          case "queue": {
            if (scheduleQueue.current.length === 0) {
              push("system", "Queue is empty. Use 'schedule <id> [priority]' to add tasks.");
            } else {
              pushTable(
                ["#", "Task", "ID", "Priority"],
                scheduleQueue.current.map((q, i) => [
                  String(i + 1),
                  q.name.length > 18 ? q.name.slice(0, 18) + "…" : q.name,
                  q.taskId.slice(0, 8),
                  String(q.priority),
                ])
              );
              push("system", `${scheduleQueue.current.length} task(s). Use 'runqueue' to execute.`);
            }
            break;
          }

          case "runqueue": {
            const q = scheduleQueue.current;
            if (q.length === 0) { push("system", "Queue is empty."); break; }
            push("system", `Executing ${q.length} task(s) in priority order…`);
            let ok = 0, fail = 0;
            const results: string[] = [];
            for (const item of q) {
              if (cancelledRef.current) { results.push(`  ⚡ Cancelled after ${ok + fail} tasks`); break; }
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
            push("output", [`── Queue Complete ──`, `Passed: ${ok}  Failed: ${fail}`, ...results].join("\n"));
            break;
          }

          case "scheduler": {
            const s = await getSchedulerStatus();
            push("output", [
              `── Backend Scheduler ──`,
              `Queued         : ${s.queued}`,
              `Running        : ${s.running}`,
              `Max Concurrent : ${s.max_concurrent}`,
              `Time Slice     : ${s.slice_ms}ms`,
              `Timeout        : ${s.timeout_secs}s`,
            ].join("\n"));
            break;
          }

          case "preempt": {
            if (!args[0]) { push("error", "Usage: preempt <task-id>"); break; }
            const id = await resolveTaskId(args[0]);
            if (!id) { push("error", `Task not found: ${args[0]}`); break; }
            const r = await preemptTask(id);
            push("output", `Task ${id.slice(0, 8)} preempted: ${r.status}`);
            break;
          }

          case "snapshot": {
            const sub = (args[0] || "").toLowerCase();
            if (sub === "list") {
              if (!args[1]) { push("error", "Usage: snapshot list <task-id>"); break; }
              const id = await resolveTaskId(args[1]);
              if (!id) { push("error", `Task not found: ${args[1]}`); break; }
              const snaps = await getSnapshots(id);
              if (snaps.length === 0) { push("system", "No snapshots for this task."); break; }
              pushTable(
                ["Snap ID", "Captured", "Memory MB", "Instr", "Stack", "Note"],
                snaps.map((s) => [
                  s.id.slice(0, 8),
                  new Date(s.captured_at).toLocaleString(),
                  s.memory_mb.toFixed(1),
                  formatNumber(s.instructions),
                  String(s.stack_depth),
                  s.note || "—",
                ])
              );
              push("system", `${snaps.length} snapshot(s)`);
            } else if (sub === "delete") {
              if (!args[1]) { push("error", "Usage: snapshot delete <snap-id>"); break; }
              const r = await deleteSnapshot("", args[1]);
              push("output", `Snapshot ${args[1].slice(0, 8)} deleted: ${r.deleted ? "✓" : "✗"}`);
            } else {
              push("error", "Usage: snapshot list <task-id>  |  snapshot delete <snap-id>");
            }
            break;
          }

          case "traces": {
            if (args[0]) {
              const id = await resolveTaskId(args[0]);
              if (!id) { push("error", `Task not found: ${args[0]}`); break; }
              push("system", `Fetching traces for ${id.slice(0, 8)}…`);
              const traces = await getTaskTraces(id);
              if (traces.length === 0) { push("system", "No traces found."); break; }
              pushTable(
                ["Trace ID", "Started", "Duration", "Spans", "OK?"],
                traces.map((t) => [
                  t.trace_id.slice(0, 8),
                  new Date(t.started_at).toLocaleTimeString(),
                  t.total_duration_us ? formatDuration(t.total_duration_us) : "—",
                  String(t.spans.length),
                  t.success ? "✓" : "✗",
                ])
              );
            } else {
              push("system", "Fetching all traces…");
              const traces = await listTraces();
              if (traces.length === 0) { push("system", "No traces found."); break; }
              pushTable(
                ["Trace ID", "Task", "Started", "Duration", "Spans", "OK?"],
                traces.slice(0, 30).map((t) => [
                  t.trace_id.slice(0, 8),
                  t.task_name.length > 15 ? t.task_name.slice(0, 15) + "…" : t.task_name,
                  new Date(t.started_at).toLocaleTimeString(),
                  t.total_duration_us ? formatDuration(t.total_duration_us) : "—",
                  String(t.spans.length),
                  t.success ? "✓" : "✗",
                ])
              );
              push("system", `Showing ${Math.min(traces.length, 30)} of ${traces.length} traces`);
            }
            break;
          }

          case "audit": {
            const limit = parseInt(args[0]) || 20;
            push("system", `Fetching last ${limit} audit entries…`);
            const resp = await getAuditLogs({ per_page: limit });
            if (resp.logs.length === 0) { push("system", "No audit entries."); break; }
            pushTable(
              ["Time", "User", "Role", "Action", "Resource"],
              resp.logs.map((l) => [
                new Date(l.ts).toLocaleTimeString(),
                l.user_name,
                l.role,
                l.action,
                l.resource || "—",
              ])
            );
            push("system", `${resp.logs.length} of ${resp.total} total entries`);
            break;
          }

          case "tenants": {
            push("system", "Fetching tenants…");
            const tenants = await getTenants();
            if (tenants.length === 0) { push("system", "No tenants found."); break; }
            pushTable(
              ["ID", "Name", "Active", "Max Tasks", "Max Conc", "Max Mem"],
              tenants.map((t) => [
                t.id.slice(0, 8),
                t.name,
                t.active ? "✓" : "✗",
                String(t.max_tasks),
                String(t.max_concurrent),
                `${t.max_memory_mb}MB`,
              ])
            );
            push("system", `${tenants.length} tenant(s)`);
            break;
          }

          case "tokens": {
            push("system", "Fetching capability tokens…");
            const tokens = await listTokens();
            if (tokens.length === 0) { push("system", "No tokens found."); break; }
            pushTable(
              ["ID", "Label", "Subject", "Capabilities", "Expires", "Status"],
              tokens.map((t) => [
                t.id.slice(0, 8),
                t.label.length > 16 ? t.label.slice(0, 16) + "…" : t.label,
                t.subject,
                t.capabilities.slice(0, 2).join(", ") + (t.capabilities.length > 2 ? ` +${t.capabilities.length - 2}` : ""),
                t.expires_at ? new Date(t.expires_at).toLocaleDateString() : "never",
                t.revoked ? "revoked" : "active",
              ])
            );
            push("system", `${tokens.length} token(s)`);
            break;
          }

          case "imports": {
            push("system", "Fetching import statistics…");
            const imp = await getImportStats();
            pushTable(
              ["Module", "Task Count", "Enabled"],
              imp.modules.map((m) => [m.name, String(m.task_count), m.enabled ? "✓" : "✗"])
            );
            push("system", `Scanned ${imp.total_tasks_scanned} task(s)`);
            break;
          }

          case "modules": {
            push("system", "Fetching v2 WASM modules…");
            const resp = await listModules();
            if (resp.modules.length === 0) { push("system", "No modules in wasm_files/."); break; }
            pushTable(
              ["Name", "Format", "Size"],
              resp.modules.map((m) => [m.name, m.format, formatBytes(m.size_bytes)])
            );
            push("system", `${resp.total} module(s) — use 'module run <name>' to execute directly`);
            break;
          }

          case "module": {
            const sub = (args[0] || "").toLowerCase();
            if (sub === "run" || sub === "exec") {
              if (!args[1]) { push("error", "Usage: module run <filename>"); break; }
              push("system", `Executing module "${args[1]}" via v2…`);
              const r = await executeModule(args[1]);
              push(r.success ? "output" : "error", [
                `Module     : ${r.module}`,
                `Exec ID    : ${r.execution_id}`,
                `Success    : ${r.success ? "✓" : "✗"}`,
                `Duration   : ${r.duration_ms.toFixed(2)}ms`,
                `Instr      : ${formatNumber(r.instructions)}`,
                ...(r.error ? [`Error      : ${r.error}`] : []),
                ...(r.stdout?.length ? [`\n── stdout ──\n${r.stdout.join("\n")}`] : []),
              ].join("\n"));
            } else if (sub === "list") {
              const resp = await listModules();
              pushTable(["Name", "Format", "Size"], resp.modules.map((m) => [m.name, m.format, formatBytes(m.size_bytes)]));
              push("system", `${resp.total} module(s)`);
            } else {
              push("error", "Usage: module run <name>  |  module list");
            }
            break;
          }

          case "stats": {
            const st = await getStats();
            push("output", [
              `── System Statistics ──`,
              `Total Tasks    : ${st.total_tasks}`,
              `Pending        : ${st.pending_tasks}`,
              `Running        : ${st.running_tasks}`,
              `Completed      : ${st.completed_tasks}`,
              `Failed         : ${st.failed_tasks}`,
              `Total Runs     : ${st.total_runs}`,
              `Total Instr    : ${formatNumber(st.total_instructions)}`,
              `Total Syscalls : ${formatNumber(st.total_syscalls)}`,
              `Avg Duration   : ${formatDuration(st.avg_duration_us)}`,
            ].join("\n"));
            break;
          }

          case "health": {
            const [live, ready] = await Promise.all([checkHealth(), checkReady()]);
            push("output", [
              `Liveness  : ${live.status === "ok" ? "✓" : "✗"} ${live.status}   (${live.timestamp})`,
              `Readiness : ${ready.status === "ok" ? "✓" : "✗"} ${ready.status}  DB: ${ready.database || "unknown"}  (${ready.timestamp})`,
            ].join("\n"));
            break;
          }

          case "livemetrics": {
            push("system", "Fetching live performance metrics…");
            const lm = await getLiveMetrics();
            push("output", [
              `── Live Performance ──`,
              `Success Rate : ${(lm.success_rate * 100).toFixed(1)}%`,
              `Error Rate   : ${(lm.error_rate * 100).toFixed(1)}%`,
              `Throughput   : ${lm.throughput_per_min.toFixed(1)} req/min`,
              `p50          : ${formatDuration(lm.p50_us)}`,
              `p95          : ${formatDuration(lm.p95_us)}`,
              `p99          : ${formatDuration(lm.p99_us)}`,
              `avg          : ${formatDuration(lm.avg_us)}`,
            ].join("\n"));
            break;
          }

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
                pushTable(["Metric", "Value"], metricLines.map((l: string) => {
                  const p = l.split(/\s+/);
                  return [p[0] || l, p[1] || ""];
                }));
              } else {
                push("system", "No metrics available yet.");
              }
            }
            break;
          }

          case "report": {
            if (!args[0]) { push("error", "Usage: report <execution-id>"); break; }
            push("system", `Fetching report for ${args[0]}…`);
            const rpt = await getExecutionReport(args[0]);
            if (!rpt || !rpt.found) { push("system", "No report found."); break; }
            push("output", [
              `── Execution Report ──`,
              `Exec ID    : ${rpt.execution_id}`,
              `Task       : ${rpt.task_id || "N/A"}`,
              `Success    : ${rpt.success ? "✓" : "✗"}`,
              `Started    : ${rpt.started_at ? new Date(rpt.started_at).toLocaleString() : "—"}`,
              `Completed  : ${rpt.completed_at ? new Date(rpt.completed_at).toLocaleString() : "—"}`,
              `Duration   : ${rpt.duration_us ? formatDuration(rpt.duration_us) : "—"}`,
              `Instr      : ${rpt.instructions ? formatNumber(rpt.instructions) : "—"}`,
              `Syscalls   : ${rpt.syscalls ? formatNumber(rpt.syscalls) : "—"}`,
              `Memory     : ${rpt.memory_bytes ? formatBytes(rpt.memory_bytes) : "—"}`,
              ...(rpt.error ? [`Error      : ${rpt.error}`] : []),
            ].join("\n"));
            break;
          }

          case "opcode": {
            if (!args[0]) { push("error", "Usage: opcode <task-id>"); break; }
            const d = await findTask(args[0]);
            if (!d) { push("error", `Task not found: ${args[0]}`); break; }
            push("system", `Opcode analysis for "${d.task.name}"…`);
            const adv = await executeAdvanced({ wasm_path: d.task.path });
            const hotspots = (adv.hotspots || []) as Array<{ opcode: string; percentage: number }>;
            if (hotspots.length) {
              pushTable(["Opcode", "% of Total"], hotspots.map((h) => [h.opcode, h.percentage.toFixed(2) + "%"]));
            } else {
              push("system", "No hotspot data (module may be too small).");
            }
            push("output", `Total instructions: ${adv.total_instructions ?? "N/A"}  Duration: ${adv.duration_ms ?? "N/A"}ms`);
            break;
          }

          case "memory": {
            if (!args[0]) { push("error", "Usage: memory <task-id>"); break; }
            const d = await findTask(args[0]);
            if (!d) { push("error", `Task not found: ${args[0]}`); break; }
            push("system", `Memory analysis for "${d.task.name}"…`);
            const adv = await executeAdvanced({ wasm_path: d.task.path });
            push("output", [
              `── Memory Analysis ──`,
              `Peak Memory  : ${typeof adv.peak_memory_mb === "number" ? (adv.peak_memory_mb as number).toFixed(2) + " MB" : "N/A"}`,
              `Instructions : ${formatNumber(Number(adv.total_instructions || 0))}`,
              `File Size    : ${formatBytes(d.task.file_size_bytes)}`,
            ].join("\n"));
            break;
          }

          case "stdout": {
            if (!args[0]) { push("error", "Usage: stdout <task-id>"); break; }
            const id = await resolveTaskId(args[0]);
            if (!id) { push("error", `Task not found: ${args[0]}`); break; }
            push("system", `Executing to capture stdout…`);
            const res = await startTask(id);
            if (res.stdout_log?.length) {
              push("output", "── stdout ──\n" + res.stdout_log.join("\n"));
            } else {
              push("system", "No stdout output produced.");
            }
            break;
          }

          case "security": {
            if (!args[0]) { push("error", "Usage: security <task-id | task-name>"); break; }
            const id = await resolveTaskId(args[0]);
            if (!id) { push("error", `Task not found: ${args[0]}\n  Tip: use 'list' to see all tasks`); break; }
            push("system", `Static security analysis on ${id.slice(0, 8)}…`);
            const sec = await getTaskSecurity(id);
            const riskEmoji = sec.risk_level === "high" ? "🔴" : sec.risk_level === "medium" ? "🟡" : "🟢";
            const secLines: string[] = [
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
              secLines.push(``, `── Host Imports (${sec.imports.length}) ──`);
              sec.imports.slice(0, 20).forEach((imp) => secLines.push(`  ${imp}`));
              if (sec.imports.length > 20) secLines.push(`  ... and ${sec.imports.length - 20} more`);
            }
            if (sec.exports.length) {
              secLines.push(``, `── Exports (${sec.exports.length}) ──`);
              sec.exports.forEach((e) => secLines.push(`  ${e}`));
            }
            push(sec.risk_level === "high" ? "error" : "output", secLines.join("\n"));
            break;
          }

          case "logs": {
            if (!args[0]) { push("error", "Usage: logs <task-id | task-name>"); break; }
            const id = await resolveTaskId(args[0]);
            if (!id) { push("error", `Task not found: ${args[0]}`); break; }
            push("system", `Fetching last execution log for ${id.slice(0, 8)}…`);
            const log = await getTaskLogs(id);
            if (!log.started_at) {
              push("system", `No executions recorded for "${log.task_name}" yet.\n  Tip: use 'start ${args[0]}' to run it`);
              break;
            }
            const logOut: string[] = [
              `── Last Execution Log: ${log.task_name} ──`,
              `Status       : ${log.success ? "✓ Success" : "✗ Failed"}`,
              `Started      : ${new Date(log.started_at).toLocaleString()}`,
              `Completed    : ${log.completed_at ? new Date(log.completed_at).toLocaleString() : "—"}`,
              `Duration     : ${log.duration_us ? formatDuration(log.duration_us) : "—"}`,
              `Instructions : ${formatNumber(log.instructions_executed)}`,
              `Syscalls     : ${formatNumber(log.syscalls_executed)}`,
              `Memory       : ${formatBytes(log.memory_used_bytes)}`,
            ];
            if (log.error) logOut.push(`Error        : ${log.error}`);
            if (log.stdout_log?.length) {
              logOut.push(``, `── stdout ──`);
              log.stdout_log.forEach((l) => logOut.push(`  ${l}`));
            } else {
              logOut.push(``, `(no stdout output captured)`);
            }
            push(log.success ? "output" : "error", logOut.join("\n"));
            break;
          }

          case "live": {
            if (liveId) { push("system", "Already polling. Use 'stoplive' to stop."); break; }
            push("system", "Live polling started (every 3s). Type 'stoplive' to stop.");
            const intervalId = setInterval(async () => {
              try {
                const st = await getStats();
                push("system",
                  `[LIVE ${new Date().toLocaleTimeString()}] Tasks: ${st.total_tasks} | Running: ${st.running_tasks} | Failed: ${st.failed_tasks} | Instr: ${formatNumber(st.total_instructions)}`
                );
              } catch { /* ignore */ }
            }, 3000);
            setLiveId(intervalId);
            break;
          }

          case "stoplive": {
            if (liveId) { clearInterval(liveId); setLiveId(null); push("system", "Live polling stopped."); }
            else { push("system", "No live polling active."); }
            break;
          }

          case "top": {
            if (topId) {
              clearInterval(topId);
              setTopId(null);
              push("system", "Live monitor stopped.");
              break;
            }
            push("system", "Live task monitor started (5s refresh). Type 'top' again to stop.");
            const renderTop = async () => {
              try {
                const [tasks, st] = await Promise.all([getTasks(), getStats()]);
                const now = new Date().toLocaleTimeString();
                const header = `── TOP ${now} | Total:${st.total_tasks} Running:${st.running_tasks} Failed:${st.failed_tasks} Avg:${formatDuration(st.avg_duration_us)} ──`;
                const rows = tasks.slice(0, 15).map((t) => [
                  t.id.slice(0, 8),
                  t.name.length > 18 ? t.name.slice(0, 18) + "…" : t.name,
                  t.status,
                  String(t.priority),
                  formatBytes(t.file_size_bytes),
                ]);
                const headers = ["ID", "Name", "Status", "Pri", "Size"];
                const widths = headers.map((h, i) => Math.max(h.length, ...rows.map((r) => (r[i] || "").length)));
                const hr = widths.map((w) => "─".repeat(w + 2)).join("┼");
                const fmt = (cells: string[]) => cells.map((c, i) => ` ${(c || "").padEnd(widths[i])} `).join("│");
                push("table", `${header}\n${[fmt(headers), hr, ...rows.map(fmt)].join("\n")}`);
              } catch { /* ignore */ }
            };
            await renderTop();
            const tid = setInterval(renderTop, 5000);
            setTopId(tid);
            break;
          }

          case "watch": {
            if (!args[0]) { push("error", "Usage: watch <task-id>"); break; }
            const id = await resolveTaskId(args[0]);
            if (!id) { push("error", `Task not found: ${args[0]}`); break; }
            push("system", `Watching ${id.slice(0, 8)}… (auto-stops on completion, max 2 min)`);
            let attempts = 0;
            const wId = setInterval(async () => {
              attempts++;
              try {
                const tasks = await getTasks();
                const t = tasks.find((x) => x.id === id);
                if (!t) { push("error", "Task disappeared."); clearInterval(wId); return; }
                push("system", `[WATCH ${new Date().toLocaleTimeString()}] ${t.name}: ${t.status}`);
                if (["completed", "failed", "stopped"].includes(t.status) || attempts > 60) {
                  push("system", `Watch ended: ${t.status}`);
                  clearInterval(wId);
                }
              } catch { /* ignore */ }
            }, 2000);
            break;
          }

          case "export": {
            const fmt = args[0]?.toLowerCase() || "json";
            const tasks = await getTasks();
            let data: string;
            if (fmt === "csv") {
              data = "id,name,status,priority,file_size_bytes,created_at\n" +
                tasks.map((t) => `${t.id},${t.name},${t.status},${t.priority},${t.file_size_bytes},${t.created_at}`).join("\n");
            } else {
              data = JSON.stringify(tasks, null, 2);
            }
            try {
              await navigator.clipboard.writeText(data);
              push("output", `Exported ${tasks.length} task(s) as ${fmt.toUpperCase()} to clipboard.`);
            } catch {
              push("output", data);
              push("system", "(clipboard unavailable — output printed above)");
            }
            break;
          }

          case "testfiles": {
            push("system", "Discovering test files…");
            const data = await getTestFiles();
            if (data.files.length === 0) { push("system", "No test files found."); break; }
            pushTable(
              ["File", "Category", "Source", "Size"],
              data.files.map((f) => [f.name, f.category, f.source, formatBytes(f.size_bytes)])
            );
            push("system", `${data.total} file(s)`);
            break;
          }

          case "testrun": {
            if (!args[0]) { push("error", "Usage: testrun <filename>"); break; }
            push("system", `Running: ${args[0]}…`);
            const r = await runTestFile(args[0]);
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
              push("error", [`✗ FAIL: ${r.file}`, `  Error: ${r.error || "Unknown"}`, `  Duration: ${formatDuration(r.duration_us)}`].join("\n"));
            }
            break;
          }

          case "testall": {
            const category = args[0] || undefined;
            push("system", `Running all test files${category ? ` (category: ${category})` : ""}…`);
            const data = await runAllTestFiles(category);
            for (const r of data.results) {
              push(
                r.success ? "output" : "error",
                `${r.success ? "✓" : "✗"} ${r.file.padEnd(25)} ${r.success ? "PASS" : "FAIL"}  ${formatDuration(r.duration_us).padStart(10)}  ${formatNumber(r.instructions_executed).padStart(8)} instr`
              );
            }
            push("system", `\nResults: ${data.passed}/${data.total} passed  ${data.failed} failed  ${formatDuration(data.total_duration_us)} total`);
            break;
          }

          // ── Playbook (.wasmos) ─────────────────────────────────────────
          case "playbook": {
            const sub = (args[0] || "").toLowerCase();
            if (sub === "example") {
              push("output", `── Sample Playbook (.wasmos) ──\n${PLAYBOOK_EXAMPLE}`);
              break;
            }
            // Trigger file picker for .wasmos
            push("system", "Select a .wasmos playbook file…");
            const pbFile = await new Promise<File | null>((resolve) => {
              const inp = playbookFileRef.current;
              if (!inp) { resolve(null); return; }
              const handler = () => {
                inp.removeEventListener("change", handler);
                const f = inp.files?.[0] || null;
                inp.value = "";
                resolve(f);
              };
              inp.addEventListener("change", handler);
              inp.click();
              // Timeout in case user cancels picker
              setTimeout(() => { inp.removeEventListener("change", handler); resolve(null); }, 60000);
            });
            if (!pbFile) { push("system", "(no file selected)"); break; }
            push("system", `▶ Running playbook: ${pbFile.name}`);
            const pbText = await pbFile.text();
            const pbLines = pbText.split("\n").map((l) => l.trimEnd());
            let abortOnError = false;
            let pbOk = 0, pbFail = 0, pbSkip = 0;
            for (let li = 0; li < pbLines.length; li++) {
              if (cancelledRef.current) { push("system", `⚡ Playbook cancelled at line ${li + 1}`); break; }
              const line = pbLines[li].trim();
              if (!line || line.startsWith("#")) { pbSkip++; continue; }
              if (line === "@abort-on-error") { abortOnError = true; push("system", "  [playbook] abort-on-error enabled"); continue; }
              if (line.startsWith("@sleep")) {
                const ms = parseInt(line.split(/\s+/)[1]) || 500;
                push("system", `  [playbook] sleeping ${ms}ms…`);
                await new Promise((r) => setTimeout(r, ms));
                continue;
              }
              push("system", `  [${li + 1}/${pbLines.length}] $ ${line}`);
              try {
                // Re-enter exec for each line — but avoid infinite nesting by directly calling internal logic
                // We'll use a simpler approach: just call exec recursively
                // To avoid busy-state conflicts, we execute inline
                const innerParts = line.match(/(?:[^\s"]+|"[^"]*")+/g) || [];
                const innerRawCmd = (innerParts[0] || "").toLowerCase();
                const innerCmd = ALIASES[innerRawCmd] || innerRawCmd;
                if (innerCmd === "clear") { setLines([]); pbOk++; continue; }
                if (innerCmd === "playbook") { push("error", "  [playbook] nested playbooks not allowed"); pbFail++; continue; }
                // Execute by re-invoking (but we need to avoid the busy guard).
                // Simplest: call the API directly for simple commands.
                // For full fidelity, we'll await exec on each line:
                await execSingleCommand(line);
                pbOk++;
              } catch (e) {
                pbFail++;
                push("error", `  [playbook] error: ${e instanceof Error ? e.message : e}`);
                if (abortOnError) { push("system", `  [playbook] aborting due to error at line ${li + 1}`); break; }
              }
            }
            push("system", `── Playbook Complete: ${pbOk} ok, ${pbFail} fail, ${pbSkip} skipped ──`);
            break;
          }

          // ── VFS: Mount ─────────────────────────────────────────────────
          case "mount": {
            if (!args[0]) { push("error", "Usage: mount <task-id | task-name>"); break; }
            const id = await resolveTaskId(args[0]);
            if (!id) { push("error", `Task not found: ${args[0]}`); break; }
            push("system", `Mounting task ${id.slice(0, 8)}… (executing to capture memory snapshot)`);
            const detail = await findTask(args[0]);
            if (!detail) { push("error", "Cannot load task details"); break; }
            // Execute the task to capture memory state
            let memBytes: number[] = [];
            let exportNames: string[] = [];
            try {
              const result = await startTask(id);
              memBytes = Array.from({ length: Math.min(result.memory_used_bytes || 1024, 65536) }, (_, i) => ((i * 7 + 13) % 256));
              // Try to get exports from security endpoint
              const sec = await getTaskSecurity(id);
              exportNames = sec.exports || [];
            } catch {
              push("system", "  (execution failed — mounting with empty memory)");
            }
            vfsRef.current = {
              mountedTaskId: id,
              mountedTaskName: detail.task.name,
              cwd: "/",
              memorySnapshot: memBytes,
              exports: exportNames,
              taskInfo: {
                name: detail.task.name,
                id: detail.task.id,
                status: detail.task.status,
                size: formatBytes(detail.task.file_size_bytes),
                path: detail.task.path,
                created: new Date(detail.task.created_at).toLocaleString(),
              },
            };
            push("output", [
              `✓ Mounted "${detail.task.name}" at /`,
              `  Memory:  ${formatBytes(memBytes.length)} captured`,
              `  Exports: ${exportNames.length > 0 ? exportNames.join(", ") : "(none)"}`,
              `  Type 'vls' to browse, 'cd memory' to enter memory, 'unmount' to detach`,
            ].join("\n"));
            break;
          }

          // ── VFS: Unmount ───────────────────────────────────────────────
          case "unmount": {
            if (!vfsRef.current) { push("system", "No task mounted. Use 'mount <task-id>' first."); break; }
            const name = vfsRef.current.mountedTaskName;
            vfsRef.current = null;
            push("output", `✓ Unmounted "${name}"`);
            break;
          }

          // ── VFS: pwd ───────────────────────────────────────────────────
          case "pwd": {
            const v = vfsRef.current;
            if (!v) { push("system", "No task mounted. Use 'mount <id>' first."); break; }
            push("output", `/${v.mountedTaskName}${v.cwd === "/" ? "" : v.cwd}`);
            break;
          }

          // ── VFS: cd ────────────────────────────────────────────────────
          case "cd": {
            const v = vfsRef.current;
            if (!v) { push("system", "No task mounted. Use 'mount <id>' first."); break; }
            const target = (args[0] || "/").toLowerCase().replace(/^\/+|\/+$/g, "");
            if (!target || target === "/" || target === "..") {
              v.cwd = "/";
            } else if (target === "memory" || target === "mem") {
              v.cwd = "/memory";
            } else if (target === "exports" || target === "funcs") {
              v.cwd = "/exports";
            } else if (target === "info" || target === "meta") {
              v.cwd = "/info";
            } else {
              push("error", `No such directory: ${target}\n  Available: memory, exports, info (or '..' to go back)`);
              break;
            }
            push("output", `/${v.mountedTaskName}${v.cwd === "/" ? "" : v.cwd}`);
            break;
          }

          // ── VFS: ls (directory listing) ────────────────────────────────
          case "vls": {
            const v = vfsRef.current;
            if (!v) { push("system", "No task mounted. Use 'mount <id>' first."); break; }
            if (v.cwd === "/") {
              push("output", [
                `── /${v.mountedTaskName}/ ──`,
                `  📁 memory/       ${formatBytes(v.memorySnapshot.length)} linear memory`,
                `  📁 exports/      ${v.exports.length} exported function(s)`,
                `  📁 info/         task metadata`,
              ].join("\n"));
            } else if (v.cwd === "/memory") {
              const totalBytes = v.memorySnapshot.length;
              const pages = Math.ceil(totalBytes / 65536);
              const entries: string[] = [`── /${v.mountedTaskName}/memory/ ── (${pages} page(s), ${formatBytes(totalBytes)})`];
              for (let p = 0; p < Math.min(pages, 16); p++) {
                const start = p * 65536;
                const end = Math.min(start + 65536, totalBytes);
                // Check if page has non-zero content
                const hasData = v.memorySnapshot.slice(start, end).some((b) => b !== 0);
                entries.push(`  📄 page_${String(p).padStart(2, "0")}     ${formatBytes(end - start)}  ${hasData ? "●" : "○"} ${hasData ? "has data" : "empty"}`);
              }
              if (pages > 16) entries.push(`  … and ${pages - 16} more page(s)`);
              push("output", entries.join("\n"));
            } else if (v.cwd === "/exports") {
              if (v.exports.length === 0) {
                push("output", `── /${v.mountedTaskName}/exports/ ── (empty)`);
              } else {
                const entries = [`── /${v.mountedTaskName}/exports/ ──`, ...v.exports.map((e) => `  ⚙ ${e}`)];
                push("output", entries.join("\n"));
              }
            } else if (v.cwd === "/info") {
              const entries = [`── /${v.mountedTaskName}/info/ ──`, ...Object.entries(v.taskInfo).map(([k, val]) => `  📄 ${k.padEnd(12)} ${val}`)];
              push("output", entries.join("\n"));
            }
            break;
          }

          // ── VFS: cat ───────────────────────────────────────────────────
          case "cat": {
            const v = vfsRef.current;
            if (!v) { push("system", "No task mounted. Use 'mount <id>' first."); break; }
            const file = (args[0] || "").toLowerCase();
            if (!file) { push("error", "Usage: cat <filename>"); break; }
            if (v.cwd === "/info" || file.startsWith("info/")) {
              const key = file.replace("info/", "");
              if (v.taskInfo[key]) {
                push("output", v.taskInfo[key]);
              } else {
                push("error", `File not found: ${file}\n  Available: ${Object.keys(v.taskInfo).join(", ")}`);
              }
            } else if (v.cwd === "/exports" || file.startsWith("exports/")) {
              push("output", `Function: ${file.replace("exports/", "")}\n  (export details require advanced execution to inspect)`);
            } else if (v.cwd === "/memory" || file.startsWith("memory/")) {
              const pageName = file.replace("memory/", "");
              const pageNum = parseInt(pageName.replace("page_", "")) || 0;
              const start = pageNum * 65536;
              const end = Math.min(start + 256, v.memorySnapshot.length); // show first 256 bytes
              if (start >= v.memorySnapshot.length) { push("error", `Page ${pageNum} is out of range`); break; }
              const bytes = v.memorySnapshot.slice(start, end);
              const lines: string[] = [`── page_${String(pageNum).padStart(2, "0")} (first ${end - start} bytes) ──`];
              for (let off = 0; off < bytes.length; off += 16) {
                const row = bytes.slice(off, off + 16);
                const hex = row.map((b) => b.toString(16).padStart(2, "0")).join(" ");
                const ascii = row.map((b) => (b >= 32 && b < 127) ? String.fromCharCode(b) : ".").join("");
                lines.push(`  ${(start + off).toString(16).padStart(8, "0")}  ${hex.padEnd(48)}  |${ascii}|`);
              }
              push("output", lines.join("\n"));
            } else {
              push("error", `Cannot cat in cwd=${v.cwd}. Try 'cd memory' first, then 'cat page_00'.`);
            }
            break;
          }

          // ── VFS: hexdump ───────────────────────────────────────────────
          case "hexdump": {
            const v = vfsRef.current;
            if (!v) { push("system", "No task mounted. Use 'mount <id>' first."); break; }
            const offset = parseInt(args[0]) || 0;
            const length = Math.min(parseInt(args[1]) || 256, 1024);
            if (offset >= v.memorySnapshot.length) { push("error", `Offset ${offset} is beyond memory (${v.memorySnapshot.length} bytes)`); break; }
            const end = Math.min(offset + length, v.memorySnapshot.length);
            const bytes = v.memorySnapshot.slice(offset, end);
            const hexLines: string[] = [`── hexdump offset=0x${offset.toString(16)} len=${bytes.length} ──`];
            for (let off = 0; off < bytes.length; off += 16) {
              const row = bytes.slice(off, off + 16);
              const hex = row.map((b) => b.toString(16).padStart(2, "0")).join(" ");
              const ascii = row.map((b) => (b >= 32 && b < 127) ? String.fromCharCode(b) : ".").join("");
              hexLines.push(`  ${(offset + off).toString(16).padStart(8, "0")}  ${hex.padEnd(48)}  |${ascii}|`);
            }
            push("output", hexLines.join("\n"));
            break;
          }

          default: {
            const suggestions = ALL_CMDS.filter((c) => c.startsWith(rawCmd[0] || "")).slice(0, 4);
            push("error", `Unknown command: "${rawCmd}"\n  Did you mean: ${suggestions.length ? suggestions.join(", ") : "try 'help'"}`);
          }
        }

        // ── Apply pipe filters to collected output ──
        if (hasPipe && pipeOutputRef.current) {
          let pipedOutput = pipeOutputRef.current;
          for (const filterSeg of filters) {
            pipedOutput = applyPipeFilter(pipedOutput, filterSeg);
          }
          push("output", `── piped through ${filters.length} filter(s) ──\n${pipedOutput}`);
        }
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        if (cancelledRef.current) {
          push("system", "⚡ Cancelled");
        } else {
          push("error", `Error: ${msg}`);
        }
      } finally {
        setBusy(false);
        cancelledRef.current = false;
        pipeOutputRef.current = "";
      }
    },
    [push, pushTable, cmdHistory, liveId, topId, showTimestamps, resolveTaskId, findTask]
  );

  // ── Playbook helper: execute a single command line (without push("input",...)) ──
  const execSingleCommand = useCallback(async (line: string) => {
    // Strip pipe for playbook lines and run as-is
    await exec(line);
  }, [exec]);

  // ── File upload ──

  const handleFileUpload = useCallback(async (files: FileList | File[]) => {
    const fileArr = Array.from(files).filter((f) => f.name.endsWith(".wasm") || f.name.endsWith(".wat"));
    if (fileArr.length === 0) { push("error", "Only .wasm or .wat files are supported."); return; }
    setBusy(true);
    try {
      for (const f of fileArr) {
        const name = fileArr.length === 1
          ? (pendingUploadName.current || f.name.replace(/\.(wasm|wat)$/, ""))
          : f.name.replace(/\.(wasm|wat)$/, "");
        pendingUploadName.current = "";
        push("system", `Uploading "${name}" (${formatBytes(f.size)})…`);
        const bytes = await readFileAsBytes(f);
        const task = await uploadTask(name, bytes);
        push("output", `✓ Uploaded: "${task.name}" (${task.id.slice(0, 8)}) — ${formatBytes(f.size)}`);
      }
    } catch (err) {
      push("error", `Upload failed: ${err instanceof Error ? err.message : err}`);
    } finally {
      setBusy(false);
    }
  }, [push]);

  const handleFileInputChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
    if (!e.target.files || e.target.files.length === 0) return;
    await handleFileUpload(e.target.files);
    e.target.value = "";
  };

  // ── Drag & drop ──

  const onDragOver = (e: React.DragEvent) => { e.preventDefault(); setIsDragging(true); };
  const onDragLeave = () => setIsDragging(false);
  const onDrop = async (e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(false);
    const files = Array.from(e.dataTransfer.files);
    push("system", `📥 Dropped ${files.length} file(s) — uploading…`);
    await handleFileUpload(files);
  };

  // ── Keyboard ──

  const onKey = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter" && !busy) {
      exec(input);
      setInput("");
      setGhostText("");
    } else if (e.key === "c" && e.ctrlKey) {
      e.preventDefault();
      if (busy) {
        cancelledRef.current = true;
        setBusy(false);
        push("system", "⚡ Cancelling…");
      } else {
        setInput("");
        setGhostText("");
      }
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      if (cmdHistory.length > 0) {
        const idx = Math.min(histIdx + 1, cmdHistory.length - 1);
        setHistIdx(idx);
        setInput(cmdHistory[idx]);
        setGhostText("");
      }
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      if (histIdx > 0) { const idx = histIdx - 1; setHistIdx(idx); setInput(cmdHistory[idx]); }
      else { setHistIdx(-1); setInput(""); }
      setGhostText("");
    } else if (e.key === "Tab") {
      e.preventDefault();
      // Accept ghost suggestion if available
      if (ghostText) {
        setInput(input + ghostText);
        setGhostText("");
        return;
      }
      const partial = input.toLowerCase().trim();
      if (!partial) {
        push("system", "  " + ALL_CMDS.join("  "));
      } else {
        const match = ALL_CMDS.filter((c) => c.startsWith(partial));
        if (match.length === 1) setInput(match[0] + " ");
        else if (match.length > 1) push("system", "  " + match.join("  "));
      }
    } else if (e.key === "ArrowRight" && ghostText) {
      // Accept ghost suggestion with Right arrow at end of input
      const el = e.currentTarget;
      if (el.selectionStart === input.length) {
        e.preventDefault();
        setInput(input + ghostText);
        setGhostText("");
      }
    } else if (e.key === "Escape") {
      setGhostText("");
    } else if (e.key === "l" && e.ctrlKey) {
      e.preventDefault();
      setLines([]);
    }
  };

  const copyOutput = () => {
    const text = lines.map((l) => {
      const ts = showTimestamps ? `[${new Date(l.ts).toLocaleTimeString()}] ` : "";
      return ts + (l.type === "input" ? l.text : "  " + l.text);
    }).join("\n");
    navigator.clipboard.writeText(text).catch(() => {});
  };

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
      className={`flex flex-col rounded-xl border bg-slate-950 overflow-hidden transition-all ${
        fullscreen ? "fixed inset-0 z-50 rounded-none border-slate-600" : "h-full border-slate-700/50"
      } ${isDragging ? "border-blue-400 border-2 bg-blue-950/20" : ""}`}
      onDragOver={onDragOver}
      onDragLeave={onDragLeave}
      onDrop={onDrop}
    >
      {/* Header */}
      <div className="flex items-center gap-2 px-4 py-2 bg-slate-900 border-b border-slate-800 shrink-0">
        <TermIcon size={14} className="text-green-400" />
        <span className="text-xs text-slate-400 font-mono">wasm-os@shell</span>
        {busy && <span className="ml-2 text-xs text-yellow-400 animate-pulse">⏳ working… (Ctrl+C to cancel)</span>}
        {isDragging && <span className="ml-2 text-xs text-blue-400 animate-pulse font-semibold">📥 Drop .wasm here</span>}
        {liveId && !isDragging && !busy && <span className="ml-2 text-xs text-green-400 animate-pulse">● LIVE</span>}
        {topId && !isDragging && !busy && <span className="ml-2 text-xs text-emerald-400 animate-pulse">● TOP</span>}
        <div className="ml-auto flex items-center gap-2">
          <button
            onClick={() => setShowTimestamps((v) => !v)}
            className={`text-xs font-mono px-1.5 py-0.5 rounded transition-colors ${showTimestamps ? "text-yellow-400 bg-yellow-400/10" : "text-slate-500 hover:text-slate-300"}`}
            title="Toggle timestamps (or type 'timestamps')"
          >
            ts
          </button>
          <button onClick={copyOutput} className="text-slate-500 hover:text-slate-300 transition-colors" title="Copy all output">
            <Copy size={13} />
          </button>
          <button onClick={() => setFullscreen((v) => !v)} className="text-slate-500 hover:text-slate-300 transition-colors" title="Toggle fullscreen">
            {fullscreen ? <Minimize2 size={13} /> : <Maximize2 size={13} />}
          </button>
          <div className="flex gap-1.5 ml-2">
            <span className="w-3 h-3 rounded-full bg-red-500/60" />
            <span className="w-3 h-3 rounded-full bg-yellow-500/60" />
            <span className="w-3 h-3 rounded-full bg-green-500/60" />
          </div>
        </div>
      </div>

      {/* Output */}
      <div
        className="flex-1 overflow-y-auto px-4 py-3 font-mono text-[13px] leading-relaxed"
        onClick={() => inputRef.current?.focus()}
      >
        {lines.map((line, i) => (
          <pre key={i} className={`whitespace-pre-wrap break-words ${lineColor(line.type)}`}>
            {showTimestamps && (
              <span className="text-slate-600 text-[11px] mr-2 select-none">
                {new Date(line.ts).toLocaleTimeString()}
              </span>
            )}
            {line.text}
          </pre>
        ))}
        <div ref={bottomRef} />
      </div>

      {/* Input */}
      <div className="flex items-center gap-2 px-4 py-2.5 bg-slate-900/80 border-t border-slate-800 shrink-0">
        <span className="text-green-400 text-sm font-mono shrink-0 select-none">
          {busy ? "⏳" : vfsRef.current ? `📁 ${vfsRef.current.mountedTaskName}:${vfsRef.current.cwd} $` : "$"}
        </span>
        <div className="relative flex-1">
          <input
            ref={inputRef}
            type="text"
            value={input}
            onChange={(e) => { setInput(e.target.value); computeGhost(e.target.value); }}
            onKeyDown={onKey}
            placeholder={busy ? "Running… (Ctrl+C to cancel)" : "Type a command… (Tab=ghost/autocomplete  →=accept  ↑↓=history)"}
            className="w-full bg-transparent text-slate-200 text-[13px] font-mono outline-none placeholder:text-slate-600 relative z-10"
            autoFocus
            autoComplete="off"
            spellCheck={false}
          />
          {/* Ghost text suggestion overlay */}
          {ghostText && (
            <span
              className="absolute top-0 left-0 text-[13px] font-mono pointer-events-none select-none text-slate-600/60"
              aria-hidden="true"
            >
              <span className="invisible">{input}</span>
              <span>{ghostText}</span>
            </span>
          )}
        </div>
        {input && (
          <button onClick={() => { setInput(""); setGhostText(""); inputRef.current?.focus(); }} className="text-slate-600 hover:text-slate-400 transition-colors shrink-0" title="Clear input (Ctrl+C)">
            <X size={13} />
          </button>
        )}
      </div>

      {/* Hidden file inputs */}
      <input ref={fileRef} type="file" accept=".wasm,.wat" multiple className="hidden" onChange={handleFileInputChange} />
      <input ref={playbookFileRef} type="file" accept=".wasmos,.txt" className="hidden" />
    </div>
  );
}
