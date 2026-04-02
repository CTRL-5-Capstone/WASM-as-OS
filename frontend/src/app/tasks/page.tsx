"use client";

import { useEffect, useState, useRef, useCallback, type DragEvent } from "react";
import Link from "next/link";
import {
  Upload, Play, Square, Trash2, RefreshCw, FileCode, Search, X,
  CheckCircle, AlertTriangle, Clock, Cpu, Zap, HardDrive,
  ArrowUpDown, Activity, Pause, RotateCcw, Edit2, History,
  ShieldAlert, ExternalLink, Terminal as TerminalIcon, FileText,
} from "lucide-react";
import {
  getTasks, getTask, uploadTask, startTask, stopTask, deleteTask,
  pauseTask, restartTask, updateTask, getTaskExecutionHistory,
  getTaskLogs, readFileAsBytes,
  type Task, type TaskDetail, type ExecutionResult,
  type TaskLog, type ExecutionHistory,
} from "@/lib/api";
import { formatBytes, formatDuration, timeAgo, cn } from "@/lib/utils";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Separator } from "@/components/ui/separator";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Skeleton } from "@/components/ui/skeleton";
import { Alert, AlertDescription } from "@/components/ui/alert";

// ─── Sort helpers ────────────────────────────────────────────────────

type SortKey = "name" | "status" | "size" | "created";
const sortFns: Record<SortKey, (a: Task, b: Task) => number> = {
  name:    (a, b) => a.name.localeCompare(b.name),
  status:  (a, b) => a.status.localeCompare(b.status),
  size:    (a, b) => b.file_size_bytes - a.file_size_bytes,
  created: (a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime(),
};

// ─── Status helpers ──────────────────────────────────────────────────

// Backend serialises TaskStatus lowercase ("running", "completed", etc.)
function statusVariant(status: string) {
  switch (status?.toLowerCase()) {
    case "running":   return "green"       as const;
    case "completed": return "default"     as const;
    case "failed":    return "destructive" as const;
    case "stopped":   return "yellow"      as const;
    default:          return "secondary"   as const;
  }
}

function statusDot(status: string) {
  const cls: Record<string, string> = {
    running:   "bg-green-400 animate-pulse",
    completed: "bg-blue-400",
    failed:    "bg-red-400",
    stopped:   "bg-yellow-400",
    pending:   "bg-muted-foreground",
  };
  return (
    <span
      className={cn(
        "inline-block h-1.5 w-1.5 rounded-full shrink-0",
        cls[status?.toLowerCase()] ?? "bg-muted-foreground"
      )}
    />
  );
}

// ─── Sub-components ──────────────────────────────────────────────────

function MetricPill({ icon: Icon, label, value }: {
  icon: React.ElementType; label: string; value: string;
}) {
  return (
    <div className="flex items-center gap-2 rounded-lg bg-muted/30 border border-border px-3 py-2">
      <Icon size={13} className="text-muted-foreground shrink-0" />
      <div>
        <p className="text-[10px] text-muted-foreground uppercase tracking-wider">{label}</p>
        <p className="text-xs font-semibold text-foreground">{value}</p>
      </div>
    </div>
  );
}

function ExecuteResultPanel({ result }: { result: ExecutionResult }) {
  return (
    <div className="space-y-3">
      <div className="flex items-center gap-2">
        {result.success ? (
          <CheckCircle size={15} className="text-green-400" />
        ) : (
          <AlertTriangle size={15} className="text-red-400" />
        )}
        <span className={cn("text-sm font-medium", result.success ? "text-green-400" : "text-red-400")}>
          {result.success ? "Execution successful" : "Execution failed"}
        </span>
        {result.duration_us != null && (
          <span className="ml-auto text-xs text-muted-foreground">
            {formatDuration(result.duration_us)}
          </span>
        )}
      </div>

      {result.stdout_log?.length > 0 && (
        <div>
          <p className="mb-1.5 text-[10px] uppercase tracking-wider text-muted-foreground font-medium">stdout</p>
          <pre className="rounded-lg bg-black/40 border border-border p-3 text-xs font-mono text-green-300 overflow-auto max-h-48 whitespace-pre-wrap">
            {result.stdout_log.join("\n")}
          </pre>
        </div>
      )}
      {result.error && (
        <div>
          <p className="mb-1.5 text-[10px] uppercase tracking-wider text-muted-foreground font-medium">stderr</p>
          <pre className="rounded-lg bg-red-950/30 border border-red-900/30 p-3 text-xs font-mono text-red-300 overflow-auto max-h-32 whitespace-pre-wrap">
            {result.error}
          </pre>
        </div>
      )}

      <div className="grid grid-cols-2 gap-2 pt-1">
        <MetricPill icon={Cpu}       label="Instructions"  value={(result.instructions_executed ?? 0).toLocaleString()} />
        <MetricPill icon={Zap}       label="Syscalls"      value={(result.syscalls_executed ?? 0).toLocaleString()} />
        <MetricPill icon={HardDrive} label="Memory (B)"    value={(result.memory_used_bytes ?? 0).toLocaleString()} />
        <MetricPill icon={Activity}  label="Return value"  value={String(result.return_value ?? 0)} />
      </div>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// Main page
// ═══════════════════════════════════════════════════════════════════════

export default function TasksPage() {
  // ── State ──
  const [tasks,        setTasks]        = useState<Task[]>([]);
  const [selected,     setSelected]     = useState<TaskDetail | null>(null);
  const [loadingDetail,setLoadingDetail]= useState(false);
  const [execResult,   setExecResult]   = useState<ExecutionResult | null>(null);
  const [taskLog,      setTaskLog]      = useState<TaskLog | null>(null);
  const [execHistory,  setExecHistory]  = useState<ExecutionHistory[]>([]);
  const [historyLoading, setHistoryLoading] = useState(false);
  const [activeTab,    setActiveTab]    = useState("overview");
  const [editingName,  setEditingName]  = useState(false);
  const [editName,     setEditName]     = useState("");
  const [filter,       setFilter]       = useState("");
  const [statusFilter, setStatusFilter] = useState("all");
  const [sortKey,      setSortKey]      = useState<SortKey>("created");
  const [sortAsc,      setSortAsc]      = useState(false);
  const [uploading,    setUploading]    = useState(false);
  const [executing,    setExecuting]    = useState<string | null>(null);
  const [dragOver,     setDragOver]     = useState(false);
  const [toastMsg,     setToastMsg]     = useState<{ msg: string; ok: boolean } | null>(null);

  const fileRef = useRef<HTMLInputElement>(null);

  // ── Data fetching ──

  const refresh = useCallback(async () => {
    try { setTasks(await getTasks()); } catch {}
  }, []);

  useEffect(() => {
    refresh();
    const id = setInterval(refresh, 6_000);
    return () => clearInterval(id);
  }, [refresh]);

  const notify = (msg: string, ok = true) => {
    setToastMsg({ msg, ok });
    setTimeout(() => setToastMsg(null), 4_000);
  };

  // ── Filtering / sorting ──

  const filtered = tasks
    .filter((t) => {
      if (filter && !t.name.toLowerCase().includes(filter.toLowerCase()) && !t.id.startsWith(filter)) return false;
      if (statusFilter !== "all" && t.status !== statusFilter) return false;
      return true;
    })
    .sort((a, b) => sortAsc ? sortFns[sortKey](a, b) : -sortFns[sortKey](a, b));

  const toggleSort = (key: SortKey) => {
    if (sortKey === key) setSortAsc(!sortAsc);
    else { setSortKey(key); setSortAsc(false); }
  };

  const counts = tasks.reduce<Record<string, number>>(
    (acc, t) => { acc[t.status] = (acc[t.status] || 0) + 1; return acc; }, {}
  );

  // ── Task selection ──

  const selectTask = async (id: string) => {
    setLoadingDetail(true);
    setActiveTab("overview");
    setExecResult(null);
    setTaskLog(null);
    try {
      const d = await getTask(id);
      setSelected(d);
      try { setTaskLog(await getTaskLogs(id)); } catch {}
    } catch {}
    finally { setLoadingDetail(false); }
  };

  // ── Actions ──

  const handleUpload = async (file: File) => {
    if (!file.name.match(/\.(wasm|wat)$/)) { notify("Only .wasm / .wat files accepted", false); return; }
    setUploading(true);
    try {
      const bytes = await readFileAsBytes(file);
      await uploadTask(file.name.replace(/\.(wasm|wat)$/, ""), bytes);
      notify("Uploaded " + file.name);
      refresh();
    } catch (e: unknown) { notify(e instanceof Error ? e.message : "Upload failed", false); }
    finally { setUploading(false); }
  };

  const handleDrop = (e: DragEvent<HTMLDivElement>) => {
    e.preventDefault(); setDragOver(false);
    const file = e.dataTransfer.files?.[0];
    if (file) handleUpload(file);
  };

  const handleStart = async (id: string) => {
    setExecuting(id);
    try {
      const r = await startTask(id);
      setExecResult(r);
      setActiveTab("execute");
      notify(r.success ? "Execution complete" : "Execution failed", r.success);
      if (selected?.task.id === id) {
        const d = await getTask(id);
        setSelected(d);
        try { setTaskLog(await getTaskLogs(id)); } catch {}
      }
      refresh();
    } catch (e: unknown) { notify(e instanceof Error ? e.message : "Execution failed", false); }
    finally { setExecuting(null); }
  };

  const handleStop    = async (id: string) => {
    try { await stopTask(id); notify("Stopped"); refresh(); if (selected?.task.id === id) selectTask(id); }
    catch (e: unknown) { notify(e instanceof Error ? e.message : "Stop failed", false); }
  };

  const handleDelete  = async (id: string) => {
    try { await deleteTask(id); notify("Deleted"); if (selected?.task.id === id) setSelected(null); refresh(); }
    catch (e: unknown) { notify(e instanceof Error ? e.message : "Delete failed", false); }
  };

  const handlePause   = async (id: string) => {
    try { await pauseTask(id); notify("Paused"); refresh(); if (selected?.task.id === id) selectTask(id); }
    catch (e: unknown) { notify(e instanceof Error ? e.message : "Pause failed", false); }
  };

  const handleRestart = async (id: string) => {
    try { await restartTask(id); notify("Queued for restart"); refresh(); if (selected?.task.id === id) selectTask(id); }
    catch (e: unknown) { notify(e instanceof Error ? e.message : "Restart failed", false); }
  };

  const handleUpdateName = async () => {
    if (!selected || !editName.trim()) return;
    try {
      await updateTask(selected.task.id, { name: editName.trim() });
      notify("Renamed");
      setEditingName(false);
      refresh();
      selectTask(selected.task.id);
    } catch (e: unknown) { notify(e instanceof Error ? e.message : "Rename failed", false); }
  };

  const loadHistory = useCallback(async (id: string) => {
    setHistoryLoading(true);
    try {
      const res = await getTaskExecutionHistory(id, 50);
      setExecHistory(res.executions ?? []);
    } catch { setExecHistory([]); }
    finally { setHistoryLoading(false); }
  }, []);

  useEffect(() => {
    if (activeTab === "history" && selected?.task.id) loadHistory(selected.task.id);
  }, [activeTab, selected?.task.id, loadHistory]);

  const sel = selected?.task ?? null;

  // ══════════════════════════════════════════════════
  // Render
  // ══════════════════════════════════════════════════

  return (
    <div className="animate-fade-in space-y-5">

      {/* Toast */}
      {toastMsg && (
        <div className={cn(
          "fixed top-4 right-4 z-50 flex items-center gap-2 rounded-lg border px-4 py-3 text-sm font-medium shadow-lg",
          toastMsg.ok
            ? "bg-green-950/80 border-green-800 text-green-300"
            : "bg-red-950/80 border-red-800 text-red-300"
        )}>
          {toastMsg.ok ? <CheckCircle size={14} /> : <AlertTriangle size={14} />}
          {toastMsg.msg}
        </div>
      )}

      {/* Hidden file input */}
      <input
        ref={fileRef}
        type="file"
        accept=".wasm,.wat"
        className="hidden"
        onChange={(e) => { const f = e.target.files?.[0]; if (f) handleUpload(f); e.target.value = ""; }}
      />

      {/* ── Header ── */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold gradient-text">Tasks</h1>
          <p className="mt-0.5 text-sm text-muted-foreground">
            Upload, execute and inspect WASM modules
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Button onClick={() => fileRef.current?.click()} size="sm" disabled={uploading}>
            <Upload size={14} />
            {uploading ? "Uploading…" : "Upload WASM"}
          </Button>
          <Button onClick={refresh} variant="ghost" size="icon" className="h-9 w-9">
            <RefreshCw size={14} />
          </Button>
        </div>
      </div>

      {/* ── Status ribbon ── */}
      <div className="flex gap-1.5 flex-wrap">
        {[
          { label: "All",       count: tasks.length,            filter: "all",       cls: "text-foreground" },
          { label: "Running",   count: counts["running"] || 0,  filter: "running",   cls: "text-green-400" },
          { label: "Completed", count: counts["completed"] || 0,filter: "completed", cls: "text-blue-400" },
          { label: "Failed",    count: counts["failed"] || 0,   filter: "failed",    cls: "text-red-400" },
          { label: "Pending",   count: counts["pending"] || 0,  filter: "pending",   cls: "text-yellow-400" },
          { label: "Stopped",   count: counts["stopped"] || 0,  filter: "stopped",   cls: "text-muted-foreground" },
        ].map(({ label, count, filter: f, cls }) => (
          <button
            key={label}
            onClick={() => setStatusFilter(f)}
            className={cn(
              "rounded-full px-3 py-1 text-xs font-medium border border-border transition-all",
              cls,
              statusFilter === f
                ? "ring-1 ring-primary bg-primary/10 border-primary/30"
                : "bg-muted/20 hover:bg-muted/40"
            )}
          >
            {label} <span className="ml-1 font-bold">{count}</span>
          </button>
        ))}
      </div>

      {/* ── Two-column layout ── */}
      <div className="grid grid-cols-1 xl:grid-cols-[380px_1fr] gap-4 items-start">

        {/* ── LEFT: Task list ── */}
        <div className="flex flex-col gap-3">
          {/* Upload drop zone */}
          <div
            onDragOver={(e) => { e.preventDefault(); setDragOver(true); }}
            onDragLeave={() => setDragOver(false)}
            onDrop={handleDrop}
            onClick={() => fileRef.current?.click()}
            className={cn(
              "flex items-center justify-center gap-3 rounded-xl border-2 border-dashed p-4 cursor-pointer transition-all",
              dragOver ? "border-primary/60 bg-primary/8" : "border-border hover:border-primary/30 hover:bg-muted/20"
            )}
          >
            <Upload size={18} className={dragOver ? "text-primary" : "text-muted-foreground"} />
            <div>
              <p className="text-sm font-medium">
                {uploading ? "Uploading…" : "Drop .wasm / .wat here"}
              </p>
              <p className="text-[11px] text-muted-foreground">or click to browse · Max 50 MB</p>
            </div>
          </div>

          {/* Search + sort bar */}
          <div className="flex gap-2">
            <div className="relative flex-1">
              <Search size={13} className="absolute left-2.5 top-1/2 -translate-y-1/2 text-muted-foreground pointer-events-none" />
              <Input
                placeholder="Search tasks…"
                value={filter}
                onChange={(e) => setFilter(e.target.value)}
                className="pl-8 h-8 text-sm"
              />
              {filter && (
                <button onClick={() => setFilter("")} className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground">
                  <X size={13} />
                </button>
              )}
            </div>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => toggleSort("created")}
              className="h-8 shrink-0 text-xs"
            >
              <ArrowUpDown size={12} />
              {sortKey === "created" ? (sortAsc ? "Oldest" : "Newest") : sortKey}
            </Button>
          </div>

          {/* Task list */}
          <Card className="overflow-hidden">
            <ScrollArea className="h-[520px]">
              {filtered.length === 0 ? (
                <div className="flex flex-col items-center justify-center py-16 text-center text-muted-foreground">
                  <FileCode size={32} className="mb-3 opacity-30" />
                  <p className="text-sm font-medium">No tasks found</p>
                  <p className="text-xs mt-1">Upload a .wasm file to get started</p>
                </div>
              ) : (
                <div className="divide-y divide-border">
                  {filtered.map((task) => (
                    <button
                      key={task.id}
                      onClick={() => selectTask(task.id)}
                      className={cn(
                        "w-full flex items-start gap-3 px-3.5 py-3 text-left transition-colors",
                        sel?.id === task.id
                          ? "bg-primary/8 border-l-2 border-primary"
                          : "hover:bg-muted/30 border-l-2 border-transparent"
                      )}
                    >
                      {/* Icon */}
                      <div className={cn(
                        "mt-0.5 flex h-7 w-7 shrink-0 items-center justify-center rounded-md",
                        sel?.id === task.id ? "bg-primary/15 text-primary" : "bg-muted/40 text-muted-foreground"
                      )}>
                        <FileCode size={14} />
                      </div>

                      {/* Info */}
                      <div className="min-w-0 flex-1">
                        <div className="flex items-center gap-2">
                          <span className="text-xs font-medium truncate">{task.name}</span>
                          {statusDot(task.status)}
                        </div>
                        <div className="flex items-center gap-2 mt-0.5 text-[11px] text-muted-foreground">
                          <span>{formatBytes(task.file_size_bytes)}</span>
                          <span>·</span>
                          <span>{timeAgo(task.created_at)}</span>
                        </div>
                      </div>

                      {/* Quick run/stop */}
                      <div
                        className="flex shrink-0 items-center gap-1 ml-1"
                        onClick={(e) => e.stopPropagation()}
                      >
                        {task.status === "running" ? (
                          <button
                            onClick={() => handleStop(task.id)}
                            className="rounded p-1 text-red-400 hover:bg-red-400/10 transition-colors"
                            title="Stop"
                          >
                            <Square size={12} />
                          </button>
                        ) : (
                          <button
                            onClick={() => handleStart(task.id)}
                            disabled={executing === task.id}
                            className="rounded p-1 text-green-400 hover:bg-green-400/10 transition-colors disabled:opacity-40"
                            title="Execute"
                          >
                            {executing === task.id ? (
                              <RefreshCw size={12} className="animate-spin" />
                            ) : (
                              <Play size={12} />
                            )}
                          </button>
                        )}
                      </div>
                    </button>
                  ))}
                </div>
              )}
            </ScrollArea>
          </Card>
        </div>

        {/* ── RIGHT: Detail panel ── */}
        <div className="min-w-0">
          {!selected && !loadingDetail ? (
            <Card className="flex items-center justify-center h-[600px]">
              <div className="text-center text-muted-foreground">
                <FileCode size={40} className="mx-auto mb-3 opacity-20" />
                <p className="text-sm font-medium">Select a task to view details</p>
                <p className="text-xs mt-1 opacity-70">Or upload a new WASM module</p>
              </div>
            </Card>
          ) : loadingDetail ? (
            <Card className="space-y-4 p-6">
              <Skeleton className="h-6 w-1/3" />
              <Skeleton className="h-4 w-1/4" />
              <Skeleton className="h-32 w-full" />
            </Card>
          ) : selected && sel && (
            <Card>
              <CardHeader className="pb-3">
                {/* Task name + rename */}
                <div className="flex items-start justify-between gap-3">
                  <div className="min-w-0 flex-1">
                    {editingName ? (
                      <div className="flex items-center gap-2">
                        <Input
                          value={editName}
                          onChange={(e) => setEditName(e.target.value)}
                          className="h-7 text-sm"
                          autoFocus
                          onKeyDown={(e) => {
                            if (e.key === "Enter") handleUpdateName();
                            if (e.key === "Escape") setEditingName(false);
                          }}
                        />
                        <Button size="sm" className="h-7 text-xs" onClick={handleUpdateName}>Save</Button>
                        <Button size="sm" variant="ghost" className="h-7 text-xs" onClick={() => setEditingName(false)}>Cancel</Button>
                      </div>
                    ) : (
                      <div className="flex items-center gap-2">
                        <CardTitle className="text-base font-semibold truncate">{sel.name}</CardTitle>
                        <button
                          onClick={() => { setEditingName(true); setEditName(sel.name); }}
                          className="shrink-0 rounded p-1 text-muted-foreground hover:text-foreground transition-colors"
                          title="Rename"
                        >
                          <Edit2 size={12} />
                        </button>
                      </div>
                    )}
                    <div className="flex items-center gap-2 mt-1">
                      <Badge variant={statusVariant(sel.status)} className="text-[10px] h-4 px-1.5">
                        {sel.status}
                      </Badge>
                      <span className="text-[11px] text-muted-foreground font-mono">
                        {sel.id.slice(0, 8)}…
                      </span>
                    </div>
                  </div>

                  {/* Action buttons */}
                  <div className="flex items-center gap-1.5 shrink-0">
                    <Link href={`/security?task=${sel.id}`}>
                      <Button variant="outline" size="sm" className="h-7 text-xs gap-1 text-purple-400 border-purple-400/30 hover:bg-purple-400/10">
                        <ShieldAlert size={12} />
                        Security
                      </Button>
                    </Link>
                    {sel.status === "running" ? (
                      <>
                        <Button variant="ghost" size="icon" className="h-7 w-7 text-yellow-400" onClick={() => handlePause(sel.id)} title="Pause">
                          <Pause size={13} />
                        </Button>
                        <Button variant="ghost" size="icon" className="h-7 w-7 text-red-400" onClick={() => handleStop(sel.id)} title="Stop">
                          <Square size={13} />
                        </Button>
                      </>
                    ) : (
                      <>
                        <Button
                          size="sm"
                          className="h-7 text-xs"
                          onClick={() => handleStart(sel.id)}
                          disabled={executing === sel.id}
                        >
                          {executing === sel.id ? (
                            <><RefreshCw size={12} className="animate-spin" />Running…</>
                          ) : (
                            <><Play size={12} />Execute</>
                          )}
                        </Button>
                        {(sel.status === "completed" || sel.status === "failed") && (
                          <Button variant="ghost" size="icon" className="h-7 w-7 text-muted-foreground" onClick={() => handleRestart(sel.id)} title="Re-run">
                            <RotateCcw size={13} />
                          </Button>
                        )}
                        <Button variant="ghost" size="icon" className="h-7 w-7 text-destructive" onClick={() => handleDelete(sel.id)} title="Delete">
                          <Trash2 size={13} />
                        </Button>
                      </>
                    )}
                  </div>
                </div>
              </CardHeader>

              <Separator />

              <CardContent className="pt-4">
                <Tabs value={activeTab} onValueChange={setActiveTab}>
                  <TabsList className="h-8 text-xs mb-4">
                    <TabsTrigger value="overview"  className="text-xs px-3 h-7">Overview</TabsTrigger>
                    <TabsTrigger value="execute"   className="text-xs px-3 h-7">Execute</TabsTrigger>
                    <TabsTrigger value="logs"      className="text-xs px-3 h-7">Logs</TabsTrigger>
                    <TabsTrigger value="history"   className="text-xs px-3 h-7">History</TabsTrigger>
                  </TabsList>

                  {/* Overview tab */}
                  <TabsContent value="overview" className="space-y-4 mt-0">
                    <div className="grid grid-cols-2 gap-2">
                      <MetricPill icon={HardDrive} label="File size"   value={formatBytes(sel.file_size_bytes)} />
                      <MetricPill icon={Clock}     label="Uploaded"    value={timeAgo(sel.created_at)} />
                      {sel.status !== "pending" && (
                        <>
                          <MetricPill icon={Cpu}     label="Status"       value={sel.status} />
                          <MetricPill icon={Activity} label="Priority"     value={sel.priority?.toString() ?? "—"} />
                        </>
                      )}
                    </div>

                    {/* Aggregated metrics from TaskDetail */}
                    {selected.metrics && (
                      <>
                        <Separator />
                        <div>
                          <p className="text-[11px] uppercase tracking-wider text-muted-foreground font-medium mb-2">Execution Stats</p>
                          <div className="grid grid-cols-2 gap-2">
                            <MetricPill icon={Activity}  label="Total runs"     value={selected.metrics.total_runs.toString()} />
                            <MetricPill icon={Zap}        label="Failed runs"   value={selected.metrics.failed_runs.toString()} />
                            <MetricPill icon={Clock}      label="Avg duration"  value={formatDuration(selected.metrics.avg_duration_us)} />
                            <MetricPill icon={Cpu}        label="Total instrs"  value={selected.metrics.total_instructions.toLocaleString()} />
                          </div>
                        </div>
                      </>
                    )}

                    {/* Quick security link */}
                    <Alert variant="info">
                      <ShieldAlert size={14} />
                      <AlertDescription className="flex items-center justify-between">
                        <span>Deep security analysis available</span>
                        <Link href={`/security?task=${sel.id}`} className="flex items-center gap-1 text-primary hover:underline text-xs font-medium">
                          Open Security Hub <ExternalLink size={10} />
                        </Link>
                      </AlertDescription>
                    </Alert>
                  </TabsContent>

                  {/* Execute tab */}
                  <TabsContent value="execute" className="mt-0">
                    {execResult ? (
                      <ExecuteResultPanel result={execResult} />
                    ) : (
                      <div className="flex flex-col items-center justify-center py-12 text-center text-muted-foreground">
                        <TerminalIcon size={32} className="mb-3 opacity-20" />
                        <p className="text-sm font-medium">No execution yet</p>
                        <p className="text-xs mt-1">Click Execute to run this module</p>
                        <Button
                          className="mt-4"
                          size="sm"
                          onClick={() => handleStart(sel.id)}
                          disabled={executing === sel.id}
                        >
                          {executing === sel.id ? (
                            <><RefreshCw size={13} className="animate-spin" />Running…</>
                          ) : (
                            <><Play size={13} />Execute Now</>
                          )}
                        </Button>
                      </div>
                    )}
                  </TabsContent>

                  {/* Logs tab */}
                  <TabsContent value="logs" className="mt-0">
                    {taskLog?.stdout_log && taskLog.stdout_log.length > 0 ? (
                      <ScrollArea className="h-72">
                        <div className="space-y-0.5">
                          {taskLog.stdout_log.map((line, i) => (
                            <div key={i} className="flex gap-2 text-xs font-mono px-1">
                              <span className="text-muted-foreground/50 shrink-0 w-8 text-right">{i + 1}</span>
                              <span className="text-foreground/80">{line}</span>
                            </div>
                          ))}
                        </div>
                      </ScrollArea>
                    ) : taskLog?.error ? (
                      <pre className="text-xs font-mono text-red-400 p-2">{taskLog.error}</pre>
                    ) : (
                      <div className="flex items-center justify-center py-12 text-muted-foreground text-sm">
                        No log output
                      </div>
                    )}
                  </TabsContent>

                  {/* History tab */}
                  <TabsContent value="history" className="mt-0">
                    {historyLoading ? (
                      <div className="space-y-2">
                        {[...Array(4)].map((_, i) => <Skeleton key={i} className="h-10 w-full" />)}
                      </div>
                    ) : execHistory.length === 0 ? (
                      <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
                        <History size={32} className="mb-3 opacity-20" />
                        <p className="text-sm">No execution history</p>
                      </div>
                    ) : (
                      <ScrollArea className="h-72">
                        <div className="divide-y divide-border">
                          {execHistory.map((h, i) => (
                            <div key={i} className="flex items-center gap-3 py-2.5 px-1">
                              {h.success ? (
                                <CheckCircle size={13} className="text-green-400 shrink-0" />
                              ) : (
                                <AlertTriangle size={13} className="text-red-400 shrink-0" />
                              )}
                              <div className="flex-1 min-w-0">
                                <p className="text-xs font-medium">
                                  {h.success ? "Success" : "Failed"}
                                  {h.error && (
                                    <span className="ml-1 text-muted-foreground font-normal text-[11px] truncate">· {h.error}</span>
                                  )}
                                </p>
                                <p className="text-[11px] text-muted-foreground">
                                  {timeAgo(h.started_at)}
                                  {h.duration_us != null && ` · ${formatDuration(h.duration_us)}`}
                                </p>
                              </div>
                              {h.instructions_executed > 0 && (
                                <span className="text-[11px] text-muted-foreground shrink-0">
                                  {h.instructions_executed.toLocaleString()} instrs
                                </span>
                              )}
                              {/* Link to full execution report */}
                              <Link
                                href={`/execution/report?id=${h.execution_id ?? h.id}`}
                                title="View execution report"
                                className="text-muted-foreground hover:text-foreground transition-colors shrink-0"
                              >
                                <FileText size={13} />
                              </Link>
                            </div>
                          ))}
                        </div>
                      </ScrollArea>
                    )}
                  </TabsContent>
                </Tabs>
              </CardContent>
            </Card>
          )}
        </div>
      </div>
    </div>
  );
}
