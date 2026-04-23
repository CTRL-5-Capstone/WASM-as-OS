"use client";

import { useEffect, useState, useCallback } from "react";
import {
  Play, RefreshCw, CheckCircle, AlertCircle, FileCode, Clock, Cpu,
  Zap, HardDrive, FlaskConical, PlayCircle, ChevronDown, ChevronRight,
  Filter, BarChart3, Trophy, XCircle,
} from "lucide-react";
import {
  getTestFiles, runTestFile, runAllTestFiles,
  type TestFile, type TestRunResult, type TestRunAllResult,
} from "@/lib/api";
import { formatBytes, formatDuration, formatNumber, cn } from "@/lib/utils";
import { useTerminal } from "@/lib/terminal-context";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";

// ─── Category colors ────────────────────────────────────────────────

const CAT_COLORS: Record<string, string> = {
  arithmetic:    "bg-blue-500/10 text-blue-400 border-blue-500/20",
  integration:   "bg-purple-500/10 text-purple-400 border-purple-500/20",
  "control-flow":"bg-amber-500/10 text-amber-400 border-amber-500/20",
  application:   "bg-emerald-500/10 text-emerald-400 border-emerald-500/20",
  complex:       "bg-red-500/10 text-red-400 border-red-500/20",
  large:         "bg-orange-500/10 text-orange-400 border-orange-500/20",
  general:       "bg-muted text-muted-foreground border-border",
};

const catColor = (cat: string) => CAT_COLORS[cat] || CAT_COLORS.general;

// ═════════════════════════════════════════════════════════════════════

type RunStatus = "idle" | "running" | "pass" | "fail";

interface FileState {
  file: TestFile;
  status: RunStatus;
  result: TestRunResult | null;
  expanded: boolean;
}

// ═════════════════════════════════════════════════════════════════════

