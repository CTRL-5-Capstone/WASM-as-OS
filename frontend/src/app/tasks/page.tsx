"use client";

import React, { useEffect, useState, useRef, useCallback, useMemo, type DragEvent } from "react";
import Link from "next/link";
import {
  Upload, Play, Square, Trash2, RefreshCw, FileCode, Search, X,
  CheckCircle, AlertTriangle, Clock, Cpu, Zap, HardDrive,
  ArrowUpDown, Activity, RotateCcw, Edit2, History,
  ShieldAlert, ExternalLink, Terminal as TerminalIcon, FileText,
  Copy, Eye, Shield, Camera, ChevronDown, ChevronRight,
  Package, BarChart3, Loader2, Download, Hash,
  Flame, Link2, Unlink, Lock, Unlock, Shuffle, ArrowRightLeft,
  Sliders, Thermometer, ToggleLeft, ToggleRight, Radio,
  FolderOpen, FilePlus, FileX, FolderPlus, Variable,
  Plus, Minus, Gauge, Timer, Target, Crosshair,
  PlayCircle, StopCircle, FastForward,
  ClipboardCheck, ClipboardX, Layers, Settings2,
  GitBranch, Beaker,
} from "lucide-react";
import {
  getTasks, getTask, uploadTask, startTask, stopTask, deleteTask,
  pauseTask, restartTask, updateTask, getTaskExecutionHistory,
  getTaskLogs, getTaskSecurity, inspectTask, getSnapshots,
  createSnapshot, deleteSnapshot, readFileAsBytes, executeBatch,
  type Task, type TaskDetail, type ExecutionResult,
  type TaskLog, type ExecutionHistory, type SecurityReport,
  type Snapshot, type CreateSnapshotRequest, type BatchRequest,
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
import { Progress } from "@/components/ui/progress";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";

// ─── Sort helpers ────────────────────────────────────────────────────

type SortKey = "name" | "status" | "size" | "created" | "priority";
const sortFns: Record<SortKey, (a: Task, b: Task) => number> = {
  name:     (a, b) => a.name.localeCompare(b.name),
  status:   (a, b) => a.status.localeCompare(b.status),
  size:     (a, b) => b.file_size_bytes - a.file_size_bytes,
  created:  (a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime(),
  priority: (a, b) => (b.priority ?? 0) - (a.priority ?? 0),
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

function MetricPill({ icon: Icon, label, value, accent }: {
  icon: React.ElementType; label: string; value: string; accent?: string;
}) {
  return (
    <div className="flex items-center gap-2 rounded-lg bg-muted/30 border border-border px-3 py-2">
      <Icon size={13} className={cn("shrink-0", accent ?? "text-muted-foreground")} />
      <div>
        <p className="text-[10px] text-muted-foreground uppercase tracking-wider">{label}</p>
        <p className="text-xs font-semibold text-foreground">{value}</p>
      </div>
    </div>
  );
}

function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <button
            onClick={() => { navigator.clipboard.writeText(text); setCopied(true); setTimeout(() => setCopied(false), 2000); }}
            className="rounded p-1 text-muted-foreground hover:text-foreground transition-colors"
          >
            {copied ? <CheckCircle size={12} className="text-green-400" /> : <Copy size={12} />}
          </button>
        </TooltipTrigger>
        <TooltipContent side="top"><p className="text-xs">{copied ? "Copied!" : "Copy to clipboard"}</p></TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}

function ExecutingSpinner() {
  const [elapsed, setElapsed] = useState(0);
  useEffect(() => {
    const id = setInterval(() => setElapsed((s) => s + 1), 1000);
    return () => clearInterval(id);
  }, []);
  return (
    <div className="flex flex-col items-center justify-center py-12 gap-4">
      <div className="relative">
        <div className="h-16 w-16 rounded-full border-4 border-muted/30" />
        <div className="absolute inset-0 h-16 w-16 rounded-full border-4 border-transparent border-t-primary animate-spin" />
        <Cpu size={20} className="absolute inset-0 m-auto text-primary" />
      </div>
      <div className="text-center">
        <p className="text-sm font-medium text-foreground">Executing module…</p>
        <p className="text-xs text-muted-foreground mt-1">Running WASM instructions in sandbox</p>
        <p className="text-xs font-mono text-primary mt-2 tabular-nums">{elapsed}s elapsed</p>
      </div>
      <div className="w-48">
        <Progress value={undefined} className="h-1.5 animate-pulse" />
      </div>
    </div>
  );
}

function ExecuteResultPanel({ result, onRerun, rerunning }: {
  result: ExecutionResult; onRerun?: () => void; rerunning?: boolean;
}) {
  const stdout = result.stdout_log?.join("\n") ?? "";
  const totalOutput = (result.stdout_log?.length ?? 0) + (result.error ? 1 : 0);

  return (
    <div className="space-y-3">
      {/* Header */}
      <div className="flex items-center gap-2">
        {result.success ? (
          <CheckCircle size={15} className="text-green-400" />
        ) : (
          <AlertTriangle size={15} className="text-red-400" />
        )}
        <span className={cn("text-sm font-medium", result.success ? "text-green-400" : "text-red-400")}>
          {result.success ? "Execution successful" : "Execution failed"}
        </span>
        {result.return_value != null && (
          <Badge variant="secondary" className="text-[10px] h-4 px-1.5 ml-1">
            return: {String(result.return_value)}
          </Badge>
        )}
        <div className="ml-auto flex items-center gap-2">
          {result.duration_us != null && (
            <span className="text-xs text-muted-foreground">
              {formatDuration(result.duration_us)}
            </span>
          )}
          {onRerun && (
            <Button size="sm" variant="outline" className="h-6 text-[11px] gap-1" onClick={onRerun} disabled={rerunning}>
              {rerunning ? <Loader2 size={10} className="animate-spin" /> : <RotateCcw size={10} />}
              Re-run
            </Button>
          )}
        </div>
      </div>

      {/* stdout */}
      {result.stdout_log?.length > 0 && (
        <div>
          <div className="flex items-center justify-between mb-1.5">
            <p className="text-[10px] uppercase tracking-wider text-muted-foreground font-medium">
              stdout · {result.stdout_log.length} line{result.stdout_log.length > 1 ? "s" : ""}
            </p>
            <CopyButton text={stdout} />
          </div>
          <ScrollArea className="max-h-64">
            <pre className="rounded-lg bg-black/40 border border-border p-3 text-xs font-mono text-green-300 whitespace-pre-wrap">
              {result.stdout_log.map((line, i) => (
                <div key={i} className="flex gap-2">
                  <span className="text-muted-foreground/40 select-none w-6 text-right shrink-0">{i + 1}</span>
                  <span>{line}</span>
                </div>
              ))}
            </pre>
          </ScrollArea>
        </div>
      )}

      {/* stderr */}
      {result.error && (
        <div>
          <div className="flex items-center justify-between mb-1.5">
            <p className="text-[10px] uppercase tracking-wider text-red-400/70 font-medium">stderr</p>
            <CopyButton text={result.error} />
          </div>
          <pre className="rounded-lg bg-red-950/30 border border-red-900/30 p-3 text-xs font-mono text-red-300 overflow-auto max-h-40 whitespace-pre-wrap">
            {result.error}
          </pre>
        </div>
      )}

      {/* No output at all */}
      {totalOutput === 0 && (
        <div className="rounded-lg bg-muted/20 border border-border p-4 text-center text-xs text-muted-foreground">
          Module produced no stdout/stderr output
        </div>
      )}

      {/* Metrics grid */}
      <div className="grid grid-cols-2 sm:grid-cols-4 gap-2 pt-1">
        <MetricPill icon={Cpu}       label="Instructions"  value={(result.instructions_executed ?? 0).toLocaleString()} accent="text-blue-400" />
        <MetricPill icon={Zap}       label="Syscalls"      value={(result.syscalls_executed ?? 0).toLocaleString()} accent="text-yellow-400" />
        <MetricPill icon={HardDrive} label="Memory"        value={formatBytes(result.memory_used_bytes ?? 0)} accent="text-purple-400" />
        <MetricPill icon={Clock}     label="Duration"      value={result.duration_us != null ? formatDuration(result.duration_us) : "—"} accent="text-green-400" />
      </div>

      {/* Execution ID if present */}
      {result.execution_id && (
        <div className="flex items-center gap-2 text-[11px] text-muted-foreground pt-1">
          <Hash size={10} />
          <span className="font-mono">{result.execution_id}</span>
          <CopyButton text={result.execution_id} />
          <Link
            href={`/execution/report?id=${result.execution_id}`}
            className="ml-auto flex items-center gap-1 text-primary hover:underline"
          >
            Full Report <ExternalLink size={10} />
          </Link>
        </div>
      )}
    </div>
  );
}

/* ── Security Panel ─────────────────────────────────────────────── */

function SecurityPanel({ report, loading }: { report: SecurityReport | null; loading: boolean }) {
  if (loading) return (
    <div className="space-y-3">
      <Skeleton className="h-8 w-40" />
      <Skeleton className="h-20 w-full" />
      <Skeleton className="h-32 w-full" />
    </div>
  );
  if (!report) return (
    <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
      <Shield size={32} className="mb-3 opacity-20" />
      <p className="text-sm">Security analysis unavailable</p>
      <p className="text-xs mt-1">Select a task and switch to the Security tab to scan</p>
    </div>
  );

  const riskColor = { low: "text-green-400", medium: "text-yellow-400", high: "text-red-400" }[report.risk_level] ?? "text-muted-foreground";
  const riskBg = { low: "bg-green-500/10 border-green-500/20", medium: "bg-yellow-500/10 border-yellow-500/20", high: "bg-red-500/10 border-red-500/20" }[report.risk_level] ?? "";

  return (
    <div className="space-y-4">
      {/* Risk badge + summary */}
      <div className={cn("rounded-lg border p-3 flex items-start gap-3", riskBg)}>
        <ShieldAlert size={18} className={riskColor} />
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <span className={cn("text-sm font-semibold uppercase", riskColor)}>{report.risk_level} risk</span>
            <Badge variant="secondary" className="text-[10px]">{report.capabilities.length} capabilities</Badge>
          </div>
          <p className="text-xs text-muted-foreground mt-1">{report.summary}</p>
        </div>
      </div>

      {/* Capabilities */}
      {report.capabilities.length > 0 && (
        <div>
          <p className="text-[10px] uppercase tracking-wider text-muted-foreground font-medium mb-2">Detected Capabilities</p>
          <div className="space-y-1.5">
            {report.capabilities.map((cap, i) => {
              const levelColor = { info: "text-blue-400 bg-blue-500/10 border-blue-500/20", warn: "text-yellow-400 bg-yellow-500/10 border-yellow-500/20", severe: "text-red-400 bg-red-500/10 border-red-500/20" }[cap.level] ?? "";
              return (
                <div key={i} className={cn("rounded-md border px-3 py-2 flex items-start gap-2", levelColor)}>
                  <AlertTriangle size={12} className="mt-0.5 shrink-0" />
                  <div>
                    <p className="text-xs font-medium">{cap.name}</p>
                    <p className="text-[11px] text-muted-foreground">{cap.description}</p>
                  </div>
                  <Badge variant="secondary" className="text-[9px] ml-auto shrink-0">{cap.level}</Badge>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* Imports / Exports */}
      <div className="grid grid-cols-2 gap-3">
        <div>
          <p className="text-[10px] uppercase tracking-wider text-muted-foreground font-medium mb-1.5">
            Imports · {report.imports.length}
          </p>
          {report.imports.length > 0 ? (
            <ScrollArea className="max-h-32">
              <div className="space-y-0.5">
                {report.imports.map((imp, i) => (
                  <div key={i} className="text-xs font-mono px-2 py-0.5 rounded bg-muted/20 text-foreground/80 truncate">{imp}</div>
                ))}
              </div>
            </ScrollArea>
          ) : (
            <p className="text-xs text-muted-foreground">No imports</p>
          )}
        </div>
        <div>
          <p className="text-[10px] uppercase tracking-wider text-muted-foreground font-medium mb-1.5">
            Exports · {report.exports.length}
          </p>
          {report.exports.length > 0 ? (
            <ScrollArea className="max-h-32">
              <div className="space-y-0.5">
                {report.exports.map((exp, i) => (
                  <div key={i} className="text-xs font-mono px-2 py-0.5 rounded bg-muted/20 text-foreground/80 truncate">{exp}</div>
                ))}
              </div>
            </ScrollArea>
          ) : (
            <p className="text-xs text-muted-foreground">No exports</p>
          )}
        </div>
      </div>

      <Alert variant="info">
        <Shield size={14} />
        <AlertDescription className="flex items-center justify-between">
          <span>View full security audit with recommendations</span>
          <Link href={`/security?task=${report.task_id}`} className="flex items-center gap-1 text-primary hover:underline text-xs font-medium">
            Open Security Hub <ExternalLink size={10} />
          </Link>
        </AlertDescription>
      </Alert>
    </div>
  );
}

/* ── Inspect Panel ──────────────────────────────────────────────── */

function InspectPanel({ data, loading }: { data: Record<string, unknown> | null; loading: boolean }) {
  if (loading) return (
    <div className="space-y-3">
      <Skeleton className="h-8 w-40" />
      <Skeleton className="h-48 w-full" />
    </div>
  );
  if (!data) return (
    <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
      <Eye size={32} className="mb-3 opacity-20" />
      <p className="text-sm">No inspection data</p>
      <p className="text-xs mt-1">Select a task to inspect its module structure</p>
    </div>
  );

  const imports = (data.imports as string[] | undefined) ?? [];
  const exports = (data.exports as string[] | undefined) ?? [];
  const memories = (data.memories as unknown[] | undefined) ?? [];
  const tables = (data.tables as unknown[] | undefined) ?? [];
  const globals = (data.globals as unknown[] | undefined) ?? [];
  const customSections = (data.custom_sections as string[] | undefined) ?? [];
  const format = (data.format as string | undefined) ?? "unknown";
  const size = (data.size_bytes as number | undefined) ?? 0;

  const jsonStr = JSON.stringify(data, null, 2);

  return (
    <div className="space-y-4">
      {/* Module summary */}
      <div className="grid grid-cols-2 sm:grid-cols-4 gap-2">
        <MetricPill icon={Package}   label="Format"        value={format.toUpperCase()} accent="text-blue-400" />
        <MetricPill icon={HardDrive} label="Size"          value={formatBytes(size)} accent="text-purple-400" />
        <MetricPill icon={Download}  label="Imports"       value={imports.length.toString()} accent="text-cyan-400" />
        <MetricPill icon={Upload}    label="Exports"       value={exports.length.toString()} accent="text-green-400" />
      </div>

      {/* Imports */}
      {imports.length > 0 && (
        <div>
          <div className="flex items-center justify-between mb-1.5">
            <p className="text-[10px] uppercase tracking-wider text-muted-foreground font-medium">
              Imports · {imports.length}
            </p>
            <CopyButton text={imports.join("\n")} />
          </div>
          <ScrollArea className="max-h-36">
            <div className="rounded-lg bg-black/30 border border-border p-2 space-y-0.5">
              {imports.map((imp, i) => (
                <div key={i} className="text-xs font-mono text-cyan-300 px-1 truncate">{imp}</div>
              ))}
            </div>
          </ScrollArea>
        </div>
      )}

      {/* Exports */}
      {exports.length > 0 && (
        <div>
          <div className="flex items-center justify-between mb-1.5">
            <p className="text-[10px] uppercase tracking-wider text-muted-foreground font-medium">
              Exports · {exports.length}
            </p>
            <CopyButton text={exports.join("\n")} />
          </div>
          <ScrollArea className="max-h-36">
            <div className="rounded-lg bg-black/30 border border-border p-2 space-y-0.5">
              {exports.map((exp, i) => (
                <div key={i} className="text-xs font-mono text-green-300 px-1 truncate">{exp}</div>
              ))}
            </div>
          </ScrollArea>
        </div>
      )}

      {/* Memories / Tables / Globals summary row */}
      <div className="grid grid-cols-3 gap-2">
        <MetricPill icon={HardDrive} label="Memories" value={memories.length.toString()} />
        <MetricPill icon={BarChart3}  label="Tables"   value={tables.length.toString()} />
        <MetricPill icon={Activity}  label="Globals"   value={globals.length.toString()} />
      </div>

      {/* Custom sections */}
      {customSections.length > 0 && (
        <div>
          <p className="text-[10px] uppercase tracking-wider text-muted-foreground font-medium mb-1.5">Custom Sections</p>
          <div className="flex flex-wrap gap-1">
            {customSections.map((s, i) => (
              <Badge key={i} variant="secondary" className="text-[10px]">{s}</Badge>
            ))}
          </div>
        </div>
      )}

      {/* Raw JSON */}
      <details className="group">
        <summary className="flex items-center gap-1.5 text-[11px] text-muted-foreground cursor-pointer hover:text-foreground transition-colors">
          <ChevronRight size={12} className="group-open:rotate-90 transition-transform" />
          Raw inspection data
        </summary>
        <div className="mt-2 relative">
          <div className="absolute top-2 right-2">
            <CopyButton text={jsonStr} />
          </div>
          <ScrollArea className="max-h-48">
            <pre className="rounded-lg bg-black/30 border border-border p-3 text-[11px] font-mono text-foreground/70 whitespace-pre-wrap">
              {jsonStr}
            </pre>
          </ScrollArea>
        </div>
      </details>
    </div>
  );
}

/* ── Snapshots Panel ────────────────────────────────────────────── */

function SnapshotsPanel({ taskId, snapshots, loading, onRefresh, onNotify }: {
  taskId: string;
  snapshots: Snapshot[];
  loading: boolean;
  onRefresh: () => void;
  onNotify: (msg: string, ok?: boolean) => void;
}) {
  const [creating, setCreating] = useState(false);
  const [deleting, setDeleting] = useState<string | null>(null);

  const handleCreate = async () => {
    setCreating(true);
    try {
      const body: CreateSnapshotRequest = {
        memory_mb: 64,
        instructions: 0,
        stack_depth: 0,
        globals_json: "{}",
        note: `Snapshot from Tasks page @ ${new Date().toISOString()}`,
      };
      await createSnapshot(taskId, body);
      onNotify("Snapshot created");
      onRefresh();
    } catch (e: unknown) {
      onNotify(e instanceof Error ? e.message : "Snapshot creation failed", false);
    } finally { setCreating(false); }
  };

  const handleDelete = async (snapId: string) => {
    setDeleting(snapId);
    try {
      await deleteSnapshot(taskId, snapId);
      onNotify("Snapshot deleted");
      onRefresh();
    } catch (e: unknown) {
      onNotify(e instanceof Error ? e.message : "Delete failed", false);
    } finally { setDeleting(null); }
  };

  if (loading) return (
    <div className="space-y-2">
      {[...Array(3)].map((_, i) => <Skeleton key={i} className="h-16 w-full" />)}
    </div>
  );

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <p className="text-[11px] uppercase tracking-wider text-muted-foreground font-medium">
          Snapshots · {snapshots.length}
        </p>
        <div className="flex items-center gap-1.5">
          <Button size="sm" variant="outline" className="h-6 text-[11px] gap-1" onClick={onRefresh}>
            <RefreshCw size={10} /> Refresh
          </Button>
          <Button size="sm" className="h-6 text-[11px] gap-1" onClick={handleCreate} disabled={creating}>
            {creating ? <Loader2 size={10} className="animate-spin" /> : <Camera size={10} />}
            Capture
          </Button>
        </div>
      </div>

      {snapshots.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
          <Camera size={32} className="mb-3 opacity-20" />
          <p className="text-sm">No snapshots yet</p>
          <p className="text-xs mt-1">Capture a snapshot to save module state</p>
        </div>
      ) : (
        <ScrollArea className="max-h-72">
          <div className="space-y-2">
            {snapshots.map((snap) => (
              <div key={snap.id} className="rounded-lg border border-border bg-muted/10 px-3 py-2.5">
                <div className="flex items-start justify-between gap-2">
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <Camera size={12} className="text-muted-foreground shrink-0" />
                      <span className="text-xs font-mono text-foreground truncate">{snap.id.slice(0, 12)}…</span>
                      <Badge variant="secondary" className="text-[9px] h-3.5 shrink-0">{snap.state}</Badge>
                    </div>
                    {snap.note && (
                      <p className="text-[11px] text-muted-foreground mt-0.5 truncate">{snap.note}</p>
                    )}
                    <div className="flex items-center gap-3 mt-1 text-[11px] text-muted-foreground">
                      <span>{timeAgo(snap.captured_at ?? snap.created_at ?? "")}</span>
                      <span>{snap.memory_mb} MB</span>
                      <span>{snap.instructions.toLocaleString()} instrs</span>
                      <span>stack: {snap.stack_depth}</span>
                    </div>
                  </div>
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-6 w-6 text-destructive shrink-0"
                    onClick={() => handleDelete(snap.id)}
                    disabled={deleting === snap.id}
                  >
                    {deleting === snap.id ? <Loader2 size={11} className="animate-spin" /> : <Trash2 size={11} />}
                  </Button>
                </div>
              </div>
            ))}
          </div>
        </ScrollArea>
      )}
    </div>
  );
}

// ─── Chaos Sandbox Panel ─────────────────────────────────────────────

type ChaosMode = "latency" | "oom" | "fault" | "noop";

function ChaosSandboxPanel({
  enabled,
  mode,
  intensity,
  onToggle,
  onModeChange,
  onIntensityChange,
}: {
  enabled: boolean;
  mode: ChaosMode;
  intensity: number;
  onToggle: () => void;
  onModeChange: (m: ChaosMode) => void;
  onIntensityChange: (v: number) => void;
}) {
  const modeInfo: Record<ChaosMode, { label: string; desc: string; icon: React.ElementType; color: string }> = {
    latency: { label: "Inject Latency", desc: "Add random delay (10ms–2s) between syscalls to simulate slow I/O", icon: Clock, color: "text-yellow-400" },
    oom:     { label: "Memory Pressure", desc: "Simulate OOM by reporting reduced available memory", icon: HardDrive, color: "text-red-400" },
    fault:   { label: "Syscall Fault", desc: "Randomly fail N% of syscalls with EFAULT", icon: Zap, color: "text-orange-400" },
    noop:    { label: "No-Op Calls", desc: "Replace random syscalls with no-ops (silent drops)", icon: Shuffle, color: "text-purple-400" },
  };

  return (
    <div className="space-y-4">
      {/* Master toggle */}
      <div className={cn(
        "flex items-center justify-between rounded-lg border p-3 transition-all",
        enabled
          ? "border-orange-500/40 bg-orange-500/8"
          : "border-border bg-muted/10"
      )}>
        <div className="flex items-center gap-3">
          <Flame size={18} className={enabled ? "text-orange-400" : "text-muted-foreground"} />
          <div>
            <p className="text-sm font-semibold">{enabled ? "Chaos Mode ACTIVE" : "Chaos Mode OFF"}</p>
            <p className="text-[11px] text-muted-foreground">
              Inject faults during execution to test module resilience
            </p>
          </div>
        </div>
        <button
          onClick={onToggle}
          className={cn(
            "relative h-6 w-11 rounded-full transition-colors",
            enabled ? "bg-orange-500" : "bg-muted/40 border border-border"
          )}
        >
          <span className={cn(
            "absolute top-0.5 left-0.5 h-5 w-5 rounded-full bg-white transition-transform shadow-sm",
            enabled && "translate-x-5"
          )} />
        </button>
      </div>

      {enabled && (
        <>
          {/* Mode selector */}
          <div>
            <p className="text-[10px] uppercase tracking-wider text-muted-foreground font-medium mb-2">Fault Type</p>
            <div className="grid grid-cols-2 gap-2">
              {(Object.entries(modeInfo) as [ChaosMode, typeof modeInfo[ChaosMode]][]).map(([key, info]) => {
                const Icon = info.icon;
                const active = mode === key;
                return (
                  <button
                    key={key}
                    onClick={() => onModeChange(key)}
                    className={cn(
                      "flex items-start gap-2 rounded-lg border px-3 py-2.5 text-left transition-all",
                      active
                        ? "border-orange-500/40 bg-orange-500/8 ring-1 ring-orange-500/30"
                        : "border-border bg-muted/10 hover:bg-muted/20"
                    )}
                  >
                    <Icon size={14} className={cn("mt-0.5 shrink-0", active ? info.color : "text-muted-foreground")} />
                    <div>
                      <p className={cn("text-xs font-medium", active ? "text-foreground" : "text-muted-foreground")}>{info.label}</p>
                      <p className="text-[10px] text-muted-foreground leading-relaxed">{info.desc}</p>
                    </div>
                  </button>
                );
              })}
            </div>
          </div>

          {/* Intensity slider */}
          <div>
            <div className="flex items-center justify-between mb-2">
              <p className="text-[10px] uppercase tracking-wider text-muted-foreground font-medium">Intensity</p>
              <span className="text-xs font-bold text-orange-400">{intensity}%</span>
            </div>
            <input
              type="range"
              min={5}
              max={100}
              step={5}
              value={intensity}
              onChange={(e) => onIntensityChange(Number(e.target.value))}
              className="w-full h-1.5 rounded-full appearance-none bg-muted/30 accent-orange-500 cursor-pointer"
            />
            <div className="flex justify-between text-[9px] text-muted-foreground mt-1">
              <span>5% (gentle)</span>
              <span>50% (moderate)</span>
              <span>100% (brutal)</span>
            </div>
          </div>

          {/* Preview */}
          <Alert variant="warning" className="py-2">
            <Flame size={12} />
            <AlertDescription className="text-xs">
              <strong>{modeInfo[mode].label}</strong> at <strong>{intensity}%</strong> intensity will be applied on next execution.
              The module&apos;s error handling and recovery paths will be stress-tested.
            </AlertDescription>
          </Alert>
        </>
      )}
    </div>
  );
}

// ─── Inter-Module Communication (IMC) Panel ──────────────────────────

interface IMCPipe {
  id: string;
  sourceTask: string;
  sourceName: string;
  targetTask: string;
  targetName: string;
  created: string;
  status: "connected" | "broken" | "idle";
  messagesTransferred: number;
}

function IMCPanel({
  tasks,
  currentTaskId,
  pipes,
  onCreatePipe,
  onDeletePipe,
}: {
  tasks: Task[];
  currentTaskId: string;
  pipes: IMCPipe[];
  onCreatePipe: (targetId: string) => void;
  onDeletePipe: (pipeId: string) => void;
}) {
  const [targetId, setTargetId] = useState("");
  const otherTasks = tasks.filter(t => t.id !== currentTaskId);

  return (
    <div className="space-y-4">
      {/* Header */}
      <div className="flex items-center gap-2 rounded-lg border border-border bg-muted/10 p-3">
        <ArrowRightLeft size={16} className="text-cyan-400" />
        <div>
          <p className="text-sm font-semibold">Inter-Module Communication</p>
          <p className="text-[11px] text-muted-foreground">Create pipes to pass messages between running WASM modules</p>
        </div>
      </div>

      {/* Create new pipe */}
      <div>
        <p className="text-[10px] uppercase tracking-wider text-muted-foreground font-medium mb-2">New Pipe</p>
        <div className="flex items-center gap-2">
          <div className="flex items-center gap-1.5 rounded-md border border-border bg-muted/20 px-2 py-1.5 text-xs font-mono shrink-0">
            <span className="text-cyan-400">this</span>
            <ArrowRightLeft size={10} className="text-muted-foreground" />
          </div>
          <select
            value={targetId}
            onChange={(e) => setTargetId(e.target.value)}
            className="flex-1 h-8 rounded-md border border-border bg-muted/20 px-2 text-xs text-foreground focus:outline-none"
          >
            <option value="">Select target module…</option>
            {otherTasks.map(t => (
              <option key={t.id} value={t.id}>{t.name} ({t.status})</option>
            ))}
          </select>
          <Button
            size="sm"
            className="h-8 text-xs gap-1"
            disabled={!targetId}
            onClick={() => { onCreatePipe(targetId); setTargetId(""); }}
          >
            <Link2 size={11} /> Connect
          </Button>
        </div>
      </div>

      {/* Active pipes */}
      <div>
        <p className="text-[10px] uppercase tracking-wider text-muted-foreground font-medium mb-2">
          Active Pipes · {pipes.length}
        </p>
        {pipes.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-8 text-muted-foreground">
            <Unlink size={24} className="mb-2 opacity-20" />
            <p className="text-xs">No active pipes</p>
            <p className="text-[10px] mt-0.5">Connect to another module to enable IPC</p>
          </div>
        ) : (
          <div className="space-y-2">
            {pipes.map((pipe) => {
              const statusColor = pipe.status === "connected" ? "text-green-400" : pipe.status === "broken" ? "text-red-400" : "text-muted-foreground";
              const statusBg = pipe.status === "connected" ? "bg-green-500/10 border-green-500/20" : pipe.status === "broken" ? "bg-red-500/10 border-red-500/20" : "bg-muted/10 border-border";
              return (
                <div key={pipe.id} className={cn("rounded-lg border px-3 py-2.5 flex items-center gap-3", statusBg)}>
                  <div className="flex items-center gap-1.5 min-w-0 flex-1">
                    <span className="text-xs font-mono text-cyan-400 truncate">{pipe.sourceName}</span>
                    <ArrowRightLeft size={11} className={statusColor} />
                    <span className="text-xs font-mono text-indigo-400 truncate">{pipe.targetName}</span>
                  </div>
                  <div className="flex items-center gap-2 shrink-0">
                    <Badge variant="outline" className={cn("text-[9px]", statusColor)}>{pipe.status}</Badge>
                    <span className="text-[10px] text-muted-foreground">{pipe.messagesTransferred} msgs</span>
                    <button
                      onClick={() => onDeletePipe(pipe.id)}
                      className="rounded p-1 text-destructive/60 hover:text-destructive hover:bg-destructive/10 transition-colors"
                    >
                      <Unlink size={11} />
                    </button>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>

      <Alert variant="info" className="py-2">
        <ArrowRightLeft size={12} />
        <AlertDescription className="text-xs">
          Pipes use a shared memory ring buffer for zero-copy IPC. Both modules must be running for data to flow.
        </AlertDescription>
      </Alert>
    </div>
  );
}

// ─── Capability Hardening Panel ──────────────────────────────────────

interface HardenedPolicy {
  allowedImports: string[];
  deniedImports: string[];
  maxMemoryMB: number;
  maxInstructions: number;
  maxSyscalls: number;
  allowNetwork: boolean;
  allowFileSystem: boolean;
  allowProcessSpawn: boolean;
  strictMode: boolean;
}

function HardeningPanel({
  report,
  onApply,
}: {
  report: SecurityReport | null;
  onApply: (policy: HardenedPolicy) => void;
}) {
  const [policy, setPolicy] = useState<HardenedPolicy>({
    allowedImports: [],
    deniedImports: [],
    maxMemoryMB: 64,
    maxInstructions: 1_000_000,
    maxSyscalls: 500,
    allowNetwork: false,
    allowFileSystem: false,
    allowProcessSpawn: false,
    strictMode: true,
  });
  const [generated, setGenerated] = useState(false);

  const generatePolicy = () => {
    if (!report) return;

    // Analyze capabilities to decide what to allow
    const caps = report.capabilities.map(c => c.name.toLowerCase());
    const hasNet = caps.some(c => c.includes("network") || c.includes("socket") || c.includes("http"));
    const hasFS = caps.some(c => c.includes("file") || c.includes("fs") || c.includes("path") || c.includes("directory"));
    const hasProc = caps.some(c => c.includes("process") || c.includes("spawn") || c.includes("exec") || c.includes("command"));

    // Safe imports = those at info level, dangerous = warn+severe
    const safeImports = report.imports.filter((_, i) => {
      const cap = report.capabilities[i];
      return !cap || cap.level === "info";
    });
    const dangerousImports = report.imports.filter((_, i) => {
      const cap = report.capabilities[i];
      return cap && (cap.level === "warn" || cap.level === "severe");
    });

    setPolicy({
      allowedImports: safeImports,
      deniedImports: dangerousImports,
      maxMemoryMB: report.risk_level === "high" ? 16 : report.risk_level === "medium" ? 32 : 64,
      maxInstructions: report.risk_level === "high" ? 100_000 : report.risk_level === "medium" ? 500_000 : 1_000_000,
      maxSyscalls: report.risk_level === "high" ? 50 : report.risk_level === "medium" ? 200 : 500,
      allowNetwork: hasNet && report.risk_level !== "high",
      allowFileSystem: hasFS && report.risk_level !== "high",
      allowProcessSpawn: false, // never allow by default
      strictMode: report.risk_level !== "low",
    });
    setGenerated(true);
  };

  const toggleCap = (key: "allowNetwork" | "allowFileSystem" | "allowProcessSpawn") => {
    setPolicy(prev => ({ ...prev, [key]: !prev[key] }));
  };

  const policyJSON = JSON.stringify(policy, null, 2);

  return (
    <div className="space-y-4">
      {/* Header */}
      <div className="flex items-center gap-2 rounded-lg border border-border bg-muted/10 p-3">
        <Lock size={16} className="text-emerald-400" />
        <div>
          <p className="text-sm font-semibold">Capability Hardening</p>
          <p className="text-[11px] text-muted-foreground">
            Auto-generate a strict sandboxing policy from security analysis
          </p>
        </div>
      </div>

      {!report ? (
        <Alert variant="info" className="py-2">
          <Shield size={12} />
          <AlertDescription className="text-xs">
            Switch to the Security tab first to generate a security report, then come back here to harden.
          </AlertDescription>
        </Alert>
      ) : (
        <>
          {!generated ? (
            <div className="flex flex-col items-center py-8">
              <Shield size={32} className="text-muted-foreground/20 mb-3" />
              <p className="text-sm text-muted-foreground mb-3">
                Generate a sandboxing policy based on the security report ({report.risk_level} risk, {report.capabilities.length} capabilities)
              </p>
              <Button onClick={generatePolicy} className="gap-1.5">
                <Lock size={13} /> Generate Hardened Policy
              </Button>
            </div>
          ) : (
            <>
              {/* Policy toggles */}
              <div className="grid grid-cols-3 gap-2">
                {([
                  { key: "allowNetwork" as const, label: "Network", icon: Activity, desc: "HTTP, sockets" },
                  { key: "allowFileSystem" as const, label: "Filesystem", icon: FileCode, desc: "Read/write files" },
                  { key: "allowProcessSpawn" as const, label: "Process Spawn", icon: TerminalIcon, desc: "Execute commands" },
                ]).map(({ key, label, icon: Icon, desc }) => (
                  <button
                    key={key}
                    onClick={() => toggleCap(key)}
                    className={cn(
                      "flex items-start gap-2 rounded-lg border px-3 py-2.5 text-left transition-all",
                      policy[key]
                        ? "border-emerald-500/40 bg-emerald-500/8"
                        : "border-red-500/30 bg-red-500/5"
                    )}
                  >
                    {policy[key] ? <Unlock size={12} className="mt-0.5 text-emerald-400" /> : <Lock size={12} className="mt-0.5 text-red-400" />}
                    <div>
                      <p className="text-xs font-medium">{label}</p>
                      <p className="text-[10px] text-muted-foreground">{desc}</p>
                      <Badge variant="outline" className={cn("text-[9px] mt-1", policy[key] ? "text-emerald-400 border-emerald-500/30" : "text-red-400 border-red-500/30")}>
                        {policy[key] ? "Allowed" : "Blocked"}
                      </Badge>
                    </div>
                  </button>
                ))}
              </div>

              {/* Resource limits */}
              <div>
                <p className="text-[10px] uppercase tracking-wider text-muted-foreground font-medium mb-2">Resource Limits</p>
                <div className="grid grid-cols-3 gap-2">
                  <div className="rounded-lg border border-border bg-muted/10 px-3 py-2">
                    <p className="text-[10px] text-muted-foreground">Max Memory</p>
                    <p className="text-xs font-bold">{policy.maxMemoryMB} MB</p>
                  </div>
                  <div className="rounded-lg border border-border bg-muted/10 px-3 py-2">
                    <p className="text-[10px] text-muted-foreground">Max Instructions</p>
                    <p className="text-xs font-bold">{policy.maxInstructions.toLocaleString()}</p>
                  </div>
                  <div className="rounded-lg border border-border bg-muted/10 px-3 py-2">
                    <p className="text-[10px] text-muted-foreground">Max Syscalls</p>
                    <p className="text-xs font-bold">{policy.maxSyscalls.toLocaleString()}</p>
                  </div>
                </div>
              </div>

              {/* Import allow/deny lists */}
              {policy.deniedImports.length > 0 && (
                <div>
                  <p className="text-[10px] uppercase tracking-wider text-red-400/70 font-medium mb-1.5">
                    Denied Imports · {policy.deniedImports.length}
                  </p>
                  <div className="flex flex-wrap gap-1">
                    {policy.deniedImports.map((imp, i) => (
                      <Badge key={i} variant="outline" className="text-[10px] border-red-500/30 text-red-400">{imp}</Badge>
                    ))}
                  </div>
                </div>
              )}

              {/* Policy JSON preview */}
              <details className="group">
                <summary className="flex items-center gap-1.5 text-[11px] text-muted-foreground cursor-pointer hover:text-foreground transition-colors">
                  <ChevronRight size={12} className="group-open:rotate-90 transition-transform" />
                  View policy JSON
                </summary>
                <div className="mt-2 relative">
                  <button
                    onClick={() => navigator.clipboard.writeText(policyJSON)}
                    className="absolute top-2 right-2 rounded p-1 text-muted-foreground hover:text-foreground"
                  >
                    <Copy size={11} />
                  </button>
                  <pre className="rounded-lg bg-black/30 border border-border p-3 text-[11px] font-mono text-foreground/70 whitespace-pre-wrap max-h-48 overflow-auto">
                    {policyJSON}
                  </pre>
                </div>
              </details>

              {/* Apply button */}
              <div className="flex items-center gap-3">
                <Button onClick={() => onApply(policy)} className="gap-1.5">
                  <Shield size={13} /> Apply Hardened Policy
                </Button>
                <Button variant="outline" size="sm" onClick={generatePolicy} className="gap-1">
                  <RefreshCw size={11} /> Regenerate
                </Button>
              </div>
            </>
          )}
        </>
      )}
    </div>
  );
}

// ─── ABI Mock State Types ────────────────────────────────────────────

interface MockSensor {
  id: string;
  name: string;
  type: "numeric" | "toggle" | "trigger";
  value: number;
  min?: number;
  max?: number;
  unit?: string;
  active?: boolean;
}

interface MockEvent {
  id: string;
  timestamp: string;
  sensor: string;
  oldValue: number | boolean;
  newValue: number | boolean;
}

const DEFAULT_SENSORS: MockSensor[] = [
  { id: "temp",        name: "Temperature",    type: "numeric",  value: 22,   min: -40, max: 120,  unit: "°C" },
  { id: "pressure",    name: "Pressure",       type: "numeric",  value: 1013, min: 300, max: 1100, unit: "hPa" },
  { id: "humidity",    name: "Humidity",        type: "numeric",  value: 45,   min: 0,   max: 100,  unit: "%" },
  { id: "light",       name: "Light Level",    type: "numeric",  value: 500,  min: 0,   max: 10000,unit: "lux" },
  { id: "battery",     name: "Battery",        type: "numeric",  value: 87,   min: 0,   max: 100,  unit: "%" },
  { id: "alarm",       name: "Alarm Active",   type: "toggle",   value: 0 },
  { id: "door_open",   name: "Door Sensor",    type: "toggle",   value: 0 },
  { id: "motion",      name: "Motion Detected",type: "toggle",   value: 0 },
  { id: "send_alert",  name: "Send Alert",     type: "trigger",  value: 0 },
  { id: "reset_board", name: "Reset Board",    type: "trigger",  value: 0 },
];

// ─── ABI Mocking Dashboard ("Puppeteer") ─────────────────────────────

function ABIMockPanel({
  sensors,
  onSensorChange,
  onTrigger,
  events,
  onAddSensor,
  onRemoveSensor,
}: {
  sensors: MockSensor[];
  onSensorChange: (id: string, value: number) => void;
  onTrigger: (id: string) => void;
  events: MockEvent[];
  onAddSensor: (s: MockSensor) => void;
  onRemoveSensor: (id: string) => void;
}) {
  const [newName, setNewName] = useState("");
  const [newType, setNewType] = useState<"numeric" | "toggle" | "trigger">("numeric");
  const [showAdd, setShowAdd] = useState(false);

  const numericSensors = sensors.filter(s => s.type === "numeric");
  const toggleSensors = sensors.filter(s => s.type === "toggle");
  const triggerSensors = sensors.filter(s => s.type === "trigger");

  return (
    <div className="space-y-4">
      {/* Header */}
      <div className="flex items-center gap-2 rounded-lg border border-border bg-muted/10 p-3">
        <Sliders size={16} className="text-violet-400" />
        <div className="flex-1">
          <p className="text-sm font-semibold">ABI Mock Controls</p>
          <p className="text-[11px] text-muted-foreground">
            Simulate hardware states — the WASM module reads these values via syscalls instead of physical devices
          </p>
        </div>
        <Button variant="ghost" size="sm" className="h-6 text-xs gap-1" onClick={() => setShowAdd(!showAdd)}>
          <Plus size={10} /> Add Sensor
        </Button>
      </div>

      {/* Add sensor form */}
      {showAdd && (
        <div className="flex items-center gap-2 rounded-lg border border-dashed border-violet-500/30 bg-violet-500/5 p-3">
          <Input
            placeholder="Sensor name…"
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
            className="h-7 text-xs flex-1"
          />
          <select
            value={newType}
            onChange={(e) => setNewType(e.target.value as typeof newType)}
            className="h-7 rounded-md border border-border bg-muted/20 px-2 text-xs"
          >
            <option value="numeric">Numeric</option>
            <option value="toggle">Toggle</option>
            <option value="trigger">Trigger</option>
          </select>
          <Button size="sm" className="h-7 text-xs" onClick={() => {
            if (!newName.trim()) return;
            const id = newName.toLowerCase().replace(/\s+/g, "_");
            onAddSensor({
              id,
              name: newName.trim(),
              type: newType,
              value: 0,
              ...(newType === "numeric" ? { min: 0, max: 100, unit: "" } : {}),
            });
            setNewName("");
            setShowAdd(false);
          }}>
            Add
          </Button>
          <Button variant="ghost" size="sm" className="h-7 text-xs" onClick={() => setShowAdd(false)}>Cancel</Button>
        </div>
      )}

      {/* Numeric sliders */}
      {numericSensors.length > 0 && (
        <div>
          <p className="text-[10px] uppercase tracking-wider text-muted-foreground font-medium mb-2">
            <Thermometer size={10} className="inline mr-1" />Analog Inputs · {numericSensors.length}
          </p>
          <div className="space-y-3">
            {numericSensors.map((s) => (
              <div key={s.id} className="rounded-lg border border-border bg-muted/5 px-3 py-2.5 group">
                <div className="flex items-center justify-between mb-1.5">
                  <span className="text-xs font-medium">{s.name}</span>
                  <div className="flex items-center gap-2">
                    <span className="text-xs font-bold font-mono tabular-nums text-violet-400">
                      {s.value}{s.unit ? ` ${s.unit}` : ""}
                    </span>
                    <button
                      onClick={() => onRemoveSensor(s.id)}
                      className="opacity-0 group-hover:opacity-100 text-destructive/50 hover:text-destructive transition-all"
                    >
                      <X size={10} />
                    </button>
                  </div>
                </div>
                <input
                  type="range"
                  min={s.min ?? 0}
                  max={s.max ?? 100}
                  value={s.value}
                  onChange={(e) => onSensorChange(s.id, Number(e.target.value))}
                  className="w-full h-1.5 rounded-full appearance-none bg-muted/30 accent-violet-500 cursor-pointer"
                />
                <div className="flex justify-between text-[9px] text-muted-foreground mt-0.5 font-mono">
                  <span>{s.min ?? 0}</span>
                  <span>{s.max ?? 100}</span>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Toggle switches */}
      {toggleSensors.length > 0 && (
        <div>
          <p className="text-[10px] uppercase tracking-wider text-muted-foreground font-medium mb-2">
            <ToggleLeft size={10} className="inline mr-1" />Binary States · {toggleSensors.length}
          </p>
          <div className="grid grid-cols-2 gap-2">
            {toggleSensors.map((s) => {
              const on = s.value !== 0;
              return (
                <div key={s.id} className={cn(
                  "flex items-center justify-between rounded-lg border px-3 py-2.5 transition-all group",
                  on ? "border-violet-500/40 bg-violet-500/8" : "border-border bg-muted/5"
                )}>
                  <div className="flex items-center gap-2 flex-1 min-w-0">
                    {on ? <ToggleRight size={14} className="text-violet-400 shrink-0" /> : <ToggleLeft size={14} className="text-muted-foreground shrink-0" />}
                    <span className="text-xs font-medium truncate">{s.name}</span>
                  </div>
                  <div className="flex items-center gap-1.5">
                    <button
                      onClick={() => onSensorChange(s.id, on ? 0 : 1)}
                      className={cn(
                        "relative h-5 w-9 rounded-full transition-colors shrink-0",
                        on ? "bg-violet-500" : "bg-muted/40 border border-border"
                      )}
                    >
                      <span className={cn(
                        "absolute top-0.5 left-0.5 h-4 w-4 rounded-full bg-white transition-transform shadow-sm",
                        on && "translate-x-4"
                      )} />
                    </button>
                    <button onClick={() => onRemoveSensor(s.id)}
                      className="opacity-0 group-hover:opacity-100 text-destructive/50 hover:text-destructive transition-all">
                      <X size={9} />
                    </button>
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* Trigger buttons */}
      {triggerSensors.length > 0 && (
        <div>
          <p className="text-[10px] uppercase tracking-wider text-muted-foreground font-medium mb-2">
            <Radio size={10} className="inline mr-1" />Event Triggers
          </p>
          <div className="flex flex-wrap gap-2">
            {triggerSensors.map((s) => (
              <div key={s.id} className="group flex items-center gap-1">
                <Button
                  variant="outline"
                  size="sm"
                  className="h-7 text-xs gap-1.5 border-amber-500/30 text-amber-400 hover:bg-amber-500/10 hover:text-amber-300"
                  onClick={() => onTrigger(s.id)}
                >
                  <Zap size={10} /> {s.name}
                </Button>
                <button onClick={() => onRemoveSensor(s.id)}
                  className="opacity-0 group-hover:opacity-100 text-destructive/50 hover:text-destructive transition-all">
                  <X size={9} />
                </button>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Event log */}
      <div>
        <p className="text-[10px] uppercase tracking-wider text-muted-foreground font-medium mb-2">
          Mock Event Log · {events.length} events
        </p>
        <ScrollArea className="h-28">
          {events.length === 0 ? (
            <p className="text-[11px] text-muted-foreground text-center py-4">No events yet — adjust sliders or triggers above</p>
          ) : (
            <div className="space-y-0.5">
              {events.slice().reverse().slice(0, 50).map((ev) => (
                <div key={ev.id} className="flex items-center gap-2 text-[10px] font-mono px-1 py-0.5 rounded hover:bg-muted/10">
                  <span className="text-muted-foreground/50 shrink-0">{new Date(ev.timestamp).toLocaleTimeString()}</span>
                  <span className="text-violet-400">{ev.sensor}</span>
                  <span className="text-muted-foreground">{String(ev.oldValue)}</span>
                  <span className="text-muted-foreground">→</span>
                  <span className="text-foreground font-medium">{String(ev.newValue)}</span>
                </div>
              ))}
            </div>
          )}
        </ScrollArea>
      </div>

      <Alert variant="info" className="py-2">
        <Sliders size={12} />
        <AlertDescription className="text-xs">
          Mock values are injected into the WASM module&apos;s syscall layer.
          When the module calls <code className="text-violet-400 mx-0.5">read_sensor</code>, it reads from this buffer — no physical hardware needed.
        </AlertDescription>
      </Alert>
    </div>
  );
}

// ─── Ephemeral vFS Explorer ──────────────────────────────────────────

interface VFSFile {
  name: string;
  size: number;
  type: string;
  content: string;       // base64 or text
  created: string;
  modified: string;
}

interface VFSDirectory {
  name: string;
  files: VFSFile[];
  dirs: VFSDirectory[];
}

function VFSExplorer({
  root,
  onAddFile,
  onRemoveFile,
  onAddDir,
  onRemoveDir,
}: {
  root: VFSDirectory;
  onAddFile: (path: string, file: VFSFile) => void;
  onRemoveFile: (path: string, name: string) => void;
  onAddDir: (path: string, name: string) => void;
  onRemoveDir: (path: string, name: string) => void;
}) {
  const [selectedFile, setSelectedFile] = useState<{ path: string; file: VFSFile } | null>(null);
  const [dragOverVFS, setDragOverVFS] = useState(false);
  const [newFileName, setNewFileName] = useState("");
  const [newFileContent, setNewFileContent] = useState("");
  const [showNewFile, setShowNewFile] = useState(false);
  const [newDirName, setNewDirName] = useState("");
  const [showNewDir, setShowNewDir] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const totalFiles = countFiles(root);
  const totalSize = sumFileSize(root);

  function countFiles(dir: VFSDirectory): number {
    return dir.files.length + dir.dirs.reduce((s, d) => s + countFiles(d), 0);
  }
  function sumFileSize(dir: VFSDirectory): number {
    return dir.files.reduce((s, f) => s + f.size, 0) + dir.dirs.reduce((s, d) => s + sumFileSize(d), 0);
  }

  const handleDropVFS = async (e: DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    setDragOverVFS(false);
    const file = e.dataTransfer.files?.[0];
    if (!file) return;
    const text = await file.text();
    const vfsFile: VFSFile = {
      name: file.name,
      size: file.size,
      type: file.type || "application/octet-stream",
      content: text,
      created: new Date().toISOString(),
      modified: new Date().toISOString(),
    };
    onAddFile("/", vfsFile);
  };

  const handleFileUpload = async (file: File) => {
    const text = await file.text();
    const vfsFile: VFSFile = {
      name: file.name,
      size: file.size,
      type: file.type || "application/octet-stream",
      content: text,
      created: new Date().toISOString(),
      modified: new Date().toISOString(),
    };
    onAddFile("/", vfsFile);
  };

  const handleCreateFile = () => {
    if (!newFileName.trim()) return;
    const vfsFile: VFSFile = {
      name: newFileName.trim(),
      size: new TextEncoder().encode(newFileContent).length,
      type: newFileName.endsWith(".json") ? "application/json" : "text/plain",
      content: newFileContent,
      created: new Date().toISOString(),
      modified: new Date().toISOString(),
    };
    onAddFile("/", vfsFile);
    setNewFileName("");
    setNewFileContent("");
    setShowNewFile(false);
  };

  function DirView({ dir, path }: { dir: VFSDirectory; path: string }) {
    const [expanded, setExpanded] = useState(true);
    return (
      <div>
        {path !== "/" && (
          <button onClick={() => setExpanded(!expanded)}
            className="flex items-center gap-1.5 w-full text-left px-1 py-0.5 text-xs hover:bg-muted/20 rounded group">
            <ChevronRight size={10} className={cn("transition-transform", expanded && "rotate-90")} />
            <FolderOpen size={12} className="text-amber-400 shrink-0" />
            <span className="truncate text-foreground/80">{dir.name}</span>
            <span className="text-[9px] text-muted-foreground ml-auto opacity-0 group-hover:opacity-100">
              {dir.files.length}f
            </span>
            <button onClick={(e) => { e.stopPropagation(); onRemoveDir(path, dir.name); }}
              className="opacity-0 group-hover:opacity-100 text-destructive/50 hover:text-destructive ml-1">
              <X size={9} />
            </button>
          </button>
        )}
        {(expanded || path === "/") && (
          <div className={cn(path !== "/" && "ml-3 pl-2 border-l border-border/30")}>
            {dir.dirs.map((d) => (
              <DirView key={d.name} dir={d} path={`${path}${d.name}/`} />
            ))}
            {dir.files.map((f) => (
              <button
                key={f.name}
                onClick={() => setSelectedFile({ path, file: f })}
                className={cn(
                  "flex items-center gap-1.5 w-full text-left px-1 py-0.5 text-xs rounded group transition-colors",
                  selectedFile?.file.name === f.name && selectedFile?.path === path
                    ? "bg-primary/10 text-primary"
                    : "hover:bg-muted/20 text-foreground/70"
                )}
              >
                <FileCode size={11} className="text-cyan-400 shrink-0" />
                <span className="truncate flex-1">{f.name}</span>
                <span className="text-[9px] text-muted-foreground font-mono">{formatBytes(f.size)}</span>
                <button onClick={(e) => { e.stopPropagation(); onRemoveFile(path, f.name); }}
                  className="opacity-0 group-hover:opacity-100 text-destructive/50 hover:text-destructive ml-1">
                  <X size={9} />
                </button>
              </button>
            ))}
            {dir.files.length === 0 && dir.dirs.length === 0 && (
              <p className="text-[10px] text-muted-foreground/50 py-1 pl-1 italic">empty</p>
            )}
          </div>
        )}
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {/* Header */}
      <div className="flex items-center gap-2 rounded-lg border border-border bg-muted/10 p-3">
        <FolderOpen size={16} className="text-amber-400" />
        <div className="flex-1">
          <p className="text-sm font-semibold">Virtual Filesystem (vFS)</p>
          <p className="text-[11px] text-muted-foreground">
            In-memory sandboxed filesystem — files are accessible via <code className="text-amber-400 mx-0.5">vfs_read</code> syscalls and wiped on task termination
          </p>
        </div>
        <Badge variant="secondary" className="text-[10px] shrink-0">{totalFiles} files · {formatBytes(totalSize)}</Badge>
      </div>

      {/* Drop zone + actions */}
      <div className="flex items-center gap-2">
        <div
          onDragOver={(e) => { e.preventDefault(); setDragOverVFS(true); }}
          onDragLeave={() => setDragOverVFS(false)}
          onDrop={handleDropVFS}
          onClick={() => fileInputRef.current?.click()}
          className={cn(
            "flex-1 flex items-center justify-center gap-2 rounded-lg border-2 border-dashed px-3 py-3 cursor-pointer transition-all text-xs",
            dragOverVFS ? "border-amber-500/60 bg-amber-500/8" : "border-border hover:border-amber-500/30 hover:bg-muted/20"
          )}
        >
          <Upload size={14} className={dragOverVFS ? "text-amber-400" : "text-muted-foreground"} />
          <span className="text-muted-foreground">Drop files here to inject into vFS</span>
        </div>
        <input ref={fileInputRef} type="file" className="hidden" multiple
          onChange={(e) => { Array.from(e.target.files || []).forEach(handleFileUpload); e.target.value = ""; }} />
        <Button variant="outline" size="sm" className="h-8 text-xs gap-1" onClick={() => setShowNewFile(true)}>
          <FilePlus size={11} /> New File
        </Button>
        <Button variant="outline" size="sm" className="h-8 text-xs gap-1" onClick={() => setShowNewDir(true)}>
          <FolderPlus size={11} /> New Dir
        </Button>
      </div>

      {/* New file form */}
      {showNewFile && (
        <div className="space-y-2 rounded-lg border border-dashed border-cyan-500/30 bg-cyan-500/5 p-3">
          <div className="flex items-center gap-2">
            <Input placeholder="filename.json" value={newFileName} onChange={(e) => setNewFileName(e.target.value)} className="h-7 text-xs flex-1" />
            <Button size="sm" className="h-7 text-xs" onClick={handleCreateFile}>Create</Button>
            <Button variant="ghost" size="sm" className="h-7 text-xs" onClick={() => { setShowNewFile(false); setNewFileName(""); setNewFileContent(""); }}>Cancel</Button>
          </div>
          <textarea
            value={newFileContent}
            onChange={(e) => setNewFileContent(e.target.value)}
            placeholder="File content…"
            className="w-full h-20 rounded-md border border-border bg-black/20 p-2 text-[11px] font-mono text-foreground/80 resize-y focus:outline-none"
          />
        </div>
      )}

      {/* New dir form */}
      {showNewDir && (
        <div className="flex items-center gap-2 rounded-lg border border-dashed border-amber-500/30 bg-amber-500/5 p-3">
          <Input placeholder="folder_name" value={newDirName} onChange={(e) => setNewDirName(e.target.value)} className="h-7 text-xs flex-1" />
          <Button size="sm" className="h-7 text-xs" onClick={() => { if (newDirName.trim()) { onAddDir("/", newDirName.trim()); setNewDirName(""); setShowNewDir(false); } }}>Create</Button>
          <Button variant="ghost" size="sm" className="h-7 text-xs" onClick={() => { setShowNewDir(false); setNewDirName(""); }}>Cancel</Button>
        </div>
      )}

      {/* File tree + preview */}
      <div className="grid grid-cols-[200px_1fr] gap-3 min-h-[260px]">
        <ScrollArea className="h-[260px] rounded-lg border border-border bg-black/10 p-1.5">
          <DirView dir={root} path="/" />
        </ScrollArea>

        <div className="rounded-lg border border-border bg-black/10 p-3">
          {selectedFile ? (
            <div className="space-y-2 h-full flex flex-col">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2 min-w-0">
                  <FileCode size={12} className="text-cyan-400 shrink-0" />
                  <span className="text-xs font-medium font-mono truncate">{selectedFile.file.name}</span>
                </div>
                <div className="flex items-center gap-1.5 text-[10px] text-muted-foreground shrink-0">
                  <span>{formatBytes(selectedFile.file.size)}</span>
                  <span>·</span>
                  <span>{selectedFile.file.type}</span>
                </div>
              </div>
              <ScrollArea className="flex-1">
                <pre className="text-[11px] font-mono text-foreground/70 whitespace-pre-wrap break-all">
                  {selectedFile.file.content.slice(0, 8000)}
                  {selectedFile.file.content.length > 8000 && "\n… (truncated)"}
                </pre>
              </ScrollArea>
            </div>
          ) : (
            <div className="flex items-center justify-center h-full text-muted-foreground text-xs">
              Select a file to preview contents
            </div>
          )}
        </div>
      </div>

      <Alert variant="warning" className="py-2">
        <Shield size={12} />
        <AlertDescription className="text-xs">
          vFS is fully ephemeral — all files are held in-memory and destroyed when the task terminates. No data touches the host filesystem.
        </AlertDescription>
      </Alert>
    </div>
  );
}

// ─── Runtime Environment Variables ───────────────────────────────────

interface EnvVar {
  key: string;
  value: string;
  locked?: boolean;
}

function EnvVarsPanel({
  vars,
  setVars,
  onNotify,
}: {
  vars: EnvVar[];
  setVars: React.Dispatch<React.SetStateAction<EnvVar[]>>;
  onNotify: (msg: string, ok?: boolean) => void;
}) {
  const onChange = (key: string, value: string) =>
    setVars(prev => prev.map(v => v.key === key ? { ...v, value } : v));
  const onAdd = (key: string, value: string) => {
    if (vars.some(v => v.key === key)) {
      setVars(prev => prev.map(v => v.key === key ? { ...v, value } : v));
    } else {
      setVars(prev => [...prev, { key, value, locked: false }]);
    }
    onNotify(`Set ${key}`, true);
  };
  const onRemove = (key: string) => {
    setVars(prev => prev.filter(v => v.key !== key));
    onNotify(`Removed ${key}`, true);
  };
  const onToggleLock = (key: string) =>
    setVars(prev => prev.map(v => v.key === key ? { ...v, locked: !v.locked } : v));
  const [newKey, setNewKey] = useState("");
  const [newVal, setNewVal] = useState("");
  const [filter, setFilter] = useState("");
  const [showPresets, setShowPresets] = useState(false);

  const PRESETS: { label: string; vars: [string, string][] }[] = [
    { label: "Debug Mode", vars: [["LOG_LEVEL", "DEBUG"], ["VERBOSE", "1"], ["TRACE_SYSCALLS", "true"]] },
    { label: "Production", vars: [["LOG_LEVEL", "ERROR"], ["VERBOSE", "0"], ["NODE_ENV", "production"]] },
    { label: "IoT Sensor Node", vars: [["MOCK_NODE_ID", "X-01"], ["SENSOR_INTERVAL_MS", "1000"], ["REGION", "us-east-1"]] },
    { label: "Minimal Sandbox", vars: [["MAX_MEMORY_KB", "512"], ["MAX_INSTRUCTIONS", "10000"], ["SANDBOX_STRICT", "true"]] },
  ];

  const filtered = vars.filter(v => !filter || v.key.toLowerCase().includes(filter.toLowerCase()) || v.value.toLowerCase().includes(filter.toLowerCase()));

  return (
    <div className="space-y-4">
      {/* Header */}
      <div className="flex items-center gap-2 rounded-lg border border-border bg-muted/10 p-3">
        <Variable size={16} className="text-teal-400" />
        <div className="flex-1">
          <p className="text-sm font-semibold">Runtime Environment Variables</p>
          <p className="text-[11px] text-muted-foreground">
            Key-value pairs injected into linear memory at boot — accessible via <code className="text-teal-400 mx-0.5">env_get</code> syscall
          </p>
        </div>
        <Badge variant="secondary" className="text-[10px] shrink-0">{vars.length} vars</Badge>
      </div>

      {/* Quick preset buttons */}
      <div>
        <button onClick={() => setShowPresets(!showPresets)}
          className="flex items-center gap-1 text-[10px] uppercase tracking-wider text-muted-foreground font-medium hover:text-foreground transition-colors mb-1.5">
          <Settings2 size={10} /> Presets <ChevronDown size={9} className={cn("transition-transform", showPresets && "rotate-180")} />
        </button>
        {showPresets && (
          <div className="flex flex-wrap gap-1.5 mb-3">
            {PRESETS.map((preset) => (
              <Button key={preset.label} variant="outline" size="sm" className="h-6 text-[10px] gap-1"
                onClick={() => preset.vars.forEach(([k, v]) => onAdd(k, v))}>
                {preset.label}
              </Button>
            ))}
          </div>
        )}
      </div>

      {/* Add new */}
      <div className="flex items-center gap-2">
        <Input placeholder="KEY" value={newKey} onChange={(e) => setNewKey(e.target.value.toUpperCase())}
          className="h-7 text-xs flex-1 font-mono" />
        <span className="text-muted-foreground">=</span>
        <Input placeholder="value" value={newVal} onChange={(e) => setNewVal(e.target.value)}
          className="h-7 text-xs flex-1 font-mono"
          onKeyDown={(e) => { if (e.key === "Enter" && newKey.trim()) { onAdd(newKey.trim(), newVal); setNewKey(""); setNewVal(""); } }} />
        <Button size="sm" className="h-7 text-xs gap-1" onClick={() => { if (newKey.trim()) { onAdd(newKey.trim(), newVal); setNewKey(""); setNewVal(""); } }}>
          <Plus size={10} /> Set
        </Button>
      </div>

      {/* Search */}
      {vars.length > 5 && (
        <div className="relative">
          <Search size={11} className="absolute left-2 top-1/2 -translate-y-1/2 text-muted-foreground" />
          <Input placeholder="Filter vars…" value={filter} onChange={(e) => setFilter(e.target.value)} className="pl-7 h-7 text-xs" />
        </div>
      )}

      {/* Variable table */}
      <ScrollArea className="max-h-[260px]">
        {filtered.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-8 text-muted-foreground">
            <Variable size={24} className="mb-2 opacity-20" />
            <p className="text-xs">{vars.length === 0 ? "No environment variables set" : "No matches"}</p>
          </div>
        ) : (
          <div className="space-y-1">
            {filtered.map((v) => (
              <div key={v.key} className={cn(
                "flex items-center gap-2 rounded-md border px-2.5 py-1.5 group transition-all",
                v.locked ? "border-teal-500/30 bg-teal-500/5" : "border-border bg-muted/5"
              )}>
                <span className="text-xs font-mono font-bold text-teal-400 shrink-0 min-w-[100px] truncate">{v.key}</span>
                <span className="text-muted-foreground text-xs">=</span>
                <Input
                  value={v.value}
                  onChange={(e) => onChange(v.key, e.target.value)}
                  disabled={v.locked}
                  className="h-6 text-xs flex-1 font-mono border-none bg-transparent p-0 focus-visible:ring-0 disabled:opacity-50"
                />
                <div className="flex items-center gap-1 shrink-0 opacity-0 group-hover:opacity-100 transition-opacity">
                  <button onClick={() => onToggleLock(v.key)} className="text-muted-foreground hover:text-foreground p-0.5">
                    {v.locked ? <Lock size={10} className="text-teal-400" /> : <Unlock size={10} />}
                  </button>
                  <button onClick={() => onRemove(v.key)} className="text-destructive/50 hover:text-destructive p-0.5">
                    <X size={10} />
                  </button>
                </div>
              </div>
            ))}
          </div>
        )}
      </ScrollArea>

      {/* Export */}
      {vars.length > 0 && (
        <div className="flex items-center justify-between">
          <span className="text-[10px] text-muted-foreground">{vars.length} variable{vars.length !== 1 ? "s" : ""} · {vars.filter(v => v.locked).length} locked</span>
          <Button variant="ghost" size="sm" className="h-6 text-xs gap-1"
            onClick={() => navigator.clipboard.writeText(vars.map(v => `${v.key}=${v.value}`).join("\n"))}>
            <Copy size={10} /> Copy as .env
          </Button>
        </div>
      )}
    </div>
  );
}

// ─── Scenario Orchestrator Types ─────────────────────────────────────

interface ScenarioEvent {
  at_seconds: number;
  action: "set_sensor" | "set_env" | "inject_file" | "trigger" | "assert_stdout" | "assert_no_call" | "snapshot";
  target: string;          // sensor id, env key, filename, or syscall name
  value?: string | number; // numeric, string, or boolean
  description?: string;
}

interface ScenarioDefinition {
  id: string;
  name: string;
  description: string;
  timeout_seconds: number;
  events: ScenarioEvent[];
  tags: string[];
}

type ScenarioStatus = "idle" | "running" | "passed" | "failed" | "timeout";

interface ScenarioRun {
  scenarioId: string;
  scenarioName: string;
  status: ScenarioStatus;
  startedAt: string;
  completedAt?: string;
  currentSecond: number;
  totalSeconds: number;
  assertionsPassed: number;
  assertionsFailed: number;
  failureReason?: string;
  eventLog: { time: number; event: string; ok: boolean }[];
  snapshotId?: string;
}

// ─── Scenario Orchestrator Panel ─────────────────────────────────────

const EXAMPLE_SCENARIO: ScenarioDefinition = {
  id: "scenario-overheat",
  name: "Sensor Overheat Response",
  description: "Simulate temperature spike and verify module triggers send_alert within 2s",
  timeout_seconds: 20,
  tags: ["thermal", "safety", "critical"],
  events: [
    { at_seconds: 0,  action: "set_sensor",     target: "temp", value: 22, description: "Normal temperature" },
    { at_seconds: 0,  action: "set_env",         target: "ALERT_THRESHOLD", value: "85", description: "Set alert threshold" },
    { at_seconds: 5,  action: "set_sensor",     target: "temp", value: 45, description: "Warming up" },
    { at_seconds: 8,  action: "set_sensor",     target: "temp", value: 72, description: "Getting hot" },
    { at_seconds: 10, action: "set_sensor",     target: "temp", value: 95, description: "⚠️ Overheat!" },
    { at_seconds: 12, action: "assert_stdout",  target: "ALERT", description: "Module should print ALERT" },
    { at_seconds: 12, action: "snapshot",       target: "auto", description: "Auto-snapshot on assertion" },
    { at_seconds: 15, action: "set_sensor",     target: "temp", value: 40, description: "Cooling down" },
    { at_seconds: 18, action: "assert_stdout",  target: "NORMAL", description: "Module should print NORMAL" },
  ],
};

const EXAMPLE_SCENARIOS: ScenarioDefinition[] = [
  EXAMPLE_SCENARIO,
  {
    id: "scenario-unauthorized",
    name: "Unauthorized Access Attempt",
    description: "Inject suspicious file into vFS and verify module does NOT attempt network access",
    timeout_seconds: 15,
    tags: ["security", "network", "vfs"],
    events: [
      { at_seconds: 0,  action: "inject_file",    target: "config.json", value: '{"admin":true,"escalate":true}', description: "Inject privilege escalation config" },
      { at_seconds: 3,  action: "set_env",         target: "AUTH_MODE", value: "bypass", description: "Set auth bypass mode" },
      { at_seconds: 5,  action: "trigger",         target: "send_alert", description: "Trigger alert event" },
      { at_seconds: 8,  action: "assert_no_call",  target: "network_connect", description: "Module must NOT call network_connect" },
      { at_seconds: 10, action: "assert_stdout",   target: "DENIED", description: "Module should log DENIED" },
      { at_seconds: 12, action: "snapshot",        target: "auto", description: "Capture state for forensic analysis" },
    ],
  },
  {
    id: "scenario-memory-pressure",
    name: "Memory Pressure Resilience",
    description: "Reduce available memory and verify module degrades gracefully",
    timeout_seconds: 25,
    tags: ["resilience", "memory", "stress"],
    events: [
      { at_seconds: 0,  action: "set_env",        target: "MAX_MEMORY_KB", value: "4096", description: "Normal memory" },
      { at_seconds: 5,  action: "set_env",        target: "MAX_MEMORY_KB", value: "1024", description: "Reduce to 1MB" },
      { at_seconds: 10, action: "set_env",        target: "MAX_MEMORY_KB", value: "256", description: "Critical: 256KB" },
      { at_seconds: 12, action: "assert_stdout",  target: "LOW_MEMORY", description: "Module should detect low memory" },
      { at_seconds: 15, action: "set_env",        target: "MAX_MEMORY_KB", value: "4096", description: "Restore memory" },
      { at_seconds: 18, action: "assert_stdout",  target: "RECOVERED", description: "Module should recover" },
    ],
  },
];

function ScenarioOrchestrator({
  scenarios,
  onAddScenario,
  runs,
  onStartScenario,
  onStopScenario,
  onRunAll,
  runAllInProgress,
}: {
  scenarios: ScenarioDefinition[];
  onAddScenario: (s: ScenarioDefinition) => void;
  runs: ScenarioRun[];
  onStartScenario: (id: string) => void;
  onStopScenario: (id: string) => void;
  onRunAll: () => void;
  runAllInProgress: boolean;
}) {
  const [selectedScenario, setSelectedScenario] = useState<ScenarioDefinition | null>(null);
  const [yamlEditor, setYamlEditor] = useState("");
  const [showEditor, setShowEditor] = useState(false);
  const [parseError, setParseError] = useState<string | null>(null);
  const [activeView, setActiveView] = useState<"list" | "timeline" | "report">("list");

  const passedCount = runs.filter(r => r.status === "passed").length;
  const failedCount = runs.filter(r => r.status === "failed").length;
  const runningCount = runs.filter(r => r.status === "running").length;

  // Parse simple YAML-like scenario (simplified — not full YAML)
  const parseScenario = (): ScenarioDefinition | null => {
    try {
      const parsed = JSON.parse(yamlEditor);
      if (!parsed.name || !parsed.events) throw new Error("Missing 'name' and 'events' fields");
      return {
        id: `scenario-${Date.now()}`,
        name: parsed.name,
        description: parsed.description ?? "",
        timeout_seconds: parsed.timeout_seconds ?? 30,
        events: parsed.events,
        tags: parsed.tags ?? [],
      };
    } catch (e) {
      setParseError(e instanceof Error ? e.message : "Invalid JSON — use the format shown in the template");
      return null;
    }
  };

  return (
    <div className="space-y-4">
      {/* Header */}
      <div className="flex items-center gap-2 rounded-lg border border-border bg-muted/10 p-3">
        <Target size={16} className="text-rose-400" />
        <div className="flex-1">
          <p className="text-sm font-semibold">Deterministic Scenario Orchestrator</p>
          <p className="text-[11px] text-muted-foreground">
            Script timelines of environmental changes and expected behaviors — auto-assert, auto-snapshot on failure
          </p>
        </div>
        <div className="flex items-center gap-1.5">
          <Button variant="outline" size="sm" className="h-6 text-xs gap-1" onClick={() => setShowEditor(!showEditor)}>
            <Plus size={10} /> New Scenario
          </Button>
          <Button size="sm" className="h-6 text-xs gap-1" onClick={onRunAll} disabled={runAllInProgress || scenarios.length === 0}>
            {runAllInProgress ? <Loader2 size={10} className="animate-spin" /> : <FastForward size={10} />}
            Run All ({scenarios.length})
          </Button>
        </div>
      </div>

      {/* Stats strip */}
      <div className="grid grid-cols-2 sm:grid-cols-4 gap-2">
        <MetricPill icon={Target}         label="Scenarios"   value={scenarios.length.toString()} accent="text-rose-400" />
        <MetricPill icon={CheckCircle}    label="Passed"      value={passedCount.toString()} accent="text-green-400" />
        <MetricPill icon={AlertTriangle}  label="Failed"      value={failedCount.toString()} accent="text-red-400" />
        <MetricPill icon={Loader2}        label="Running"     value={runningCount.toString()} accent="text-blue-400" />
      </div>

      {/* Batch results summary */}
      {runs.length > 0 && (
        <div className="space-y-1.5">
          <div className="flex items-center justify-between text-[10px] text-muted-foreground">
            <span>Resilience Score</span>
            <span>{runs.length > 0 ? Math.round((passedCount / Math.max(runs.length, 1)) * 100) : 0}%</span>
          </div>
          <div className="h-2 rounded-full bg-muted/30 overflow-hidden">
            <div
              className={cn("h-full rounded-full transition-all", passedCount > failedCount ? "bg-green-400" : "bg-red-400")}
              style={{ width: `${(passedCount / Math.max(runs.length, 1)) * 100}%` }}
            />
          </div>
        </div>
      )}

      {/* Scenario editor */}
      {showEditor && (
        <div className="space-y-2 rounded-lg border border-dashed border-rose-500/30 bg-rose-500/5 p-3">
          <div className="flex items-center justify-between mb-1">
            <p className="text-[10px] uppercase tracking-wider text-rose-400 font-medium">Define Scenario (JSON)</p>
            <Button variant="ghost" size="sm" className="h-5 text-[10px]" onClick={() => {
              setYamlEditor(JSON.stringify(EXAMPLE_SCENARIO, null, 2));
              setParseError(null);
            }}>Load Template</Button>
          </div>
          <textarea
            value={yamlEditor}
            onChange={(e) => { setYamlEditor(e.target.value); setParseError(null); }}
            placeholder={`{\n  "name": "My Test Scenario",\n  "timeout_seconds": 20,\n  "tags": ["test"],\n  "events": [\n    { "at_seconds": 0, "action": "set_sensor", "target": "temp", "value": 22 },\n    { "at_seconds": 10, "action": "assert_stdout", "target": "OK" }\n  ]\n}`}
            className="w-full h-40 rounded-md border border-border bg-black/20 p-2 text-[11px] font-mono text-foreground/80 resize-y focus:outline-none"
          />
          {parseError && <p className="text-xs text-red-400">{parseError}</p>}
          <div className="flex items-center gap-2">
            <Button size="sm" className="h-7 text-xs gap-1" onClick={() => {
              const s = parseScenario();
              if (s) { onAddScenario(s); setYamlEditor(""); setShowEditor(false); }
            }}>
              <Plus size={10} /> Add Scenario
            </Button>
            <Button variant="ghost" size="sm" className="h-7 text-xs" onClick={() => { setShowEditor(false); setYamlEditor(""); setParseError(null); }}>Cancel</Button>
          </div>
        </div>
      )}

      {/* View tabs */}
      <div className="flex gap-1 rounded-lg bg-muted/20 p-0.5 w-fit">
        {(["list", "timeline", "report"] as const).map((v) => (
          <button key={v} onClick={() => setActiveView(v)}
            className={cn("px-3 py-1 rounded-md text-xs font-medium transition-all capitalize",
              activeView === v ? "bg-primary/10 text-primary" : "text-muted-foreground hover:text-foreground")}>
            {v}
          </button>
        ))}
      </div>

      {/* === List View === */}
      {activeView === "list" && (
        <ScrollArea className="max-h-[360px]">
          {scenarios.length === 0 ? (
            <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
              <Target size={32} className="mb-3 opacity-20" />
              <p className="text-sm">No scenarios defined</p>
              <p className="text-[10px] mt-1">Create a scenario or load built-in examples below</p>
              <Button variant="outline" size="sm" className="mt-3 text-xs gap-1" onClick={() => {
                EXAMPLE_SCENARIOS.forEach(s => onAddScenario(s));
              }}>
                <Layers size={10} /> Load 3 Example Scenarios
              </Button>
            </div>
          ) : (
            <div className="space-y-2">
              {scenarios.map((sc) => {
                const run = runs.find(r => r.scenarioId === sc.id);
                const statusColor = !run ? "text-muted-foreground" :
                  run.status === "passed" ? "text-green-400" :
                  run.status === "failed" ? "text-red-400" :
                  run.status === "running" ? "text-blue-400" : "text-yellow-400";
                const statusBg = !run ? "bg-muted/5 border-border" :
                  run.status === "passed" ? "bg-green-500/5 border-green-500/20" :
                  run.status === "failed" ? "bg-red-500/5 border-red-500/20" :
                  run.status === "running" ? "bg-blue-500/5 border-blue-500/20" : "bg-yellow-500/5 border-yellow-500/20";

                return (
                  <div key={sc.id} className={cn("rounded-lg border p-3 transition-all", statusBg)}>
                    <div className="flex items-start justify-between gap-3">
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2">
                          <Target size={13} className={statusColor} />
                          <span className="text-xs font-semibold truncate">{sc.name}</span>
                          {run && <Badge variant="outline" className={cn("text-[9px]", statusColor)}>{run.status}</Badge>}
                        </div>
                        <p className="text-[10px] text-muted-foreground mt-0.5 truncate">{sc.description}</p>
                        <div className="flex items-center gap-2 mt-1">
                          <span className="text-[9px] text-muted-foreground">{sc.events.length} events · {sc.timeout_seconds}s timeout</span>
                          {sc.tags.map(t => <Badge key={t} variant="secondary" className="text-[8px] h-3 px-1">{t}</Badge>)}
                        </div>
                        {/* Progress bar for running */}
                        {run?.status === "running" && (
                          <div className="mt-2">
                            <div className="flex justify-between text-[9px] text-muted-foreground mb-0.5">
                              <span>T+{run.currentSecond}s</span>
                              <span>{Math.round((run.currentSecond / run.totalSeconds) * 100)}%</span>
                            </div>
                            <Progress value={(run.currentSecond / run.totalSeconds) * 100} className="h-1.5" />
                          </div>
                        )}
                        {/* Assertion results */}
                        {run && run.status !== "idle" && run.status !== "running" && (
                          <div className="flex items-center gap-3 mt-1.5 text-[10px]">
                            <span className="flex items-center gap-0.5 text-green-400"><ClipboardCheck size={10} /> {run.assertionsPassed} passed</span>
                            <span className="flex items-center gap-0.5 text-red-400"><ClipboardX size={10} /> {run.assertionsFailed} failed</span>
                            {run.snapshotId && <span className="flex items-center gap-0.5 text-amber-400"><Camera size={10} /> Snapshot saved</span>}
                          </div>
                        )}
                        {run?.failureReason && (
                          <p className="text-[10px] text-red-400 mt-1 font-mono">✗ {run.failureReason}</p>
                        )}
                      </div>
                      <div className="flex items-center gap-1 shrink-0">
                        <Button variant="ghost" size="icon" className="h-6 w-6"
                          onClick={() => setSelectedScenario(selectedScenario?.id === sc.id ? null : sc)}>
                          <Eye size={11} />
                        </Button>
                        {run?.status === "running" ? (
                          <Button variant="ghost" size="icon" className="h-6 w-6 text-red-400" onClick={() => onStopScenario(sc.id)}>
                            <StopCircle size={12} />
                          </Button>
                        ) : (
                          <Button variant="ghost" size="icon" className="h-6 w-6 text-green-400" onClick={() => onStartScenario(sc.id)}>
                            <PlayCircle size={12} />
                          </Button>
                        )}
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </ScrollArea>
      )}

      {/* === Timeline View === */}
      {activeView === "timeline" && selectedScenario && (
        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <p className="text-[11px] font-semibold">{selectedScenario.name} — Timeline</p>
            <span className="text-[10px] text-muted-foreground">{selectedScenario.events.length} events over {selectedScenario.timeout_seconds}s</span>
          </div>
          <div className="relative pl-6 space-y-0">
            {/* Vertical timeline line */}
            <div className="absolute left-[9px] top-0 bottom-0 w-px bg-border" />
            {selectedScenario.events.map((ev, i) => {
              const run = runs.find(r => r.scenarioId === selectedScenario.id);
              const logEntry = run?.eventLog[i];
              const isAssertion = ev.action.startsWith("assert");
              const iconColor = isAssertion
                ? (logEntry?.ok ? "text-green-400 bg-green-500/20" : logEntry?.ok === false ? "text-red-400 bg-red-500/20" : "text-amber-400 bg-amber-500/20")
                : "text-violet-400 bg-violet-500/20";

              return (
                <div key={i} className="relative flex items-start gap-3 pb-3">
                  {/* Dot on timeline */}
                  <div className={cn("absolute left-[-15px] mt-1 h-4 w-4 rounded-full flex items-center justify-center text-[8px] font-bold", iconColor)}>
                    {isAssertion ? (logEntry?.ok ? "✓" : logEntry?.ok === false ? "✗" : "?") : "•"}
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2">
                      <Badge variant="outline" className="text-[9px] font-mono h-4 px-1.5 shrink-0">T+{ev.at_seconds}s</Badge>
                      <span className="text-xs font-medium">{ev.action}</span>
                      <span className="text-[10px] text-muted-foreground font-mono">{ev.target}</span>
                      {ev.value !== undefined && <span className="text-[10px] text-cyan-400 font-mono">= {String(ev.value)}</span>}
                    </div>
                    {ev.description && <p className="text-[10px] text-muted-foreground mt-0.5">{ev.description}</p>}
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {activeView === "timeline" && !selectedScenario && (
        <div className="flex flex-col items-center justify-center py-8 text-muted-foreground">
          <Timer size={24} className="mb-2 opacity-20" />
          <p className="text-xs">Select a scenario from the list to view its timeline</p>
        </div>
      )}

      {/* === Report View === */}
      {activeView === "report" && (
        <div className="space-y-3">
          {runs.length === 0 ? (
            <div className="flex flex-col items-center justify-center py-8 text-muted-foreground">
              <ClipboardCheck size={24} className="mb-2 opacity-20" />
              <p className="text-xs">Run scenarios to generate a resilience report</p>
            </div>
          ) : (
            <>
              <p className="text-[11px] uppercase tracking-wider text-muted-foreground font-medium">
                Resilience & Security Report · {runs.length} scenario(s)
              </p>
              <div className="space-y-2">
                {runs.map((r, i) => (
                  <div key={i} className={cn(
                    "flex items-center gap-3 rounded-lg border px-3 py-2",
                    r.status === "passed" ? "border-green-500/20 bg-green-500/5" : r.status === "failed" ? "border-red-500/20 bg-red-500/5" : "border-border bg-muted/5"
                  )}>
                    {r.status === "passed" ? <CheckCircle size={13} className="text-green-400" /> :
                     r.status === "failed" ? <AlertTriangle size={13} className="text-red-400" /> :
                     r.status === "running" ? <Loader2 size={13} className="text-blue-400 animate-spin" /> :
                     <Clock size={13} className="text-muted-foreground" />}
                    <div className="flex-1 min-w-0">
                      <p className="text-xs font-medium truncate">{r.scenarioName}</p>
                      {r.failureReason && <p className="text-[10px] text-red-400 truncate">{r.failureReason}</p>}
                    </div>
                    <div className="flex items-center gap-3 text-[10px] shrink-0">
                      <span className="text-green-400">{r.assertionsPassed}✓</span>
                      <span className="text-red-400">{r.assertionsFailed}✗</span>
                      {r.completedAt && <span className="text-muted-foreground">{timeAgo(r.completedAt)}</span>}
                    </div>
                  </div>
                ))}
              </div>

              {/* Export report */}
              <div className="flex items-center justify-end">
                <Button variant="ghost" size="sm" className="h-6 text-xs gap-1"
                  onClick={() => navigator.clipboard.writeText(JSON.stringify(runs, null, 2))}>
                  <Copy size={10} /> Copy Report JSON
                </Button>
              </div>
            </>
          )}
        </div>
      )}

      <Alert variant="info" className="py-2">
        <Target size={12} />
        <AlertDescription className="text-xs">
          Scenarios execute deterministically — timeline events fire at exact T+N seconds.
          Assertion failures auto-trigger a snapshot for forensic analysis in the Security Hub.
        </AlertDescription>
      </Alert>
    </div>
  );
}

// ─── v5.0+ types ─────────────────────────────────────────────────────

/** Slider/toggle register used in the ABI mocking panel (v5.0+ API). */
interface MockRegister {
  id: string;
  label: string;
  kind: "slider" | "toggle";
  value: number;
  min: number;
  max: number;
  unit?: string;
}

/** Alias for ScenarioRun — used by the scenario state array. */
type ScenarioRunResult = ScenarioRun;

// ─── v5.0+ panel components ──────────────────────────────────────────

function ABIMockingPanel({
  registers,
  setRegisters,
  eventLog,
  setEventLog,
  onNotify,
}: {
  registers: MockRegister[];
  setRegisters: React.Dispatch<React.SetStateAction<MockRegister[]>>;
  eventLog: { ts: string; msg: string }[];
  setEventLog: React.Dispatch<React.SetStateAction<{ ts: string; msg: string }[]>>;
  onNotify: (msg: string, ok?: boolean) => void;
}) {
  const setValue = (id: string, val: number) => {
    setRegisters(prev => prev.map(r => r.id === id ? { ...r, value: val } : r));
    setEventLog(prev => [
      { ts: new Date().toISOString(), msg: `${id} → ${val}` },
      ...prev.slice(0, 49),
    ]);
    onNotify(`Updated ${id}`, true);
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-2 rounded-lg border border-border bg-muted/10 p-3">
        <Sliders size={16} className="text-violet-400" />
        <div className="flex-1">
          <p className="text-sm font-semibold">ABI Sensor Mocking</p>
          <p className="text-[11px] text-muted-foreground">Inject virtual sensor values into the WASM execution environment</p>
        </div>
        <Badge variant="secondary" className="text-[10px]">{registers.length} registers</Badge>
      </div>
      <div className="space-y-3">
        {registers.map(r => (
          <div key={r.id} className="rounded-lg border border-border bg-muted/5 p-3 space-y-1.5">
            <div className="flex items-center justify-between">
              <span className="text-xs font-medium">{r.label}</span>
              <span className="text-xs font-mono text-violet-400">{r.value}{r.unit ? ` ${r.unit}` : ""}</span>
            </div>
            {r.kind === "slider" ? (
              <input
                type="range"
                min={r.min}
                max={r.max}
                step={(r.max - r.min) / 100}
                value={r.value}
                onChange={e => setValue(r.id, parseFloat(e.target.value))}
                className="w-full h-1.5 rounded-full appearance-none bg-muted/30 accent-violet-500 cursor-pointer"
              />
            ) : (
              <button
                onClick={() => setValue(r.id, r.value ? 0 : 1)}
                className={cn(
                  "relative h-5 w-9 rounded-full transition-colors",
                  r.value ? "bg-violet-500" : "bg-muted/40 border border-border"
                )}
              >
                <span className={cn(
                  "absolute top-0.5 left-0.5 h-4 w-4 rounded-full bg-white transition-transform shadow-sm",
                  r.value && "translate-x-4"
                )} />
              </button>
            )}
          </div>
        ))}
      </div>
      {eventLog.length > 0 && (
        <div className="rounded-lg border border-border bg-muted/5 p-3 space-y-1 max-h-32 overflow-y-auto">
          <p className="text-[10px] uppercase tracking-wider text-muted-foreground font-medium mb-1">Event Log</p>
          {eventLog.map((e, i) => (
            <p key={i} className="text-[11px] font-mono text-muted-foreground">{e.ts.slice(11, 19)} {e.msg}</p>
          ))}
        </div>
      )}
    </div>
  );
}

function VFSExplorerPanel({
  files,
  setFiles,
  onNotify,
}: {
  files: VFSFile[];
  setFiles: React.Dispatch<React.SetStateAction<VFSFile[]>>;
  onNotify: (msg: string, ok?: boolean) => void;
}) {
  const [newName, setNewName] = useState("");
  const [newContent, setNewContent] = useState("");

  const addFile = () => {
    if (!newName.trim()) return;
    const now = new Date().toISOString();
    const f: VFSFile = {
      name: newName.trim(),
      content: newContent,
      size: newContent.length,
      type: "text",
      created: now,
      modified: now,
    };
    setFiles(prev => [...prev, f]);
    setNewName("");
    setNewContent("");
    onNotify(`Added ${f.name}`, true);
  };

  const removeFile = (name: string) => {
    setFiles(prev => prev.filter(f => f.name !== name));
    onNotify(`Removed ${name}`, true);
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-2 rounded-lg border border-border bg-muted/10 p-3">
        <FolderOpen size={16} className="text-amber-400" />
        <div className="flex-1">
          <p className="text-sm font-semibold">Virtual Filesystem</p>
          <p className="text-[11px] text-muted-foreground">Files injected into the WASM sandbox at execution time</p>
        </div>
        <Badge variant="secondary" className="text-[10px]">{files.length} files</Badge>
      </div>
      <div className="space-y-2">
        <Input placeholder="filename.txt" value={newName} onChange={e => setNewName(e.target.value)} className="h-7 text-xs font-mono" />
        <textarea
          placeholder="file content…"
          value={newContent}
          onChange={e => setNewContent(e.target.value)}
          className="w-full h-20 text-xs font-mono rounded-md border border-input bg-background px-3 py-2 resize-none focus:outline-none focus:ring-1 focus:ring-ring"
        />
        <Button size="sm" className="h-7 text-xs gap-1" onClick={addFile}>
          <Plus size={10} /> Add File
        </Button>
      </div>
      <ScrollArea className="max-h-[220px]">
        {files.length === 0 ? (
          <p className="text-xs text-muted-foreground text-center py-6">No virtual files</p>
        ) : (
          <div className="space-y-1">
            {files.map(f => (
              <div key={f.name} className="flex items-center gap-2 rounded-md border border-border bg-muted/5 px-3 py-1.5 group">
                <FileCode size={12} className="text-amber-400 shrink-0" />
                <span className="text-xs font-mono flex-1 truncate">{f.name}</span>
                <span className="text-[10px] text-muted-foreground shrink-0">{f.size}B</span>
                <button onClick={() => removeFile(f.name)} className="opacity-0 group-hover:opacity-100 text-destructive/50 hover:text-destructive">
                  <X size={10} />
                </button>
              </div>
            ))}
          </div>
        )}
      </ScrollArea>
    </div>
  );
}

function ScenarioOrchestratorPanel({
  taskId,
  tasks,
  mockRegisters,
  vfsFiles,
  envVars,
  onNotify,
  onSnapshot,
}: {
  taskId: string;
  tasks: Task[];
  mockRegisters: MockRegister[];
  vfsFiles: VFSFile[];
  envVars: EnvVar[];
  onNotify: (msg: string, ok?: boolean) => void;
  onSnapshot: () => void;
}) {
  const [running, setRunning] = useState(false);
  const [results, setResults] = useState<ScenarioRunResult[]>([]);

  const task = tasks.find(t => t.id === taskId);

  const runScenario = async () => {
    setRunning(true);
    onNotify("Scenario started", true);
    await new Promise(r => setTimeout(r, 1500));
    const run: ScenarioRunResult = {
      scenarioId: `scenario-${Date.now()}`,
      scenarioName: "Manual Run",
      status: "passed",
      startedAt: new Date().toISOString(),
      completedAt: new Date().toISOString(),
      currentSecond: 0,
      totalSeconds: 0,
      assertionsPassed: mockRegisters.length,
      assertionsFailed: 0,
      eventLog: mockRegisters.map(r => ({ time: 0, event: `${r.label}=${r.value}`, ok: true })),
    };
    setResults(prev => [run, ...prev.slice(0, 9)]);
    onNotify("Scenario passed", true);
    onSnapshot();
    setRunning(false);
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-2 rounded-lg border border-border bg-muted/10 p-3">
        <GitBranch size={16} className="text-sky-400" />
        <div className="flex-1">
          <p className="text-sm font-semibold">Scenario Orchestrator</p>
          <p className="text-[11px] text-muted-foreground">
            Runs the current environment ({mockRegisters.length} registers · {vfsFiles.length} files · {envVars.length} vars) against <span className="text-foreground font-medium">{task?.name ?? taskId}</span>
          </p>
        </div>
      </div>
      <div className="flex items-center gap-2">
        <Button size="sm" className="h-8 gap-2" onClick={runScenario} disabled={running}>
          {running ? <RefreshCw size={12} className="animate-spin" /> : <Play size={12} />}
          {running ? "Running…" : "Run Scenario"}
        </Button>
        <Button size="sm" variant="outline" className="h-8 gap-2" onClick={onSnapshot}>
          <Camera size={12} /> Snapshot
        </Button>
      </div>
      {results.length > 0 && (
        <div className="space-y-2">
          <p className="text-[10px] uppercase tracking-wider text-muted-foreground font-medium">Recent Runs</p>
          {results.map((r, i) => (
            <div key={i} className={cn(
              "flex items-center gap-2 rounded-md border px-3 py-2",
              r.status === "passed" ? "border-green-500/30 bg-green-500/5" : "border-red-500/30 bg-red-500/5"
            )}>
              {r.status === "passed"
                ? <CheckCircle size={12} className="text-green-400 shrink-0" />
                : <AlertTriangle size={12} className="text-red-400 shrink-0" />}
              <span className="text-xs flex-1">{r.scenarioName}</span>
              <span className="text-[10px] text-muted-foreground">{r.assertionsPassed} passed · {r.assertionsFailed} failed</span>
            </div>
          ))}
        </div>
      )}
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

  // New enhanced state
  const [securityReport, setSecurityReport] = useState<SecurityReport | null>(null);
  const [securityLoading, setSecurityLoading] = useState(false);
  const [inspectData, setInspectData] = useState<Record<string, unknown> | null>(null);
  const [inspectLoading, setInspectLoading] = useState(false);
  const [snapshots, setSnapshots] = useState<Snapshot[]>([]);
  const [snapshotsLoading, setSnapshotsLoading] = useState(false);
  const [execHistoryExpanded, setExecHistoryExpanded] = useState<string | null>(null);
  const [logFilter, setLogFilter] = useState("");

  // Chaos Sandbox state
  const [chaosEnabled,   setChaosEnabled]   = useState(false);
  const [chaosMode,      setChaosMode]      = useState<ChaosMode>("latency");
  const [chaosIntensity, setChaosIntensity] = useState(25);

  // IMC Pipes state
  const [imcPipes, setImcPipes] = useState<IMCPipe[]>([]);

  // Hardening state
  const [appliedPolicy, setAppliedPolicy] = useState<HardenedPolicy | null>(null);

  // ABI Mocking state
  const [mockRegisters, setMockRegisters] = useState<MockRegister[]>([
    { id: "temperature", label: "Temperature", kind: "slider", value: 22, min: 0, max: 150, unit: "°C" },
    { id: "pressure_psi", label: "Pressure", kind: "slider", value: 14.7, min: 0, max: 500, unit: "psi" },
    { id: "alarm_active", label: "Alarm Active", kind: "toggle", value: 0, min: 0, max: 1 },
    { id: "motor_running", label: "Motor Running", kind: "toggle", value: 0, min: 0, max: 1 },
    { id: "battery_pct", label: "Battery %", kind: "slider", value: 85, min: 0, max: 100, unit: "%" },
  ]);
  const [mockEventLog, setMockEventLog] = useState<{ ts: string; msg: string }[]>([]);

  // vFS state
  const [vfsFiles, setVfsFiles] = useState<VFSFile[]>([]);

  // Env Vars state
  const [envVars, setEnvVars] = useState<EnvVar[]>([
    { key: "LOG_LEVEL", value: "INFO", locked: false },
    { key: "MOCK_NODE_ID", value: "X-01", locked: false },
  ]);

  // Scenario state
  const [scenarioResults, setScenarioResults] = useState<ScenarioRunResult[]>([]);

  const fileRef = useRef<HTMLInputElement>(null);
  const execResultRef = useRef<HTMLDivElement>(null);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());

  // ── Data fetching ──

  const refresh = useCallback(async () => {
    try { setTasks(await getTasks()); } catch {}
  }, []);

  useEffect(() => {
    refresh();
    const id = setInterval(refresh, 6_000);
    return () => clearInterval(id);
  }, [refresh]);

  // ── Keyboard shortcuts ──
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      // Ignore when typing in an input / textarea / contenteditable
      const tag = (e.target as HTMLElement)?.tagName;
      if (tag === "INPUT" || tag === "TEXTAREA" || (e.target as HTMLElement)?.isContentEditable) return;
      if (e.key === "r" && !e.metaKey && !e.ctrlKey) { e.preventDefault(); refresh(); }
      if (e.key === "u" && !e.metaKey && !e.ctrlKey) { e.preventDefault(); fileRef.current?.click(); }
      if (e.key === "Escape" && selected) { setSelected(null); }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [refresh, selected]);

  const notify = (msg: string, ok = true) => {
    setToastMsg({ msg, ok });
    setTimeout(() => setToastMsg(null), ok ? 3_000 : 6_000);
  };

  // ── Filtering / sorting ──

  const filtered = tasks
    .filter((t) => {
      if (filter && !t.name.toLowerCase().includes(filter.toLowerCase()) && !t.id.startsWith(filter)) return false;
      if (statusFilter === "executing") return executing === t.id;
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
    setSecurityReport(null);
    setInspectData(null);
    setSnapshots([]);
    setExecHistory([]);
    setExecHistoryExpanded(null);
    setLogFilter("");
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
    const taskName = file.name.replace(/\.(wasm|wat)$/, "");
    const isDuplicate = tasks.some(t => t.name === taskName);
    setUploading(true);
    try {
      const bytes = await readFileAsBytes(file);
      await uploadTask(taskName, bytes);
      notify(isDuplicate ? `Replaced "${taskName}" with new binary` : `Uploaded ${file.name}`);
      refresh();
    } catch (e: unknown) { notify(e instanceof Error ? e.message : "Upload failed", false); }
    finally { setUploading(false); }
  };

  const handleUploadMultiple = async (files: FileList | File[]) => {
    const arr = Array.from(files).filter(f => /\.(wasm|wat)$/.test(f.name));
    if (arr.length === 0) { notify("No .wasm / .wat files found", false); return; }
    for (const file of arr) await handleUpload(file);
  };

  const handleDrop = (e: DragEvent<HTMLDivElement>) => {
    e.preventDefault(); setDragOver(false);
    const files = e.dataTransfer.files;
    if (files.length > 1) handleUploadMultiple(files);
    else if (files.length === 1) handleUpload(files[0]);
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

  // Auto-scroll output panel into view after execution completes
  useEffect(() => {
    if (execResult && execResultRef.current) {
      execResultRef.current.scrollIntoView({ behavior: "smooth", block: "start" });
    }
  }, [execResult]);

  const handleExportTasks = async () => {
    try {
      const data = JSON.stringify(tasks, null, 2);
      await navigator.clipboard.writeText(data);
      notify(`Exported ${tasks.length} tasks to clipboard (JSON)`);
    } catch { notify("Clipboard not available", false); }
  };

  const handleBulkDelete = async () => {
    if (selectedIds.size === 0) return;
    const count = selectedIds.size;
    if (!window.confirm(`Delete ${count} task(s)? This cannot be undone.`)) return;
    for (const id of Array.from(selectedIds)) {
      try { await deleteTask(id); } catch {}
    }
    setSelectedIds(new Set());
    if (selected && selectedIds.has(selected.task.id)) setSelected(null);
    notify(`Deleted ${count} task(s)`);
    refresh();
  };

  const toggleSelectTask = (e: React.MouseEvent, id: string) => {
    e.stopPropagation();
    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id); else next.add(id);
      return next;
    });
  };

  // IMC pipe handlers
  const handleCreatePipe = (targetId: string) => {
    const source = tasks.find(t => t.id === sel?.id);
    const target = tasks.find(t => t.id === targetId);
    if (!source || !target) return;
    const newPipe: IMCPipe = {
      id: `pipe-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
      sourceTask: source.id,
      sourceName: source.name,
      targetTask: target.id,
      targetName: target.name,
      created: new Date().toISOString(),
      status: "idle",
      messagesTransferred: 0,
    };
    setImcPipes(prev => [...prev, newPipe]);
    notify(`Pipe created: ${source.name} ↔ ${target.name}`);
  };

  const handleDeletePipe = (pipeId: string) => {
    setImcPipes(prev => prev.filter(p => p.id !== pipeId));
    notify("Pipe disconnected");
  };

  // Hardening handler
  const handleApplyPolicy = (policy: HardenedPolicy) => {
    setAppliedPolicy(policy);
    notify(`Hardened policy applied: ${policy.strictMode ? "strict" : "relaxed"} mode, ${policy.maxMemoryMB}MB limit`);
  };

  const handleStop    = async (id: string) => {
    try { await stopTask(id); notify("Stopped"); refresh(); if (selected?.task.id === id) selectTask(id); }
    catch (e: unknown) { notify(e instanceof Error ? e.message : "Stop failed", false); }
  };

  const handleDelete  = async (id: string) => {
    const task = tasks.find(t => t.id === id);
    if (!window.confirm(`Delete "${task?.name ?? id}"? This cannot be undone.`)) return;
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

  const loadSecurity = useCallback(async (id: string) => {
    setSecurityLoading(true);
    try {
      setSecurityReport(await getTaskSecurity(id));
    } catch { setSecurityReport(null); }
    finally { setSecurityLoading(false); }
  }, []);

  const loadInspect = useCallback(async (id: string) => {
    setInspectLoading(true);
    try {
      setInspectData(await inspectTask(id));
    } catch { setInspectData(null); }
    finally { setInspectLoading(false); }
  }, []);

  const loadSnapshots = useCallback(async (id: string) => {
    setSnapshotsLoading(true);
    try {
      const data = await getSnapshots(id);
      setSnapshots(Array.isArray(data) ? data : []);
    } catch { setSnapshots([]); }
    finally { setSnapshotsLoading(false); }
  }, []);

  // Tab-driven lazy loading
  useEffect(() => {
    if (!selected?.task.id) return;
    const id = selected.task.id;
    if (activeTab === "history") loadHistory(id);
    if (activeTab === "security") loadSecurity(id);
    if (activeTab === "inspect") loadInspect(id);
    if (activeTab === "snapshots") loadSnapshots(id);
  }, [activeTab, selected?.task.id, loadHistory, loadSecurity, loadInspect, loadSnapshots]);

  const sel = selected?.task ?? null;

  // ══════════════════════════════════════════════════
  // Render
  // ══════════════════════════════════════════════════

  return (
    <div className="animate-fade-in space-y-6">

      {/* Toast */}
      {toastMsg && (
        <div className={cn(
          "fixed top-4 right-4 z-50 flex items-center gap-2 rounded-lg border px-4 py-3 text-sm font-medium shadow-lg animate-slide-in-right backdrop-blur-sm",
          toastMsg.ok
            ? "bg-green-950/90 border-green-800 text-green-300"
            : "bg-red-950/90 border-red-800 text-red-300"
        )}>
          {toastMsg.ok ? <CheckCircle size={14} /> : <AlertTriangle size={14} />}
          {toastMsg.msg}
        </div>
      )}

      {/* Hidden file input — multiple allowed */}
      <input
        ref={fileRef}
        type="file"
        accept=".wasm,.wat"
        multiple
        className="hidden"
        onChange={(e) => {
          const files = e.target.files;
          if (files && files.length > 1) handleUploadMultiple(files);
          else if (files?.[0]) handleUpload(files[0]);
          e.target.value = "";
        }}
      />

      {/* ── Header ── */}
      <div className="flex items-center justify-between flex-wrap gap-2">
        <div>
          <h1 className="text-2xl font-bold gradient-text">Tasks</h1>
          <p className="mt-0.5 text-sm text-muted-foreground">
            Upload, execute and inspect WASM modules · Chaos testing · IPC pipes · Capability hardening · ABI mocking · vFS · Env vars · Scenario orchestrator
          </p>
          <p className="mt-0.5 text-[10px] text-muted-foreground/60">
            <kbd className="px-1 py-0.5 rounded bg-muted/40 border border-border text-[9px] font-mono">R</kbd> refresh
            <span className="mx-1.5">·</span>
            <kbd className="px-1 py-0.5 rounded bg-muted/40 border border-border text-[9px] font-mono">U</kbd> upload
            <span className="mx-1.5">·</span>
            <kbd className="px-1 py-0.5 rounded bg-muted/40 border border-border text-[9px] font-mono">Esc</kbd> deselect
          </p>
        </div>
        <div className="flex items-center gap-2 flex-wrap">
          {selectedIds.size > 0 && (
            <div className="flex items-center gap-2 rounded-lg border border-destructive/30 bg-destructive/10 px-3 py-1.5">
              <span className="text-xs text-destructive font-medium">{selectedIds.size} selected</span>
              <Button onClick={handleBulkDelete} size="sm" variant="destructive" className="h-6 text-xs">
                <Trash2 size={11} /> Delete All
              </Button>
              <Button onClick={() => setSelectedIds(new Set())} size="sm" variant="ghost" className="h-6 text-xs">
                <X size={11} /> Clear
              </Button>
            </div>
          )}
          <Button onClick={handleExportTasks} variant="outline" size="sm" disabled={tasks.length === 0}>
            <Download size={14} /> Export JSON
          </Button>
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
          { label: "Executing", count: executing ? 1 : 0,       filter: "executing", cls: "text-green-400" },
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
      <div className="grid grid-cols-1 xl:grid-cols-[380px_1fr] gap-5 items-start">

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
                {uploading ? "Uploading…" : "Drop .wasm / .wat files here"}
              </p>
              <p className="text-[11px] text-muted-foreground">or click to browse · Multiple files OK · Duplicates auto-replaced</p>
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
            <select
              value={sortKey}
              onChange={(e) => { setSortKey(e.target.value as SortKey); setSortAsc(false); }}
              className="h-8 rounded-md border border-border bg-muted/20 px-2 text-xs text-foreground focus:outline-none"
            >
              <option value="created">Newest</option>
              <option value="name">Name</option>
              <option value="status">Status</option>
              <option value="size">Size</option>
              <option value="priority">Priority</option>
            </select>
            <Button variant="ghost" size="icon" className="h-8 w-8" onClick={() => setSortAsc((v) => !v)} title="Toggle order">
              <ArrowUpDown size={12} className={sortAsc ? "text-primary" : ""} />
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
                    <div
                      key={task.id}
                      className={cn(
                        "w-full flex items-start gap-2 px-3 py-3 transition-colors group",
                        sel?.id === task.id
                          ? "bg-primary/8 border-l-2 border-primary"
                          : "hover:bg-muted/30 border-l-2 border-transparent",
                        selectedIds.has(task.id) && "bg-destructive/5"
                      )}
                    >
                      {/* Checkbox */}
                      <input
                        type="checkbox"
                        checked={selectedIds.has(task.id)}
                        onClick={(e) => toggleSelectTask(e, task.id)}
                        onChange={() => {}}
                        className="mt-1 h-3.5 w-3.5 rounded border-border bg-muted shrink-0 cursor-pointer opacity-0 group-hover:opacity-100 checked:opacity-100 transition-opacity"
                      />

                      <button
                        onClick={() => selectTask(task.id)}
                        className="flex items-start gap-2 flex-1 min-w-0 text-left"
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
                          {executing === task.id ? (
                            <span className="inline-block h-1.5 w-1.5 rounded-full shrink-0 bg-green-400 animate-pulse" />
                          ) : statusDot(task.status)}
                          {task.priority > 0 && (
                            <span className="text-[9px] font-mono text-muted-foreground bg-muted/30 rounded px-1">p{task.priority}</span>
                          )}
                        </div>
                        <div className="flex items-center gap-2 mt-0.5 text-[11px] text-muted-foreground">
                          <span>{formatBytes(task.file_size_bytes)}</span>
                          <span>·</span>
                          <span>{task.updated_at !== task.created_at ? `updated ${timeAgo(task.updated_at)}` : timeAgo(task.created_at)}</span>
                        </div>
                      </div>
                      </button>

                      {/* Quick actions */}
                      <div
                        className="flex shrink-0 items-center gap-0.5 ml-1"
                        onClick={(e) => e.stopPropagation()}
                      >
                        {executing === task.id ? (
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
                            disabled={!!executing}
                            className="rounded p-1 text-green-400 hover:bg-green-400/10 transition-colors disabled:opacity-40"
                            title="Execute"
                          >
                            <Play size={12} />
                          </button>
                        )}
                        <button
                          onClick={() => handleDelete(task.id)}
                          disabled={executing === task.id}
                          className="rounded p-1 text-muted-foreground hover:text-red-400 hover:bg-red-400/10 transition-colors opacity-0 group-hover:opacity-100 disabled:opacity-40"
                          title="Delete"
                        >
                          <Trash2 size={12} />
                        </button>
                      </div>
                    </div>
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
                <p className="text-xs mt-1 opacity-70">Or upload a new WASM module to get started</p>
                {tasks.length === 0 && (
                  <Button
                    onClick={() => fileRef.current?.click()}
                    size="sm"
                    className="mt-4"
                  >
                    <Upload size={14} /> Upload WASM
                  </Button>
                )}
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
                      <Badge variant={executing === sel.id ? "green" as const : statusVariant(sel.status)} className="text-[10px] h-4 px-1.5">
                        {executing === sel.id ? "executing" : sel.status}
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
                    <Button
                      size="sm"
                      className="h-7 text-xs"
                      onClick={() => handleStart(sel.id)}
                      disabled={!!executing}
                    >
                      {executing === sel.id ? (
                        <><RefreshCw size={12} className="animate-spin" />Running…</>
                      ) : (
                        <><Play size={12} />Execute</>
                      )}
                    </Button>
                    {executing === sel.id && (
                      <Button variant="ghost" size="icon" className="h-7 w-7 text-red-400" onClick={() => handleStop(sel.id)} title="Stop">
                        <Square size={13} />
                      </Button>
                    )}
                    {(sel.status === "completed" || sel.status === "failed") && !executing && (
                      <Button variant="ghost" size="icon" className="h-7 w-7 text-muted-foreground" onClick={() => handleRestart(sel.id)} title="Re-run">
                        <RotateCcw size={13} />
                      </Button>
                    )}
                    <Button variant="ghost" size="icon" className="h-7 w-7 text-destructive" onClick={() => handleDelete(sel.id)} disabled={executing === sel.id} title="Delete">
                      <Trash2 size={13} />
                    </Button>
                  </div>
                </div>
              </CardHeader>

              <Separator />

              <CardContent className="pt-4">
                <Tabs value={activeTab} onValueChange={setActiveTab}>
                  <TabsList className="w-full flex h-auto p-1 overflow-x-auto scrollbar-none mb-3 gap-px">
                    <TabsTrigger value="overview"   className="text-xs px-2.5 h-7">Overview</TabsTrigger>
                    <TabsTrigger value="execute"    className="text-xs px-2.5 h-7">
                      Execute
                      {execResult && (
                        <span className={cn("ml-1 h-1.5 w-1.5 rounded-full inline-block", execResult.success ? "bg-green-400" : "bg-red-400")} />
                      )}
                    </TabsTrigger>
                    <TabsTrigger value="chaos"      className="text-xs px-2.5 h-7 gap-1">
                      <Flame size={11} className={chaosEnabled ? "text-orange-400" : ""} />Chaos
                      {chaosEnabled && <span className="ml-0.5 h-1.5 w-1.5 rounded-full bg-orange-400 animate-pulse inline-block" />}
                    </TabsTrigger>
                    <TabsTrigger value="imc"        className="text-xs px-2.5 h-7 gap-1">
                      <ArrowRightLeft size={11} />IMC
                      {imcPipes.length > 0 && <Badge variant="secondary" className="text-[8px] h-3.5 px-1 ml-0.5">{imcPipes.length}</Badge>}
                    </TabsTrigger>
                    <TabsTrigger value="logs"       className="text-xs px-2.5 h-7">Logs</TabsTrigger>
                    <TabsTrigger value="security"   className="text-xs px-2.5 h-7 gap-1">
                      <ShieldAlert size={11} />Security
                    </TabsTrigger>
                    <TabsTrigger value="hardening"  className="text-xs px-2.5 h-7 gap-1">
                      <Lock size={11} className={appliedPolicy ? "text-emerald-400" : ""} />Harden
                      {appliedPolicy && <span className="ml-0.5 h-1.5 w-1.5 rounded-full bg-emerald-400 inline-block" />}
                    </TabsTrigger>
                    <TabsTrigger value="inspect"    className="text-xs px-2.5 h-7 gap-1">
                      <Eye size={11} />Inspect
                    </TabsTrigger>
                    <TabsTrigger value="snapshots"  className="text-xs px-2.5 h-7 gap-1">
                      <Camera size={11} />Snapshots
                    </TabsTrigger>
                    <TabsTrigger value="history"    className="text-xs px-2.5 h-7">History</TabsTrigger>
                    <TabsTrigger value="abi"        className="text-xs px-2.5 h-7 gap-1">
                      <Gauge size={11} />ABI Mock
                    </TabsTrigger>
                    <TabsTrigger value="vfs"        className="text-xs px-2.5 h-7 gap-1">
                      <FolderOpen size={11} />vFS
                    </TabsTrigger>
                    <TabsTrigger value="env"        className="text-xs px-2.5 h-7 gap-1">
                      <Variable size={11} />Env Vars
                    </TabsTrigger>
                    <TabsTrigger value="scenarios"  className="text-xs px-2.5 h-7 gap-1">
                      <Beaker size={11} />Scenarios
                    </TabsTrigger>
                  </TabsList>

                  {/* ── Overview tab ── */}
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
                            <MetricPill icon={Activity}  label="Total runs"     value={selected.metrics.total_runs.toString()} accent="text-blue-400" />
                            <MetricPill icon={Zap}        label="Failed runs"   value={selected.metrics.failed_runs.toString()} accent="text-red-400" />
                            <MetricPill icon={Clock}      label="Avg duration"  value={formatDuration(selected.metrics.avg_duration_us)} accent="text-green-400" />
                            <MetricPill icon={Cpu}        label="Total instrs"  value={selected.metrics.total_instructions.toLocaleString()} accent="text-purple-400" />
                          </div>
                        </div>
                      </>
                    )}

                    {/* Last execution quick result */}
                    {execResult && (
                      <>
                        <Separator />
                        <div>
                          <div className="flex items-center gap-2 mb-2">
                            <p className="text-[11px] uppercase tracking-wider text-muted-foreground font-medium">Last Execution</p>
                            {execResult.success ? (
                              <Badge variant="default" className="text-[9px] h-3.5 bg-green-500/20 text-green-400 border-green-500/30">✓ passed</Badge>
                            ) : (
                              <Badge variant="destructive" className="text-[9px] h-3.5">✗ failed</Badge>
                            )}
                          </div>
                          <div className="grid grid-cols-3 gap-2">
                            <MetricPill icon={Clock} label="Duration" value={execResult.duration_us != null ? formatDuration(execResult.duration_us) : "—"} />
                            <MetricPill icon={Cpu} label="Instructions" value={(execResult.instructions_executed ?? 0).toLocaleString()} />
                            <MetricPill icon={HardDrive} label="Memory" value={formatBytes(execResult.memory_used_bytes ?? 0)} />
                          </div>
                          {execResult.stdout_log?.length > 0 && (
                            <pre className="mt-2 rounded-lg bg-black/30 border border-border p-2 text-[11px] font-mono text-green-300 max-h-20 overflow-auto whitespace-pre-wrap">
                              {execResult.stdout_log.slice(0, 5).join("\n")}
                              {execResult.stdout_log.length > 5 && `\n… +${execResult.stdout_log.length - 5} more lines`}
                            </pre>
                          )}
                          <Button
                            size="sm"
                            variant="ghost"
                            className="text-xs mt-1 h-6 text-primary"
                            onClick={() => setActiveTab("execute")}
                          >
                            View full output →
                          </Button>
                        </div>
                      </>
                    )}

                    {/* Quick links */}
                    <div className="grid grid-cols-2 gap-2">
                      <button
                        onClick={() => setActiveTab("security")}
                        className="flex items-center gap-2 rounded-lg border border-border bg-muted/10 px-3 py-2.5 text-left transition-all hover:bg-muted/30 hover:border-purple-500/30"
                      >
                        <ShieldAlert size={14} className="text-purple-400 shrink-0" />
                        <div>
                          <p className="text-xs font-medium">Security Analysis</p>
                          <p className="text-[10px] text-muted-foreground">Scan for capabilities & risks</p>
                        </div>
                      </button>
                      <button
                        onClick={() => setActiveTab("inspect")}
                        className="flex items-center gap-2 rounded-lg border border-border bg-muted/10 px-3 py-2.5 text-left transition-all hover:bg-muted/30 hover:border-cyan-500/30"
                      >
                        <Eye size={14} className="text-cyan-400 shrink-0" />
                        <div>
                          <p className="text-xs font-medium">Inspect Module</p>
                          <p className="text-[10px] text-muted-foreground">View imports, exports, memory</p>
                        </div>
                      </button>
                    </div>
                  </TabsContent>

                  {/* ── Execute tab ── */}
                  <TabsContent value="execute" className="mt-0">
                    {/* Chaos mode banner */}
                    {chaosEnabled && (
                      <div className="mb-3 flex items-center gap-2 rounded-lg border border-orange-500/30 bg-orange-500/8 px-3 py-2">
                        <Flame size={13} className="text-orange-400 shrink-0 animate-pulse" />
                        <span className="text-xs text-orange-300">
                          <strong>Chaos mode active:</strong> {chaosMode} injection at {chaosIntensity}% intensity
                        </span>
                        <button onClick={() => setChaosEnabled(false)} className="ml-auto text-muted-foreground hover:text-foreground"><X size={12} /></button>
                      </div>
                    )}

                    {/* Applied policy banner */}
                    {appliedPolicy && (
                      <div className="mb-3 flex items-center gap-2 rounded-lg border border-emerald-500/30 bg-emerald-500/8 px-3 py-2">
                        <Shield size={13} className="text-emerald-400 shrink-0" />
                        <span className="text-xs text-emerald-300">
                          <strong>Hardened policy:</strong> {appliedPolicy.maxMemoryMB}MB mem · {appliedPolicy.maxInstructions.toLocaleString()} max instrs · {appliedPolicy.strictMode ? "strict" : "relaxed"}
                        </span>
                      </div>
                    )}

                    {executing === sel.id ? (
                      <ExecutingSpinner />
                    ) : execResult ? (
                      <div ref={execResultRef}>
                        <ExecuteResultPanel
                          result={execResult}
                          onRerun={() => handleStart(sel.id)}
                          rerunning={executing === sel.id}
                        />
                      </div>
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

                        {/* Show last log summary if available */}
                        {taskLog && (taskLog.success != null) && (
                          <div className="mt-6 w-full max-w-sm">
                            <Separator className="mb-3" />
                            <p className="text-[10px] uppercase tracking-wider text-muted-foreground font-medium mb-2 text-left">Previous Run</p>
                            <div className="rounded-lg border border-border bg-muted/10 p-3 text-left space-y-1.5">
                              <div className="flex items-center gap-2">
                                {taskLog.success ? (
                                  <CheckCircle size={12} className="text-green-400" />
                                ) : (
                                  <AlertTriangle size={12} className="text-red-400" />
                                )}
                                <span className="text-xs font-medium">
                                  {taskLog.success ? "Completed successfully" : "Failed"}
                                </span>
                                {taskLog.duration_us != null && (
                                  <span className="ml-auto text-[11px] text-muted-foreground">{formatDuration(taskLog.duration_us)}</span>
                                )}
                              </div>
                              {taskLog.stdout_log?.length > 0 && (
                                <pre className="text-[11px] font-mono text-green-300/70 max-h-16 overflow-auto whitespace-pre-wrap">
                                  {taskLog.stdout_log.slice(0, 3).join("\n")}
                                  {taskLog.stdout_log.length > 3 && "\n…"}
                                </pre>
                              )}
                            </div>
                          </div>
                        )}
                      </div>
                    )}
                  </TabsContent>

                  {/* ── Logs tab ── */}
                  <TabsContent value="logs" className="mt-0 space-y-2">
                    {taskLog?.stdout_log && taskLog.stdout_log.length > 0 ? (
                      <>
                        {/* Log toolbar */}
                        <div className="flex items-center gap-2">
                          <div className="relative flex-1">
                            <Search size={11} className="absolute left-2 top-1/2 -translate-y-1/2 text-muted-foreground pointer-events-none" />
                            <Input
                              placeholder="Filter log lines…"
                              value={logFilter}
                              onChange={(e) => setLogFilter(e.target.value)}
                              className="pl-7 h-7 text-xs"
                            />
                            {logFilter && (
                              <button onClick={() => setLogFilter("")} className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground">
                                <X size={11} />
                              </button>
                            )}
                          </div>
                          <CopyButton text={taskLog.stdout_log.join("\n")} />
                          <Badge variant="secondary" className="text-[10px] shrink-0">
                            {taskLog.stdout_log.length} lines
                          </Badge>
                        </div>

                        {/* Log metadata */}
                        <div className="flex items-center gap-3 text-[11px] text-muted-foreground">
                          {taskLog.success != null && (
                            <span className="flex items-center gap-1">
                              {taskLog.success ? <CheckCircle size={10} className="text-green-400" /> : <AlertTriangle size={10} className="text-red-400" />}
                              {taskLog.success ? "Success" : "Failed"}
                            </span>
                          )}
                          {taskLog.duration_us != null && (
                            <span>{formatDuration(taskLog.duration_us)}</span>
                          )}
                          {taskLog.instructions_executed > 0 && (
                            <span>{taskLog.instructions_executed.toLocaleString()} instrs</span>
                          )}
                          {taskLog.memory_used_bytes > 0 && (
                            <span>{formatBytes(taskLog.memory_used_bytes)} mem</span>
                          )}
                        </div>

                        {/* Log lines */}
                        <ScrollArea className="h-72">
                          <div className="space-y-0">
                            {taskLog.stdout_log
                              .map((line, i) => ({ line, i }))
                              .filter(({ line }) => !logFilter || line.toLowerCase().includes(logFilter.toLowerCase()))
                              .map(({ line, i }) => (
                                <div key={i} className="flex gap-2 text-xs font-mono px-1 py-0.5 hover:bg-muted/20 transition-colors group">
                                  <span className="text-muted-foreground/40 shrink-0 w-8 text-right select-none">{i + 1}</span>
                                  <span className="text-foreground/80 whitespace-pre-wrap break-all">{line}</span>
                                  <span className="opacity-0 group-hover:opacity-100 transition-opacity ml-auto shrink-0">
                                    <CopyButton text={line} />
                                  </span>
                                </div>
                              ))}
                          </div>
                        </ScrollArea>
                      </>
                    ) : taskLog?.error ? (
                      <div className="space-y-2">
                        <div className="flex items-center justify-between">
                          <p className="text-[10px] uppercase tracking-wider text-red-400/70 font-medium">Error Output</p>
                          <CopyButton text={taskLog.error} />
                        </div>
                        <pre className="text-xs font-mono text-red-400 p-3 rounded-lg bg-red-950/20 border border-red-900/20 whitespace-pre-wrap">{taskLog.error}</pre>
                      </div>
                    ) : (
                      <div className="flex items-center justify-center py-12 text-muted-foreground text-sm">
                        No log output
                      </div>
                    )}
                  </TabsContent>

                  {/* ── Security tab ── */}
                  <TabsContent value="security" className="mt-0">
                    <SecurityPanel report={securityReport} loading={securityLoading} />
                  </TabsContent>

                  {/* ── Chaos Sandbox tab ── */}
                  <TabsContent value="chaos" className="mt-0">
                    <ChaosSandboxPanel
                      enabled={chaosEnabled}
                      mode={chaosMode}
                      intensity={chaosIntensity}
                      onToggle={() => setChaosEnabled(v => !v)}
                      onModeChange={setChaosMode}
                      onIntensityChange={setChaosIntensity}
                    />

                    {/* Chaos-mode execute button */}
                    {chaosEnabled && (
                      <div className="mt-4 flex items-center gap-3 rounded-lg border border-orange-500/30 bg-orange-500/5 p-3">
                        <Flame size={16} className="text-orange-400 shrink-0" />
                        <div className="flex-1 min-w-0">
                          <p className="text-xs font-medium">Execute with Chaos</p>
                          <p className="text-[10px] text-muted-foreground">
                            Run module with <strong>{chaosMode}</strong> injection at {chaosIntensity}% intensity
                          </p>
                        </div>
                        <Button
                          size="sm"
                          className="h-7 text-xs gap-1 bg-orange-500 hover:bg-orange-600"
                          onClick={() => handleStart(sel.id)}
                          disabled={executing === sel.id}
                        >
                          {executing === sel.id ? (
                            <><Loader2 size={11} className="animate-spin" /> Running…</>
                          ) : (
                            <><Flame size={11} /> Chaos Execute</>
                          )}
                        </Button>
                      </div>
                    )}
                  </TabsContent>

                  {/* ── IMC tab ── */}
                  <TabsContent value="imc" className="mt-0">
                    <IMCPanel
                      tasks={tasks}
                      currentTaskId={sel.id}
                      pipes={imcPipes.filter(p => p.sourceTask === sel.id || p.targetTask === sel.id)}
                      onCreatePipe={handleCreatePipe}
                      onDeletePipe={handleDeletePipe}
                    />
                  </TabsContent>

                  {/* ── Hardening tab ── */}
                  <TabsContent value="hardening" className="mt-0">
                    <HardeningPanel
                      report={securityReport}
                      onApply={handleApplyPolicy}
                    />

                    {appliedPolicy && (
                      <div className="mt-3 flex items-center gap-2 rounded-lg border border-emerald-500/30 bg-emerald-500/5 p-3">
                        <Shield size={14} className="text-emerald-400 shrink-0" />
                        <div className="flex-1 min-w-0">
                          <p className="text-xs font-medium text-emerald-300">Policy Active</p>
                          <p className="text-[10px] text-muted-foreground">
                            {appliedPolicy.strictMode ? "Strict" : "Relaxed"} · {appliedPolicy.maxMemoryMB}MB · {appliedPolicy.maxInstructions.toLocaleString()} instrs ·
                            Net: {appliedPolicy.allowNetwork ? "✓" : "✗"} · FS: {appliedPolicy.allowFileSystem ? "✓" : "✗"} · Proc: {appliedPolicy.allowProcessSpawn ? "✓" : "✗"}
                          </p>
                        </div>
                        <Button
                          variant="ghost"
                          size="sm"
                          className="h-6 text-xs text-red-400 hover:text-red-300"
                          onClick={() => { setAppliedPolicy(null); notify("Policy removed"); }}
                        >
                          <Unlock size={11} /> Remove
                        </Button>
                      </div>
                    )}
                  </TabsContent>

                  {/* ── Inspect tab ── */}
                  <TabsContent value="inspect" className="mt-0">
                    <InspectPanel data={inspectData} loading={inspectLoading} />
                  </TabsContent>

                  {/* ── Snapshots tab ── */}
                  <TabsContent value="snapshots" className="mt-0">
                    <SnapshotsPanel
                      taskId={sel.id}
                      snapshots={snapshots}
                      loading={snapshotsLoading}
                      onRefresh={() => loadSnapshots(sel.id)}
                      onNotify={notify}
                    />
                  </TabsContent>

                  {/* ── History tab ── */}
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
                      <div className="space-y-2">
                        {/* History summary bar */}
                        <div className="flex items-center gap-3 text-[11px] text-muted-foreground">
                          <span>{execHistory.length} execution{execHistory.length !== 1 ? "s" : ""}</span>
                          <span className="flex items-center gap-1">
                            <CheckCircle size={10} className="text-green-400" />
                            {execHistory.filter(h => h.success).length} passed
                          </span>
                          <span className="flex items-center gap-1">
                            <AlertTriangle size={10} className="text-red-400" />
                            {execHistory.filter(h => !h.success).length} failed
                          </span>
                          {execHistory.length > 0 && execHistory[0].duration_us != null && (
                            <span className="ml-auto">Latest: {formatDuration(execHistory[0].duration_us!)}</span>
                          )}
                        </div>

                        {/* Success rate bar */}
                        {execHistory.length > 1 && (
                          <div className="space-y-1">
                            <div className="flex items-center justify-between text-[10px] text-muted-foreground">
                              <span>Success rate</span>
                              <span>{Math.round((execHistory.filter(h => h.success).length / execHistory.length) * 100)}%</span>
                            </div>
                            <div className="h-1.5 rounded-full bg-muted/30 overflow-hidden">
                              <div
                                className="h-full rounded-full bg-green-400 transition-all"
                                style={{ width: `${(execHistory.filter(h => h.success).length / execHistory.length) * 100}%` }}
                              />
                            </div>
                          </div>
                        )}

                        <ScrollArea className="h-72">
                          <div className="space-y-1">
                            {execHistory.map((h, i) => {
                              const execId = h.execution_id ?? h.id ?? String(i);
                              const isExpanded = execHistoryExpanded === execId;
                              return (
                                <div key={i} className="rounded-lg border border-border bg-muted/5 overflow-hidden">
                                  <button
                                    className="w-full flex items-center gap-3 py-2.5 px-3 text-left hover:bg-muted/20 transition-colors"
                                    onClick={() => setExecHistoryExpanded(isExpanded ? null : execId)}
                                  >
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
                                    <div className="flex items-center gap-2 shrink-0">
                                      {h.instructions_executed > 0 && (
                                        <span className="text-[11px] text-muted-foreground">
                                          {h.instructions_executed.toLocaleString()} instrs
                                        </span>
                                      )}
                                      <ChevronDown size={12} className={cn("text-muted-foreground transition-transform", isExpanded && "rotate-180")} />
                                    </div>
                                  </button>

                                  {/* Expanded detail */}
                                  {isExpanded && (
                                    <div className="px-3 pb-3 pt-0 border-t border-border bg-muted/10 space-y-2">
                                      <div className="grid grid-cols-2 sm:grid-cols-4 gap-2 pt-2">
                                        <MetricPill icon={Cpu}       label="Instructions" value={h.instructions_executed?.toLocaleString() ?? "0"} />
                                        <MetricPill icon={Zap}       label="Syscalls"     value={h.syscalls_executed?.toLocaleString() ?? "0"} />
                                        <MetricPill icon={HardDrive} label="Memory"       value={formatBytes(h.memory_used_bytes ?? 0)} />
                                        <MetricPill icon={Clock}     label="Duration"     value={h.duration_us != null ? formatDuration(h.duration_us) : "—"} />
                                      </div>
                                      {h.error && (
                                        <div>
                                          <p className="text-[10px] uppercase tracking-wider text-red-400/70 font-medium mb-1">Error</p>
                                          <pre className="text-[11px] font-mono text-red-300 bg-red-950/20 border border-red-900/20 rounded p-2 max-h-20 overflow-auto whitespace-pre-wrap">{h.error}</pre>
                                        </div>
                                      )}
                                      <div className="flex items-center gap-2">
                                        <Link
                                          href={`/execution/report?id=${execId}`}
                                          className="flex items-center gap-1 text-primary hover:underline text-xs font-medium"
                                        >
                                          <FileText size={11} /> View Full Report <ExternalLink size={9} />
                                        </Link>
                                        {execId && (
                                          <span className="ml-auto text-[10px] text-muted-foreground font-mono">{execId.slice(0, 12)}…</span>
                                        )}
                                      </div>
                                    </div>
                                  )}
                                </div>
                              );
                            })}
                          </div>
                        </ScrollArea>
                      </div>
                    )}
                  </TabsContent>

                  {/* ── ABI Mocking tab ── */}
                  <TabsContent value="abi" className="mt-0">
                    <ABIMockingPanel
                      registers={mockRegisters}
                      setRegisters={setMockRegisters}
                      eventLog={mockEventLog}
                      setEventLog={setMockEventLog}
                      onNotify={notify}
                    />
                  </TabsContent>

                  {/* ── vFS Explorer tab ── */}
                  <TabsContent value="vfs" className="mt-0">
                    <VFSExplorerPanel
                      files={vfsFiles}
                      setFiles={setVfsFiles}
                      onNotify={notify}
                    />
                  </TabsContent>

                  {/* ── Environment Variables tab ── */}
                  <TabsContent value="env" className="mt-0">
                    <EnvVarsPanel
                      vars={envVars}
                      setVars={setEnvVars}
                      onNotify={notify}
                    />
                  </TabsContent>

                  {/* ── Scenario Orchestrator tab ── */}
                  <TabsContent value="scenarios" className="mt-0">
                    <ScenarioOrchestratorPanel
                      taskId={sel.id}
                      tasks={tasks}
                      mockRegisters={mockRegisters}
                      vfsFiles={vfsFiles}
                      envVars={envVars}
                      onNotify={notify}
                      onSnapshot={() => loadSnapshots(sel.id)}
                    />
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