export default function TestsPage() {
  const [files, setFiles] = useState<FileState[]>([]);
  const [loading, setLoading] = useState(true);
  const [runningAll, setRunningAll] = useState(false);
  const [batchResult, setBatchResult] = useState<TestRunAllResult | null>(null);
  const [categoryFilter, setCategoryFilter] = useState("all");
  const [toast, setToast] = useState<{ msg: string; ok: boolean } | null>(null);
  const { push: ctxPush } = useTerminal();

  const notify = (msg: string, ok = true) => {
    setToast({ msg, ok });
    setTimeout(() => setToast(null), 4000);
  };

  // ── Load test files ──

  const loadFiles = useCallback(async () => {
    setLoading(true);
    try {
      const data = await getTestFiles();
      setFiles(
        data.files.map((f) => ({
          file: f,
          status: "idle" as RunStatus,
          result: null,
          expanded: false,
        }))
      );
    } catch {
      notify("Failed to load test files", false);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadFiles();
  }, [loadFiles]);

  // ── Categories ──

  const categories = ["all", ...Array.from(new Set(files.map((f) => f.file.category)))];

  const filtered = categoryFilter === "all"
    ? files
    : files.filter((f) => f.file.category === categoryFilter);

  // ── Counts ──

  const passed = files.filter((f) => f.status === "pass").length;
  const failed = files.filter((f) => f.status === "fail").length;
  const running = files.filter((f) => f.status === "running").length;
  const idle = files.filter((f) => f.status === "idle").length;

  // ── Run single test ──

  const runSingle = async (name: string) => {
    setFiles((prev) =>
      prev.map((f) =>
        f.file.name === name ? { ...f, status: "running" as RunStatus, result: null } : f
      )
    );

    ctxPush("system", `[test] Running ${name}...`);

    try {
      const r = await runTestFile(name);
      setFiles((prev) =>
        prev.map((f) =>
          f.file.name === name
            ? { ...f, status: r.success ? "pass" : "fail", result: r, expanded: true }
            : f
        )
      );
      ctxPush(
        r.success ? "output" : "error",
        `[test] ${name}: ${r.success ? "PASS" : "FAIL"} (${formatDuration(r.duration_us)})`
      );
      notify(`${name}: ${r.success ? "PASS" : "FAIL"}`, r.success);
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : "Run failed";
      setFiles((prev) =>
        prev.map((f) =>
          f.file.name === name
            ? {
                ...f,
                status: "fail" as RunStatus,
                result: {
                  file: name,
                  success: false,
                  duration_us: 0,
                  instructions_executed: 0,
                  syscalls_executed: 0,
                  memory_used_bytes: 0,
                  stdout_log: [],
                  return_value: null,
                  error: msg,
                },
                expanded: true,
              }
            : f
        )
      );
      ctxPush("error", `[test] ${name}: ERROR — ${msg}`);
      notify(`${name}: ${msg}`, false);
    }
  };

  // ── Run all tests ──

  const runAll = async () => {
    setRunningAll(true);
    setBatchResult(null);

    // Reset all statuses
    setFiles((prev) =>
      prev.map((f) => ({ ...f, status: "running" as RunStatus, result: null, expanded: false }))
    );

    ctxPush("system", `[test-suite] Running all ${files.length} test files...`);

    try {
      const cat = categoryFilter !== "all" ? categoryFilter : undefined;
      const data = await runAllTestFiles(cat);
      setBatchResult(data);

      // Map results back to file states
      setFiles((prev) =>
        prev.map((f) => {
          const r = data.results.find((x) => x.file === f.file.name);
          if (r) {
            return { ...f, status: r.success ? "pass" : "fail", result: r };
          }
          // File was skipped (e.g. too large)
          return { ...f, status: "idle" as RunStatus };
        })
      );

      ctxPush(
        "output",
        `[test-suite] Complete: ${data.passed}/${data.total} passed, ${data.failed} failed (${formatDuration(data.total_duration_us)})`
      );
      notify(`${data.passed}/${data.total} tests passed`, data.failed === 0);
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : "Batch run failed";
      setFiles((prev) =>
        prev.map((f) => ({ ...f, status: "fail" as RunStatus }))
      );
      ctxPush("error", `[test-suite] Failed: ${msg}`);
      notify(msg, false);
    } finally {
      setRunningAll(false);
    }
  };

  // ── Toggle expand ──

  const toggle = (name: string) => {
    setFiles((prev) =>
      prev.map((f) =>
        f.file.name === name ? { ...f, expanded: !f.expanded } : f
      )
    );
  };

  // ── Reset ──

  const resetAll = () => {
    setFiles((prev) =>
      prev.map((f) => ({ ...f, status: "idle" as RunStatus, result: null, expanded: false }))
    );
    setBatchResult(null);
  };

  // ── Status icon ──

  const statusIcon = (s: RunStatus) => {
    switch (s) {
      case "running":
        return <RefreshCw size={14} className="text-indigo-400 animate-spin" />;
      case "pass":
        return <CheckCircle size={14} className="text-green-400" />;
      case "fail":
        return <XCircle size={14} className="text-red-400" />;
      default:
        return <div className="w-3.5 h-3.5 rounded-full border border-muted-foreground" />;
    }
  };

  // ═════════════════════════════════════════════════════════════════
  // Render
  // ═════════════════════════════════════════════════════════════════

  return (
    <div className="animate-fade-in space-y-6">
      {/* ── Header ── */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold gradient-text flex items-center gap-2">
            <FlaskConical size={20} /> Test Suite
          </h1>
          <p className="mt-1 text-sm text-muted-foreground">
            Run all WasmOS test files from the engine&apos;s test directories
          </p>
        </div>
        <div className="flex gap-2">
          <Button onClick={resetAll} variant="ghost" size="sm">
            <RefreshCw size={14} /> Reset
          </Button>
          <Button onClick={loadFiles} variant="ghost" size="sm">
            <RefreshCw size={14} /> Reload
          </Button>
        </div>
      </div>

      {/* ── Summary ribbon ── */}
      <div className="grid grid-cols-5 gap-3">
        {[
          { label: "Total Files", count: files.length, cls: "text-foreground", icon: FileCode, iconCls: "text-indigo-400" },
          { label: "Passed", count: passed, cls: "text-green-400", icon: CheckCircle, iconCls: "text-green-400" },
          { label: "Failed", count: failed, cls: "text-red-400", icon: XCircle, iconCls: "text-red-400" },
          { label: "Running", count: running, cls: "text-indigo-400", icon: RefreshCw, iconCls: "text-indigo-400" },
          { label: "Pending", count: idle, cls: "text-muted-foreground", icon: Clock, iconCls: "text-muted-foreground" },
        ].map(({ label, count, cls, icon: Icon, iconCls }) => (
          <Card key={label} className="p-4 text-center">
            <Icon size={18} className={cn("mx-auto mb-1", iconCls)} />
            <p className={cn("text-2xl font-bold", cls)}>{count}</p>
            <p className="text-[11px] text-muted-foreground">{label}</p>
          </Card>
        ))}
      </div>

      {/* ── Batch result banner ── */}
      {batchResult && (
        <Card
          className={cn(
            "p-4 flex items-center gap-4 border",
            batchResult.failed === 0
              ? "border-green-500/30 bg-green-500/5"
              : "border-red-500/30 bg-red-500/5"
          )}
        >
          <div className={cn("rounded-full p-3", batchResult.failed === 0 ? "bg-green-500/20" : "bg-red-500/20")}>
            {batchResult.failed === 0 ? (
              <Trophy size={22} className="text-green-400" />
            ) : (
              <AlertCircle size={22} className="text-red-400" />
            )}
          </div>
          <div className="flex-1">
            <p className={cn("text-sm font-semibold", batchResult.failed === 0 ? "text-green-400" : "text-red-400")}>
              {batchResult.failed === 0
                ? "All Tests Passed!"
                : `${batchResult.failed} of ${batchResult.total} Tests Failed`}
            </p>
            <p className="text-xs text-muted-foreground mt-0.5">
              {batchResult.passed}/{batchResult.total} passed · Total time: {formatDuration(batchResult.total_duration_us)}
            </p>
          </div>
          <div className="flex gap-4 text-center">
            <div>
              <p className="text-lg font-bold text-green-400">{batchResult.passed}</p>
              <p className="text-[10px] text-muted-foreground">Pass</p>
            </div>
            <div>
              <p className="text-lg font-bold text-red-400">{batchResult.failed}</p>
              <p className="text-[10px] text-muted-foreground">Fail</p>
            </div>
          </div>
        </Card>
      )}

      {/* ── Controls bar ── */}
      <div className="flex items-center gap-3 flex-wrap">
        {/* Run All */}
        <Button onClick={runAll} disabled={runningAll || files.length === 0} variant="green">
          {runningAll ? <RefreshCw size={14} className="animate-spin" /> : <PlayCircle size={14} />}
          {runningAll ? "Running…" : "Run All Tests"}
        </Button>

        {/* Category filter */}
        <div className="flex items-center gap-1.5">
          <Filter size={13} className="text-muted-foreground" />
          {categories.map((cat) => (
            <button
              key={cat}
              onClick={() => setCategoryFilter(cat)}
              className={cn(
                "rounded-full px-3 py-1 text-xs font-medium border transition-all",
                categoryFilter === cat
                  ? "ring-1 ring-indigo-500/40 bg-indigo-500/10 text-indigo-400 border-indigo-500/30"
                  : cat === "all"
                  ? "text-muted-foreground bg-muted border-border hover:border-border"
                  : catColor(cat)
              )}
            >
              {cat === "all" ? "All" : cat}
            </button>
          ))}
        </div>

        {/* Stats summary */}
        <div className="ml-auto text-xs text-muted-foreground">
          {filtered.length} file{filtered.length !== 1 && "s"} shown
        </div>
      </div>

      {/* ── File list ── */}
      {loading ? (
        <Card>
          <CardContent className="p-10 text-center">
          <RefreshCw size={28} className="mx-auto text-sky-400 animate-spin mb-3" />
          <p className="text-sm text-muted-foreground">Discovering test files…</p>
          </CardContent>
        </Card>
      ) : filtered.length === 0 ? (
        <Card>
          <CardContent className="p-10 text-center text-muted-foreground">
          <FlaskConical size={36} className="mx-auto mb-3 text-muted-foreground" />
          <p className="text-sm">No test files found in WasmOS test or wasm_files directories</p>
          <p className="text-xs mt-1 text-muted-foreground">Make sure the backend server is running and the test directories exist</p>
          </CardContent>
        </Card>
      ) : (
        <div className="space-y-1.5">
          {filtered.map(({ file, status, result, expanded }) => (
            <Card key={file.name + file.source} className="overflow-hidden transition-all">
              {/* ── Row ── */}
              <div
                  className={cn(
                  "flex items-center gap-3 px-4 py-3 cursor-pointer hover:bg-muted/30 transition-colors",
                  status === "pass" && "border-l-2 border-green-500",
                  status === "fail" && "border-l-2 border-red-500"
                )}
                onClick={() => toggle(file.name)}
              >
                {/* Expand arrow */}
                {expanded ? (
                  <ChevronDown size={14} className="text-muted-foreground shrink-0" />
                ) : (
                  <ChevronRight size={14} className="text-muted-foreground shrink-0" />
                )}

                {/* Status */}
                {statusIcon(status)}

                {/* File info */}
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="text-sm font-medium text-foreground truncate">{file.name}</span>
                    <span className={cn("text-[10px] rounded-full border px-2 py-0.5", catColor(file.category))}>
                      {file.category}
                    </span>
                    <span className="text-[10px] text-muted-foreground">{file.source}</span>
                  </div>
                  <div className="flex gap-3 mt-0.5 text-[11px] text-muted-foreground">
                    <span>{formatBytes(file.size_bytes)}</span>
                    {result && (
                      <>
                        <span>{formatDuration(result.duration_us)}</span>
                        <span>{formatNumber(result.instructions_executed)} instr</span>
                      </>
                    )}
                  </div>
                </div>

                {/* Run button */}
                <Button
                  onClick={(e) => {
                    e.stopPropagation();
                    runSingle(file.name);
                  }}
                  disabled={status === "running" || runningAll}
                  variant="ghost"
                  size="icon"
                  className="shrink-0 text-green-400 hover:bg-green-500/20 hover:text-green-300 disabled:opacity-30"
                  title="Run test"
                >
                  {status === "running" ? (
                    <RefreshCw size={14} className="animate-spin" />
                  ) : (
                    <Play size={14} />
                  )}
                </Button>
              </div>

              {/* ── Expanded detail ── */}
              {expanded && result && (
                <div className="border-t border-border px-5 py-4 space-y-3 bg-muted/20">
                  {/* Result banner */}
                  <div
                    className={cn(
                      "flex items-center gap-2 rounded-lg px-3 py-2 text-xs font-medium",
                      result.success
                        ? "bg-green-500/10 text-green-400"
                        : "bg-red-500/10 text-red-400"
                    )}
                  >
                    {result.success ? (
                      <CheckCircle size={14} />
                    ) : (
                      <AlertCircle size={14} />
                    )}
                    {result.success ? "PASS" : "FAIL"}
                    {result.error && (
                      <span className="ml-2 text-red-400/80 font-normal truncate">{result.error}</span>
                    )}
                  </div>

                  {/* Metric cards */}
                  <div className="grid grid-cols-4 gap-2">
                    {[
                      { label: "Duration", value: formatDuration(result.duration_us), icon: Clock, color: "text-sky-400" },
                      { label: "Instructions", value: formatNumber(result.instructions_executed), icon: Cpu, color: "text-violet-400" },
                      { label: "Syscalls", value: formatNumber(result.syscalls_executed), icon: Zap, color: "text-amber-400" },
                      { label: "Memory", value: formatBytes(result.memory_used_bytes), icon: HardDrive, color: "text-emerald-400" },
                    ].map(({ label, value, icon: Icon, color }) => (
                      <div key={label} className="rounded-lg bg-card border border-border p-2.5 text-center">
                        <Icon size={13} className={cn("mx-auto mb-1", color)} />
                        <p className="text-xs font-bold text-foreground">{value}</p>
                        <p className="text-[10px] text-muted-foreground">{label}</p>
                      </div>
                    ))}
                  </div>

                  {/* Stdout */}
                  {result.stdout_log.length > 0 && (
                    <div>
                      <h4 className="text-[11px] font-medium text-muted-foreground uppercase mb-1">stdout</h4>
                      <pre className="rounded-lg bg-black/50 p-2.5 text-xs text-green-400 max-h-32 overflow-auto font-mono leading-relaxed">
                        {result.stdout_log.join("\n")}
                      </pre>
                    </div>
                  )}

                  {/* Return value */}
                  {result.return_value != null && (
                    <div className="flex items-center gap-2 text-xs">
                      <span className="text-muted-foreground">Return:</span>
                      <code className="text-indigo-400 font-mono bg-indigo-500/10 rounded px-2 py-0.5">
                        {String(result.return_value)}
                      </code>
                    </div>
                  )}
                </div>
              )}

              {/* Expanded but no result */}
              {expanded && !result && status !== "running" && (
                <div className="border-t border-border px-5 py-4 text-center">
                  <p className="text-xs text-muted-foreground">Click Run to execute this test</p>
                </div>
              )}

              {/* Running spinner */}
              {expanded && status === "running" && (
                <div className="border-t border-border px-5 py-6 text-center">
                  <RefreshCw size={20} className="mx-auto text-indigo-400 animate-spin mb-2" />
                  <p className="text-xs text-muted-foreground">Executing…</p>
                </div>
              )}
            </Card>
          ))}
        </div>
      )}

      {/* ── Chart-like summary bar ── */}
      {(passed > 0 || failed > 0) && (
        <Card className="p-4">
          <div className="flex items-center gap-2 mb-2">
            <BarChart3 size={14} className="text-muted-foreground" />
            <span className="text-xs text-muted-foreground uppercase font-medium">Results Distribution</span>
          </div>
          <div className="flex rounded-full overflow-hidden h-3 bg-muted">
            {passed > 0 && (
              <div
                className="bg-green-500 transition-all duration-500"
                style={{ width: `${(passed / (passed + failed)) * 100}%` }}
              />
            )}
            {failed > 0 && (
              <div
                className="bg-red-500 transition-all duration-500"
                style={{ width: `${(failed / (passed + failed)) * 100}%` }}
              />
            )}
          </div>
          <div className="flex justify-between mt-1.5 text-[11px] text-muted-foreground">
            <span>
              <span className="inline-block w-2 h-2 rounded-full bg-green-500 mr-1" />
              {passed} Passed ({files.length > 0 ? Math.round((passed / files.length) * 100) : 0}%)
            </span>
            <span>
              <span className="inline-block w-2 h-2 rounded-full bg-red-500 mr-1" />
              {failed} Failed ({files.length > 0 ? Math.round((failed / files.length) * 100) : 0}%)
            </span>
          </div>
        </Card>
      )}

      {/* ── Toast ── */}
      {toast && (
        <div
          className={cn(
            "fixed bottom-6 right-6 z-50 flex items-center gap-2 rounded-lg px-4 py-3 text-sm font-medium shadow-lg animate-slide-up",
            toast.ok
              ? "bg-green-500/20 text-green-400 border border-green-500/30"
              : "bg-red-500/20 text-red-400 border border-red-500/30"
          )}
        >
          {toast.ok ? <CheckCircle size={16} /> : <AlertCircle size={16} />}
          {toast.msg}
        </div>
      )}
    </div>
  );
}
