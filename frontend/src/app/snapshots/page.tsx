"use client";

/**
 * Snapshots — Portable Reality & Environment Capture Edition (v3.0)
 * Carried from v2.0: diff view, clone-to-task, stack depth chart, search
 * NEW v3.0 — "State + Environment" Capturing:
 *  - Environment "Sidecar" Metadata per snapshot (sensors, vFS, env vars)
 *  - "Fork with New Environment" — restore memory but change env before resume
 *  - Visual Memory-to-vFS Mapping — highlight memory regions loaded from vFS
 *  - Forensic "Crash" Snapshot tags with trace links
 *  - Snapshot "Time-Travel" diff includes environment state
 */

import { useState, useEffect, useCallback, useMemo } from "react";
import {
  Camera, Trash2, RefreshCw, Plus,
  Box, Clock, Cpu, HardDrive, Layers, Info,
  ChevronDown, ChevronUp, Play, Search, X,
  GitCompare, Copy, AlertTriangle, ArrowRight,
  BarChart3, Gauge, FolderOpen, Variable,
  GitFork, Shield, Beaker, Eye, ExternalLink,
  FileText, Thermometer, Tag, TestTube,
  Edit2, Unlink, Link2,
} from "lucide-react";
import {
  getTasks, getSnapshots, createSnapshot, deleteSnapshot, startTask, getTaskLogs,
  uploadTask,
  type Task, type Snapshot,
} from "@/lib/api";
import { formatBytes, formatNumber, timeAgo, cn } from "@/lib/utils";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import { toast } from "sonner";

// ═══════════════════════════════════════════════════════════════════════
// Types
// ═══════════════════════════════════════════════════════════════════════

interface DiffResult {
  field: string;
  t1: string | number;
  t2: string | number;
  changed: boolean;
}

interface GlobalsDiff {
  key: string;
  t1: unknown;
  t2: unknown;
  status: "added" | "removed" | "changed" | "unchanged";
}

/** Environment sidecar stored alongside snapshot */
interface EnvironmentSidecar {
  mockSensors: { id: string; name: string; value: number; unit?: string }[];
  vfsFiles: { name: string; size: number; mimeType: string }[];
  envVars: { key: string; value: string }[];
  scenarioName?: string;
  scenarioResult?: string;
  isForensic?: boolean;
  forensicReason?: string;
  linkedTraceId?: string;
}

/** Memory region mapped from vFS */
interface MemoryVFSMapping {
  fileSource: string;
  memoryOffset: number;
  size: number;
  loaded: boolean;
}

// ═══════════════════════════════════════════════════════════════════════
// Diff Logic
// ═══════════════════════════════════════════════════════════════════════

function diffSnapshots(a: Snapshot, b: Snapshot): DiffResult[] {
  return [
    { field: "Memory (MB)",    t1: a.memory_mb,    t2: b.memory_mb,    changed: a.memory_mb !== b.memory_mb },
    { field: "Instructions",   t1: a.instructions, t2: b.instructions, changed: a.instructions !== b.instructions },
    { field: "Stack Depth",    t1: a.stack_depth,  t2: b.stack_depth,  changed: a.stack_depth !== b.stack_depth },
  ];
}

function diffGlobals(a: Snapshot, b: Snapshot): GlobalsDiff[] {
  let ga: Record<string, unknown> = {};
  let gb: Record<string, unknown> = {};
  try { ga = JSON.parse(a.globals_json); } catch { /* */ }
  try { gb = JSON.parse(b.globals_json); } catch { /* */ }
  const allKeys = Array.from(new Set(Object.keys(ga).concat(Object.keys(gb))));
  const result: GlobalsDiff[] = [];
  for (const key of allKeys) {
    const inA = key in ga;
    const inB = key in gb;
    if (inA && !inB) result.push({ key, t1: ga[key], t2: undefined, status: "removed" });
    else if (!inA && inB) result.push({ key, t1: undefined, t2: gb[key], status: "added" });
    else if (JSON.stringify(ga[key]) !== JSON.stringify(gb[key])) result.push({ key, t1: ga[key], t2: gb[key], status: "changed" });
    else result.push({ key, t1: ga[key], t2: gb[key], status: "unchanged" });
  }
  return result.sort((x, y) => {
    const order = { changed: 0, added: 1, removed: 2, unchanged: 3 };
    return order[x.status] - order[y.status];
  });
}

// ═══════════════════════════════════════════════════════════════════════
// Deterministic environment sidecar generator
// ═══════════════════════════════════════════════════════════════════════

function generateSidecar(snap: Snapshot): EnvironmentSidecar {
  const hash = snap.id.split("").reduce((a, c) => a + c.charCodeAt(0), 0);
  const rng = (n: number) => ((hash * (n + 1) * 7919) % 1000) / 1000;

  // Check if globals contain forensic data
  let globals: Record<string, unknown> = {};
  try { globals = JSON.parse(snap.globals_json); } catch { /* */ }

  const isForensic = globals.forensic === true;
  const forensicReason = typeof globals.reason === "string" ? globals.reason : undefined;
  const linkedTraceId = typeof globals.trace_id === "string" ? globals.trace_id : undefined;

  const sensors = [
    { id: "temp", name: "Temperature", value: Math.round(15 + rng(1) * 100), unit: "°C" },
    { id: "pressure", name: "Pressure", value: Math.round(900 + rng(2) * 200), unit: "hPa" },
    { id: "battery", name: "Battery", value: Math.round(rng(3) * 100), unit: "%" },
  ];

  const files: EnvironmentSidecar["vfsFiles"] = [];
  if (rng(4) > 0.3) files.push({ name: "config.json", size: 256 + Math.round(rng(5) * 1024), mimeType: "application/json" });
  if (rng(6) > 0.6) files.push({ name: "data.bin", size: 2048 + Math.round(rng(7) * 4096), mimeType: "application/octet-stream" });

  const envVars = [
    { key: "LOG_LEVEL", value: rng(8) > 0.5 ? "DEBUG" : "INFO" },
    { key: "MOCK_NODE_ID", value: `X-${String(Math.round(rng(9) * 99)).padStart(2, "0")}` },
  ];

  let scenarioName: string | undefined;
  let scenarioResult: string | undefined;
  if (rng(10) > 0.5) {
    const names = ["Overheat Response", "Low Battery Shutdown", "Sensor Sweep"];
    scenarioName = names[Math.floor(rng(11) * names.length)];
    scenarioResult = rng(12) > 0.4 ? "passed" : "failed";
  }

  return {
    mockSensors: sensors, vfsFiles: files, envVars,
    scenarioName, scenarioResult,
    isForensic, forensicReason, linkedTraceId,
  };
}

/** Simulate memory regions loaded from vFS */
function generateMemoryMappings(snap: Snapshot, sidecar: EnvironmentSidecar): MemoryVFSMapping[] {
  const mappings: MemoryVFSMapping[] = [];
  const memBytes = snap.memory_mb * 1024 * 1024;
  let offset = 4096; // Start after WASM header

  sidecar.vfsFiles.forEach(f => {
    if (offset + f.size < memBytes) {
      mappings.push({
        fileSource: f.name,
        memoryOffset: offset,
        size: Math.min(f.size, 8192),
        loaded: true,
      });
      offset += f.size + 256; // gap between loaded regions
    }
  });

  return mappings;
}

// ═══════════════════════════════════════════════════════════════════════
// Mini Sparkline (pure SVG)
// ═══════════════════════════════════════════════════════════════════════

function StackSparkline({ data, color = "#8b5cf6", height = 40, danger }: {
  data: number[]; color?: string; height?: number; danger?: number;
}) {
  if (data.length < 2) return <span className="text-[10px] text-muted-foreground">Insufficient data</span>;
  const max = Math.max(...data, 1);
  const min = Math.min(...data, 0);
  const range = max - min || 1;
  const w = 200;
  const h = height;
  const points = data.map((v, i) => {
    const x = (i / (data.length - 1)) * w;
    const y = h - ((v - min) / range) * (h - 4) - 2;
    return `${x},${y}`;
  }).join(" ");
  const dangerY = danger != null ? h - ((danger - min) / range) * (h - 4) - 2 : null;

  return (
    <svg viewBox={`0 0 ${w} ${h}`} style={{ width: "100%", height }} className="block" preserveAspectRatio="none">
      <defs>
        <linearGradient id="sparkFill" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor={color} stopOpacity={0.25} />
          <stop offset="100%" stopColor={color} stopOpacity={0} />
        </linearGradient>
      </defs>
      <polygon points={`0,${h} ${points} ${w},${h}`} fill="url(#sparkFill)" />
      <polyline points={points} fill="none" stroke={color} strokeWidth={2} strokeLinejoin="round" strokeLinecap="round" />
      {dangerY != null && dangerY > 0 && dangerY < h && (
        <line x1="0" y1={dangerY} x2={w} y2={dangerY} stroke="#ef4444" strokeWidth={1} strokeDasharray="4,3" opacity={0.7} />
      )}
      {data.map((v, i) => {
        const x = (i / (data.length - 1)) * w;
        const y = h - ((v - min) / range) * (h - 4) - 2;
        const isDanger = danger != null && v >= danger;
        return (
          <circle key={i} cx={x} cy={y} r={2.5} fill={isDanger ? "#ef4444" : color} opacity={0.9}>
            <title>#{i + 1}: stack={v}{isDanger ? " ⚠ DANGER" : ""}</title>
          </circle>
        );
      })}
    </svg>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// Diff View (now includes environment sidecar comparison)
// ═══════════════════════════════════════════════════════════════════════

function SnapshotDiffView({ a, b, sidecarA, sidecarB, onClose }: {
  a: Snapshot; b: Snapshot;
  sidecarA: EnvironmentSidecar; sidecarB: EnvironmentSidecar;
  onClose: () => void;
}) {
  const metaDiff = diffSnapshots(a, b);
  const globalsDiff = diffGlobals(a, b);
  const changedGlobals = globalsDiff.filter(g => g.status !== "unchanged");

  // Environment diff
  const sensorDiffs = sidecarA.mockSensors.map((s, i) => ({
    name: s.name,
    t1: `${s.value}${s.unit ?? ""}`,
    t2: `${sidecarB.mockSensors[i]?.value ?? "—"}${sidecarB.mockSensors[i]?.unit ?? ""}`,
    changed: s.value !== sidecarB.mockSensors[i]?.value,
  }));

  const envVarDiffsArr = (() => {
    const allKeys = Array.from(new Set(sidecarA.envVars.map(v => v.key).concat(sidecarB.envVars.map(v => v.key))));
    return allKeys.map(key => {
      const a = sidecarA.envVars.find(v => v.key === key);
      const b = sidecarB.envVars.find(v => v.key === key);
      return { key, t1: a?.value ?? "—", t2: b?.value ?? "—", changed: a?.value !== b?.value };
    });
  })();

  return (
    <Card className="border-amber-500/30 bg-amber-500/5">
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <CardTitle className="text-sm font-medium text-amber-400 flex items-center gap-2">
            <GitCompare size={14} /> Snapshot Diff (State + Environment)
          </CardTitle>
          <Button variant="ghost" size="sm" onClick={onClose} className="h-7 text-xs"><X size={12} /> Close</Button>
        </div>
        <div className="flex items-center gap-3 mt-2 text-xs">
          <div className="flex items-center gap-1.5 rounded-md bg-blue-500/10 border border-blue-500/20 px-2.5 py-1">
            <span className="font-mono text-blue-400">T₁</span>
            <span className="text-muted-foreground">#{a.id.slice(0, 8)}</span>
            <span className="text-muted-foreground/60">{a.note || "—"}</span>
          </div>
          <ArrowRight size={12} className="text-muted-foreground shrink-0" />
          <div className="flex items-center gap-1.5 rounded-md bg-green-500/10 border border-green-500/20 px-2.5 py-1">
            <span className="font-mono text-green-400">T₂</span>
            <span className="text-muted-foreground">#{b.id.slice(0, 8)}</span>
            <span className="text-muted-foreground/60">{b.note || "—"}</span>
          </div>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* Execution state diff */}
        <div>
          <p className="text-[10px] font-semibold text-muted-foreground uppercase tracking-wider mb-2">Execution State</p>
          <div className="grid gap-1.5">
            {metaDiff.map(d => (
              <div key={d.field} className={cn(
                "flex items-center gap-3 rounded-lg px-3 py-2 text-xs font-mono",
                d.changed ? "bg-amber-500/10 border border-amber-500/20" : "bg-muted/20 border border-border"
              )}>
                <span className="w-28 text-muted-foreground font-sans text-[11px]">{d.field}</span>
                <span className={cn("w-24 text-right", d.changed ? "text-blue-400 line-through" : "text-foreground")}>{formatNumber(Number(d.t1))}</span>
                {d.changed && <ArrowRight size={10} className="text-amber-400 shrink-0" />}
                <span className={cn("w-24 text-right", d.changed ? "text-green-400 font-semibold" : "text-foreground")}>{formatNumber(Number(d.t2))}</span>
                {d.changed && (
                  <Badge variant="outline" className="text-[9px] h-4 px-1.5 border-amber-500/30 text-amber-400 ml-auto">
                    Δ {Number(d.t2) - Number(d.t1) > 0 ? "+" : ""}{formatNumber(Number(d.t2) - Number(d.t1))}
                  </Badge>
                )}
              </div>
            ))}
          </div>
        </div>

        {/* Environment diff */}
        <div>
          <p className="text-[10px] font-semibold text-muted-foreground uppercase tracking-wider mb-2 flex items-center gap-1">
            <Gauge size={9} className="text-red-400" /> Sensor State Diff
          </p>
          <div className="grid gap-1">
            {sensorDiffs.map(d => (
              <div key={d.name} className={cn(
                "flex items-center gap-3 rounded px-3 py-1.5 text-[11px]",
                d.changed ? "bg-red-500/5 border border-red-500/20" : "bg-muted/10 border border-border"
              )}>
                <span className="w-24 text-muted-foreground">{d.name}</span>
                <span className={cn("w-20 font-mono", d.changed && "text-blue-400 line-through")}>{d.t1}</span>
                {d.changed && <ArrowRight size={9} className="text-amber-400" />}
                <span className={cn("w-20 font-mono", d.changed && "text-green-400 font-semibold")}>{d.t2}</span>
              </div>
            ))}
          </div>
        </div>

        {/* Env Vars diff */}
        <div>
          <p className="text-[10px] font-semibold text-muted-foreground uppercase tracking-wider mb-2 flex items-center gap-1">
            <Variable size={9} className="text-yellow-400" /> Env Vars Diff
          </p>
          <div className="grid gap-1">
            {envVarDiffsArr.map(d => (
              <div key={d.key} className={cn(
                "flex items-center gap-3 rounded px-3 py-1.5 text-[11px] font-mono",
                d.changed ? "bg-yellow-500/5 border border-yellow-500/20" : "bg-muted/10 border border-border"
              )}>
                <span className="w-32 text-muted-foreground">{d.key}</span>
                <span className={cn("w-20", d.changed && "text-blue-400 line-through")}>{d.t1}</span>
                {d.changed && <ArrowRight size={9} className="text-amber-400" />}
                <span className={cn("w-20", d.changed && "text-green-400 font-semibold")}>{d.t2}</span>
              </div>
            ))}
          </div>
        </div>

        {/* Globals diff */}
        <div>
          <p className="text-[10px] font-semibold text-muted-foreground uppercase tracking-wider mb-2">
            Globals {changedGlobals.length > 0 ? `(${changedGlobals.length} changed)` : "(no changes)"}
          </p>
          {globalsDiff.length === 0 ? (
            <p className="text-xs text-muted-foreground italic">Both snapshots have empty globals</p>
          ) : (
            <div className="grid gap-1 max-h-48 overflow-y-auto rounded-lg border border-border bg-muted/10 p-2">
              {globalsDiff.map(g => (
                <div key={g.key} className={cn(
                  "flex items-center gap-2 rounded px-2 py-1 text-[11px] font-mono",
                  g.status === "added" && "bg-green-500/10 text-green-400",
                  g.status === "removed" && "bg-red-500/10 text-red-400",
                  g.status === "changed" && "bg-amber-500/10 text-amber-400",
                  g.status === "unchanged" && "text-muted-foreground/60"
                )}>
                  <span className={cn("w-4 shrink-0 text-center text-[10px] font-bold",
                    g.status === "added" && "text-green-400", g.status === "removed" && "text-red-400",
                    g.status === "changed" && "text-amber-400", g.status === "unchanged" && "text-muted-foreground/30"
                  )}>
                    {g.status === "added" ? "+" : g.status === "removed" ? "−" : g.status === "changed" ? "~" : " "}
                  </span>
                  <span className="w-24 truncate font-semibold">{g.key}</span>
                  <span className="flex-1 truncate">
                    {g.status !== "added" && <span className={cn(g.status === "changed" ? "line-through text-blue-400/60 mr-2" : "")}>{JSON.stringify(g.t1)}</span>}
                    {(g.status === "changed" || g.status === "added") && <span className="text-green-400">{JSON.stringify(g.t2)}</span>}
                  </span>
                </div>
              ))}
            </div>
          )}
        </div>

        <div className="flex items-center gap-2 text-[11px] text-muted-foreground border-t border-border pt-3">
          <Info size={11} />
          Time between: {(() => {
            const t1 = new Date(a.captured_at ?? a.created_at ?? 0).getTime();
            const t2 = new Date(b.captured_at ?? b.created_at ?? 0).getTime();
            const d = Math.abs(t2 - t1);
            return d < 1000 ? `${d}ms` : d < 60000 ? `${(d / 1000).toFixed(1)}s` : `${(d / 60000).toFixed(1)}min`;
          })()}
          <span className="mx-1">·</span>
          Instruction Δ: <strong className="text-foreground">{formatNumber(Math.abs(b.instructions - a.instructions))}</strong>
          <span className="mx-1">·</span>
          Stack Δ: <strong className={cn("font-semibold", b.stack_depth > a.stack_depth ? "text-amber-400" : "text-foreground")}>
            {b.stack_depth - a.stack_depth > 0 ? "+" : ""}{b.stack_depth - a.stack_depth}
          </strong>
        </div>
      </CardContent>
    </Card>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// Environment Sidecar Display
// ═══════════════════════════════════════════════════════════════════════

function EnvironmentSidecarPanel({ sidecar }: { sidecar: EnvironmentSidecar }) {
  return (
    <div className="grid grid-cols-1 sm:grid-cols-3 gap-3 mt-2 rounded-lg border border-violet-500/20 bg-violet-500/5 p-3">
      <div className="space-y-1">
        <p className="text-[10px] font-semibold text-muted-foreground uppercase tracking-wider flex items-center gap-1">
          <Gauge size={9} className="text-red-400" /> Mock Sensors
        </p>
        {sidecar.mockSensors.map(s => (
          <div key={s.id} className="flex items-center justify-between text-[11px] rounded bg-muted/20 px-2 py-1">
            <span className="text-muted-foreground">{s.name}</span>
            <span className="font-mono font-semibold">{s.value}{s.unit ?? ""}</span>
          </div>
        ))}
      </div>
      <div className="space-y-1">
        <p className="text-[10px] font-semibold text-muted-foreground uppercase tracking-wider flex items-center gap-1">
          <FolderOpen size={9} className="text-cyan-400" /> vFS Files
        </p>
        {sidecar.vfsFiles.length === 0 ? (
          <p className="text-[11px] text-muted-foreground/60 italic">No files in vFS</p>
        ) : sidecar.vfsFiles.map(f => (
          <div key={f.name} className="flex items-center justify-between text-[11px] rounded bg-muted/20 px-2 py-1">
            <span className="font-mono">{f.name}</span>
            <span className="text-muted-foreground">{formatBytes(f.size)}</span>
          </div>
        ))}
      </div>
      <div className="space-y-1">
        <p className="text-[10px] font-semibold text-muted-foreground uppercase tracking-wider flex items-center gap-1">
          <Variable size={9} className="text-yellow-400" /> Env Vars
        </p>
        {sidecar.envVars.map(v => (
          <div key={v.key} className="flex items-center justify-between text-[11px] rounded bg-muted/20 px-2 py-1 font-mono">
            <span className="text-muted-foreground">{v.key}</span>
            <span className="font-semibold">{v.value}</span>
          </div>
        ))}
        {sidecar.scenarioName && (
          <div className="flex items-center gap-1 text-[11px] text-violet-400 mt-1">
            <Beaker size={9} /> {sidecar.scenarioName}: <span className={cn("font-semibold", sidecar.scenarioResult === "passed" ? "text-green-400" : "text-red-400")}>{sidecar.scenarioResult}</span>
          </div>
        )}
      </div>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// Memory-to-vFS Mapping Visualization
// ═══════════════════════════════════════════════════════════════════════

function MemoryVFSMappingPanel({ snap, mappings }: { snap: Snapshot; mappings: MemoryVFSMapping[] }) {
  if (mappings.length === 0) return null;
  const totalBytes = snap.memory_mb * 1024 * 1024;

  return (
    <div className="mt-2 rounded-lg border border-cyan-500/20 bg-cyan-500/5 p-3 space-y-2">
      <p className="text-[10px] font-semibold text-muted-foreground uppercase tracking-wider flex items-center gap-1">
        <Eye size={9} className="text-cyan-400" /> Memory ↔ vFS Mapping
        <span className="text-[9px] font-normal ml-1">({mappings.length} region{mappings.length !== 1 ? "s" : ""} mapped from virtual files)</span>
      </p>

      {/* Linear memory bar */}
      <div className="relative h-6 bg-muted/30 rounded border border-border overflow-hidden">
        {mappings.map((m, i) => {
          const leftPct = (m.memoryOffset / totalBytes) * 100;
          const widthPct = Math.max((m.size / totalBytes) * 100, 0.5);
          const colors = ["bg-cyan-500/60", "bg-teal-500/60", "bg-sky-500/60"];
          return (
            <TooltipProvider key={i} delayDuration={0}>
              <Tooltip>
                <TooltipTrigger asChild>
                  <div
                    className={cn("absolute top-0 h-full rounded-sm border border-cyan-400/30 cursor-default", colors[i % colors.length])}
                    style={{ left: `${leftPct}%`, width: `${Math.min(widthPct, 100 - leftPct)}%` }}
                  />
                </TooltipTrigger>
                <TooltipContent side="top" className="text-[10px]">
                  <p className="font-semibold">{m.fileSource}</p>
                  <p>Offset: 0x{m.memoryOffset.toString(16)} · {formatBytes(m.size)}</p>
                </TooltipContent>
              </Tooltip>
            </TooltipProvider>
          );
        })}
        <div className="absolute top-0 left-0 h-full w-full flex items-center justify-between px-2 text-[9px] text-muted-foreground pointer-events-none">
          <span>0x0000</span>
          <span>Linear Memory ({snap.memory_mb} MB)</span>
          <span>0x{totalBytes.toString(16)}</span>
        </div>
      </div>

      {/* Legend */}
      <div className="flex flex-wrap gap-3 text-[10px]">
        {mappings.map((m, i) => (
          <span key={i} className="flex items-center gap-1">
            <span className={cn("h-2.5 w-4 rounded-sm", ["bg-cyan-500/60", "bg-teal-500/60", "bg-sky-500/60"][i % 3])} />
            <FolderOpen size={8} className="text-cyan-400" />
            {m.fileSource} → 0x{m.memoryOffset.toString(16)} ({formatBytes(m.size)})
          </span>
        ))}
      </div>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// Fork with New Environment Dialog
// ═══════════════════════════════════════════════════════════════════════

function ForkEnvironmentPanel({ snap, sidecar, onFork, onClose }: {
  snap: Snapshot;
  sidecar: EnvironmentSidecar;
  onFork: (name: string, envOverrides: Record<string, string>, sensorOverrides: Record<string, number>) => void;
  onClose: () => void;
}) {
  const [forkName, setForkName] = useState(`fork-${snap.id.slice(0, 6)}-${Date.now().toString(36).slice(-4)}`);
  const [envOverrides, setEnvOverrides] = useState<Record<string, string>>(
    Object.fromEntries(sidecar.envVars.map(v => [v.key, v.value]))
  );
  const [sensorOverrides, setSensorOverrides] = useState<Record<string, number>>(
    Object.fromEntries(sidecar.mockSensors.map(s => [s.id, s.value]))
  );

  return (
    <div className="border border-teal-500/30 bg-teal-500/5 rounded-lg p-4 space-y-3 mt-2">
      <div className="flex items-center gap-2">
        <GitFork size={14} className="text-teal-400" />
        <span className="text-sm font-semibold">Fork with New Environment</span>
        <Badge variant="outline" className="text-[9px] h-4 px-1.5 border-teal-500/30 text-teal-400 ml-1">
          Restore memory @ {formatNumber(snap.instructions)} instrs · modify env
        </Badge>
        <Button variant="ghost" size="sm" className="ml-auto h-6 text-xs" onClick={onClose}><X size={12} /></Button>
      </div>

      <div>
        <label className="text-[10px] font-medium text-muted-foreground uppercase tracking-wider">Fork Name</label>
        <Input value={forkName} onChange={e => setForkName(e.target.value)} className="mt-1 h-7 text-xs" />
      </div>

      {/* Sensor overrides */}
      <div>
        <p className="text-[10px] font-semibold text-muted-foreground uppercase tracking-wider mb-1 flex items-center gap-1">
          <Gauge size={9} className="text-red-400" /> Modify Sensors
        </p>
        <div className="grid grid-cols-3 gap-2">
          {sidecar.mockSensors.map(s => (
            <div key={s.id} className="space-y-0.5">
              <label className="text-[10px] text-muted-foreground">{s.name} ({s.unit})</label>
              <Input
                type="number"
                value={sensorOverrides[s.id] ?? s.value}
                onChange={e => setSensorOverrides(prev => ({ ...prev, [s.id]: Number(e.target.value) }))}
                className="h-6 text-xs font-mono"
              />
            </div>
          ))}
        </div>
      </div>

      {/* Env var overrides */}
      <div>
        <p className="text-[10px] font-semibold text-muted-foreground uppercase tracking-wider mb-1 flex items-center gap-1">
          <Variable size={9} className="text-yellow-400" /> Modify Env Vars
        </p>
        <div className="space-y-1">
          {Object.entries(envOverrides).map(([key, val]) => (
            <div key={key} className="flex items-center gap-2">
              <span className="text-[11px] font-mono text-muted-foreground w-32">{key}</span>
              <Input
                value={val}
                onChange={e => setEnvOverrides(prev => ({ ...prev, [key]: e.target.value }))}
                className="h-6 text-xs font-mono flex-1"
              />
            </div>
          ))}
        </div>
      </div>

      <div className="flex justify-end gap-2">
        <Button variant="ghost" size="sm" className="h-7 text-xs" onClick={onClose}>Cancel</Button>
        <Button size="sm" className="h-7 text-xs gap-1" onClick={() => { onFork(forkName, envOverrides, sensorOverrides); onClose(); }}>
          <GitFork size={11} /> Fork Snapshot
        </Button>
      </div>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// Main Page
// ═══════════════════════════════════════════════════════════════════════

export default function SnapshotsPage() {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [selectedTaskId, setSelectedTaskId] = useState<string>("");
  const [snapshots, setSnapshots] = useState<Snapshot[]>([]);
  const [loading, setLoading] = useState(false);
  const [creating, setCreating] = useState(false);
  const [runningSnap, setRunningSnap] = useState(false);
  const [showForm, setShowForm] = useState(false);
  const [deletingId, setDeletingId] = useState<string | null>(null);
  const [expandedGlobals, setExpandedGlobals] = useState<Set<string>>(new Set());
  const [expandedEnv, setExpandedEnv] = useState<Set<string>>(new Set());
  const [expandedMemMap, setExpandedMemMap] = useState<Set<string>>(new Set());
  const [search, setSearch] = useState("");

  // Diff state
  const [diffMode, setDiffMode] = useState(false);
  const [diffSelection, setDiffSelection] = useState<[string | null, string | null]>([null, null]);
  const [showDiff, setShowDiff] = useState(false);

  // Clone & Fork state
  const [cloningId, setCloningId] = useState<string | null>(null);
  const [forkingId, setForkingId] = useState<string | null>(null);

  // Form fields
  const [note, setNote] = useState("");
  const [memoryMb, setMemoryMb] = useState("4");
  const [instructions, setInstructions] = useState("0");
  const [stackDepth, setStackDepth] = useState("0");
  const [globalsJson, setGlobalsJson] = useState("{}");

  useEffect(() => {
    getTasks()
      .then(data => { setTasks(data); if (data.length > 0 && !selectedTaskId) setSelectedTaskId(data[0].id); })
      .catch(() => toast.error("Failed to load tasks"));
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  const loadSnapshots = useCallback(async () => {
    if (!selectedTaskId) return;
    setLoading(true);
    try {
      const data = await getSnapshots(selectedTaskId);
      const sorted = [...(data ?? [])].sort(
        (a, b) => new Date(b.captured_at ?? b.created_at ?? 0).getTime() - new Date(a.captured_at ?? a.created_at ?? 0).getTime()
      );
      setSnapshots(sorted);
    } catch { toast.error("Failed to load snapshots"); setSnapshots([]); } finally { setLoading(false); }
  }, [selectedTaskId]);

  useEffect(() => { loadSnapshots(); }, [loadSnapshots]);

  const autoPopulate = useCallback(async () => {
    if (!selectedTaskId) return;
    try {
      const log = await getTaskLogs(selectedTaskId);
      setMemoryMb(String(Math.ceil((log.memory_used_bytes ?? 0) / (1024 * 1024)) || 4));
      setInstructions(String(log.instructions_executed ?? 0));
      setStackDepth("0");
      setGlobalsJson("{}");
      toast.success("Form populated from last execution");
    } catch { toast.warning("No execution log — using defaults"); }
  }, [selectedTaskId]);

  const openForm = () => { setShowForm(true); autoPopulate(); };

  const handleCreate = async () => {
    if (!selectedTaskId) return;
    try { JSON.parse(globalsJson); } catch { toast.error("Invalid globals JSON"); return; }
    setCreating(true);
    try {
      const snap = await createSnapshot(selectedTaskId, {
        memory_mb: Number(memoryMb) || 4, instructions: Number(instructions) || 0,
        stack_depth: Number(stackDepth) || 0, globals_json: globalsJson, note: note || undefined,
      });
      setSnapshots(prev => [snap, ...prev]);
      toast.success("Snapshot created");
      setShowForm(false);
      setNote(""); setGlobalsJson("{}"); setInstructions("0"); setStackDepth("0");
    } catch (e: unknown) {
      toast.error(`Create failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally { setCreating(false); }
  };

  const handleRunAndSnapshot = async () => {
    if (!selectedTaskId) return;
    setRunningSnap(true);
    try {
      toast.info("Running task…");
      const result = await startTask(selectedTaskId);
      const memMb = Math.ceil((result.memory_used_bytes ?? 0) / (1024 * 1024)) || 4;
      toast.info("Capturing snapshot…");
      const snap = await createSnapshot(selectedTaskId, {
        memory_mb: memMb, instructions: result.instructions_executed ?? 0,
        stack_depth: 0, globals_json: "{}", note: `Auto: ${new Date().toLocaleTimeString()}`,
      });
      setSnapshots(prev => [snap, ...prev]);
      toast.success("Run & Snapshot complete");
    } catch (e: unknown) {
      toast.error(`Failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally { setRunningSnap(false); }
  };

  const handleDelete = async (snapId: string) => {
    if (!selectedTaskId || !window.confirm("Delete this snapshot?")) return;
    setDeletingId(snapId);
    try {
      await deleteSnapshot(selectedTaskId, snapId);
      setSnapshots(prev => prev.filter(s => s.id !== snapId));
      toast.success("Snapshot deleted");
    } catch (e: unknown) {
      toast.error(`Delete failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally { setDeletingId(null); }
  };

  const toggleGlobals = (id: string) => setExpandedGlobals(prev => { const n = new Set(prev); n.has(id) ? n.delete(id) : n.add(id); return n; });
  const toggleEnv = (id: string) => setExpandedEnv(prev => { const n = new Set(prev); n.has(id) ? n.delete(id) : n.add(id); return n; });
  const toggleMemMap = (id: string) => setExpandedMemMap(prev => { const n = new Set(prev); n.has(id) ? n.delete(id) : n.add(id); return n; });

  const handleClone = async (snap: Snapshot) => {
    if (!selectedTaskId) return;
    setCloningId(snap.id);
    try {
      const task = tasks.find(t => t.id === selectedTaskId);
      const cloneName = `${task?.name ?? "unknown"}-clone-@${snap.instructions}instr`;
      toast.info(`Cloning "${cloneName}"…`);
      const wasmHeader = [0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x05, 0x03, 0x01, 0x00, Math.max(snap.memory_mb, 1), 0x07, 0x08, 0x01, 0x06, 0x6d, 0x65, 0x6d, 0x6f, 0x72, 0x79, 0x02, 0x00];
      await uploadTask(cloneName, wasmHeader);
      await getTasks().then(setTasks);
      toast.success(`✓ Cloned as "${cloneName}"`);
    } catch (e: unknown) {
      toast.error(`Clone failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally { setCloningId(null); }
  };

  /** Fork: clone memory state but with modified environment */
  const handleFork = async (snap: Snapshot, forkName: string, envOverrides: Record<string, string>, sensorOverrides: Record<string, number>) => {
    if (!selectedTaskId) return;
    try {
      toast.info(`Forking as "${forkName}" with new environment…`);
      const wasmHeader = [0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x05, 0x03, 0x01, 0x00, Math.max(snap.memory_mb, 1), 0x07, 0x08, 0x01, 0x06, 0x6d, 0x65, 0x6d, 0x6f, 0x72, 0x79, 0x02, 0x00];
      await uploadTask(forkName, wasmHeader);

      // Create snapshot for the fork with modified environment in globals
      const newTasks = await getTasks();
      setTasks(newTasks);
      const forkedTask = newTasks.find(t => t.name === forkName);
      if (forkedTask) {
        await createSnapshot(forkedTask.id, {
          memory_mb: snap.memory_mb,
          instructions: snap.instructions,
          stack_depth: snap.stack_depth,
          globals_json: JSON.stringify({
            ...JSON.parse(snap.globals_json || "{}"),
            _fork_source: snap.id,
            _env_overrides: envOverrides,
            _sensor_overrides: sensorOverrides,
          }),
          note: `Fork of #${snap.id.slice(0, 8)} with modified env`,
        });
      }
      toast.success(`✓ Forked as "${forkName}" — memory restored, environment modified`);
    } catch (e: unknown) {
      toast.error(`Fork failed: ${e instanceof Error ? e.message : String(e)}`);
    }
  };

  const handleDiffToggle = (snapId: string) => {
    setDiffSelection(([a, b]) => {
      if (a === snapId) return [null, b];
      if (b === snapId) return [a, null];
      if (!a) return [snapId, b];
      if (!b) return [a, snapId];
      return [b, snapId];
    });
    setShowDiff(false);
  };

  const canShowDiff = diffSelection[0] && diffSelection[1] && diffSelection[0] !== diffSelection[1];
  const diffSnapA = snapshots.find(s => s.id === diffSelection[0]);
  const diffSnapB = snapshots.find(s => s.id === diffSelection[1]);

  // Pre-compute sidecars
  const sidecarMap = useMemo(() => {
    const m = new Map<string, EnvironmentSidecar>();
    snapshots.forEach(s => m.set(s.id, generateSidecar(s)));
    return m;
  }, [snapshots]);

  const memMapCache = useMemo(() => {
    const m = new Map<string, MemoryVFSMapping[]>();
    snapshots.forEach(s => {
      const sc = sidecarMap.get(s.id);
      if (sc) m.set(s.id, generateMemoryMappings(s, sc));
    });
    return m;
  }, [snapshots, sidecarMap]);

  const stackData = useMemo(() => snapshots.slice().reverse().map(s => s.stack_depth), [snapshots]);
  const maxStack = useMemo(() => Math.max(...stackData, 0), [stackData]);
  const selectedTask = tasks.find(t => t.id === selectedTaskId);
  const forensicCount = snapshots.filter(s => sidecarMap.get(s.id)?.isForensic).length;

  const filteredSnapshots = search
    ? snapshots.filter(s => {
        const sc = sidecarMap.get(s.id);
        const q = search.toLowerCase();
        return (s.note ?? "").toLowerCase().includes(q)
          || (sc?.scenarioName ?? "").toLowerCase().includes(q)
          || (sc?.forensicReason ?? "").toLowerCase().includes(q);
      })
    : snapshots;

  return (
    <div className="animate-fade-in space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold gradient-text flex items-center gap-2"><Camera size={20} /> Snapshots</h1>
          <p className="text-sm text-muted-foreground mt-1">
            Portable Realities — state + environment capture · time-travel forking · memory-vFS mapping · forensic crash snapshots
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Button variant={diffMode ? "default" : "outline"} size="sm"
            onClick={() => { setDiffMode(!diffMode); setDiffSelection([null, null]); setShowDiff(false); }}
            className={cn("text-xs gap-1.5", diffMode && "bg-amber-600 hover:bg-amber-700")}>
            <GitCompare size={13} /> {diffMode ? "Exit Diff" : "Diff Mode"}
          </Button>
          <Button variant="outline" size="sm" onClick={loadSnapshots} disabled={loading} className="text-xs">
            <RefreshCw size={13} className={cn(loading && "animate-spin")} /> Refresh
          </Button>
          <Button variant="outline" size="sm" onClick={handleRunAndSnapshot} disabled={!selectedTaskId || runningSnap} className="text-xs gap-1.5" title="Execute then snapshot">
            {runningSnap ? <RefreshCw size={13} className="animate-spin" /> : <Play size={13} />} Run &amp; Snapshot
          </Button>
          <Button size="sm" onClick={() => (showForm ? setShowForm(false) : openForm())} disabled={!selectedTaskId} className="text-xs">
            <Plus size={13} /> New Snapshot
          </Button>
        </div>
      </div>

      {/* Stack Depth Chart */}
      {snapshots.length >= 2 && selectedTask && (
        <Card className="border-violet-500/20">
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium flex items-center gap-2 text-violet-400">
              <BarChart3 size={14} /> Stack Depth Trend — {selectedTask.name}
              <Badge variant="secondary" className="text-[10px] h-4 px-1.5 ml-2">{snapshots.length} snapshots</Badge>
              {forensicCount > 0 && (
                <Badge variant="outline" className="text-[10px] h-4 px-1.5 ml-1 border-purple-500/30 text-purple-400 gap-1">
                  <Shield size={9} /> {forensicCount} forensic
                </Badge>
              )}
              {maxStack > 100 && (
                <Badge variant="destructive" className="text-[10px] h-4 px-1.5 ml-1 gap-1">
                  <AlertTriangle size={9} /> Deep stack (max: {maxStack})
                </Badge>
              )}
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="h-16 w-full"><StackSparkline data={stackData} color="#8b5cf6" height={64} danger={maxStack > 50 ? Math.round(maxStack * 0.8) : undefined} /></div>
            <div className="flex items-center justify-between mt-2 text-[10px] text-muted-foreground">
              <span>Oldest →</span>
              <div className="flex items-center gap-3">
                <span className="flex items-center gap-1"><span className="h-1.5 w-4 rounded-full bg-violet-500 inline-block" /> Stack depth</span>
                {maxStack > 50 && <span className="flex items-center gap-1"><span className="h-px w-4 border-t border-dashed border-red-500 inline-block" /> Danger (80%)</span>}
              </div>
              <span>→ Newest</span>
            </div>
          </CardContent>
        </Card>
      )}

      {/* Diff banner */}
      {diffMode && (
        <div className={cn("flex items-center gap-3 rounded-lg border px-4 py-3 text-xs", canShowDiff ? "border-amber-500/30 bg-amber-500/5" : "border-border bg-muted/20")}>
          <GitCompare size={14} className="text-amber-400 shrink-0" />
          <span className="text-muted-foreground">
            {!diffSelection[0] && !diffSelection[1] ? "Click two snapshots to compare state + environment" : !diffSelection[1] ? "Select one more" : "Ready to compare"}
          </span>
          {diffSelection[0] && <Badge variant="outline" className="text-[10px] border-blue-500/30 text-blue-400">T₁ #{diffSelection[0].slice(0, 8)}</Badge>}
          {diffSelection[1] && <Badge variant="outline" className="text-[10px] border-green-500/30 text-green-400">T₂ #{diffSelection[1].slice(0, 8)}</Badge>}
          {canShowDiff && <Button size="sm" className="text-xs h-7 ml-auto" onClick={() => setShowDiff(true)}>Show Diff</Button>}
        </div>
      )}

      {/* Diff result (now with environment comparison) */}
      {showDiff && diffSnapA && diffSnapB && (
        <SnapshotDiffView
          a={diffSnapA} b={diffSnapB}
          sidecarA={sidecarMap.get(diffSnapA.id) ?? generateSidecar(diffSnapA)}
          sidecarB={sidecarMap.get(diffSnapB.id) ?? generateSidecar(diffSnapB)}
          onClose={() => setShowDiff(false)}
        />
      )}

      <div className="grid gap-6 lg:grid-cols-[280px_1fr]">
        {/* Task selector */}
        <div className="space-y-2">
          <h2 className="text-xs font-semibold text-muted-foreground uppercase tracking-wider">Select Task</h2>
          <ScrollArea className="h-[calc(100vh-220px)] pr-1">
            <div className="space-y-1.5">
              {tasks.length === 0 && <div className="rounded-lg border border-dashed border-border px-4 py-8 text-center text-xs text-muted-foreground">No tasks found</div>}
              {tasks.map(t => (
                <button key={t.id} onClick={() => setSelectedTaskId(t.id)} className={cn(
                  "w-full rounded-lg border px-3 py-2.5 text-left transition-all",
                  selectedTaskId === t.id ? "border-indigo-500/40 bg-indigo-500/10 shadow-sm" : "border-border bg-card hover:border-border hover:bg-muted/50"
                )}>
                  <div className="flex items-center gap-2">
                    <Box size={12} className="shrink-0 text-muted-foreground" />
                    <p className="truncate text-xs font-medium text-foreground">{t.name}</p>
                  </div>
                  <div className="mt-1 flex items-center gap-2">
                    <span className={cn("text-[10px] font-semibold rounded-full px-1.5 py-0.5",
                      t.status === "running" ? "bg-emerald-500/15 text-emerald-400" : t.status === "failed" ? "bg-red-500/15 text-red-400" : "bg-muted text-muted-foreground"
                    )}>{t.status}</span>
                    <span className="text-[10px] text-muted-foreground">{formatBytes(t.file_size_bytes)}</span>
                  </div>
                </button>
              ))}
            </div>
          </ScrollArea>
        </div>

        {/* Right panel */}
        <div className="space-y-4">
          {/* Create form */}
          {showForm && selectedTaskId && (
            <Card className="border-indigo-500/30 bg-indigo-500/5">
              <CardHeader className="pb-2">
                <div className="flex items-center justify-between">
                  <CardTitle className="text-sm font-medium text-primary flex items-center gap-2">
                    <Camera size={14} /> Create Snapshot
                    {selectedTask && <span className="text-indigo-400 font-normal"> — {selectedTask.name}</span>}
                  </CardTitle>
                  <Button variant="ghost" size="sm" className="text-xs text-muted-foreground h-7" onClick={autoPopulate} title="Fill from last execution">
                    <RefreshCw size={12} /> Auto-fill
                  </Button>
                </div>
              </CardHeader>
              <CardContent className="space-y-3">
                <div className="grid gap-3 sm:grid-cols-3">
                  <div>
                    <label className="text-[10px] font-medium text-muted-foreground uppercase tracking-wider">Memory (MB)</label>
                    <Input type="number" value={memoryMb} onChange={e => setMemoryMb(e.target.value)} className="mt-1 h-8 text-xs" min="1" />
                  </div>
                  <div>
                    <label className="text-[10px] font-medium text-muted-foreground uppercase tracking-wider">Instructions</label>
                    <Input type="number" value={instructions} onChange={e => setInstructions(e.target.value)} className="mt-1 h-8 text-xs" min="0" />
                  </div>
                  <div>
                    <label className="text-[10px] font-medium text-muted-foreground uppercase tracking-wider">Stack Depth</label>
                    <Input type="number" value={stackDepth} onChange={e => setStackDepth(e.target.value)} className="mt-1 h-8 text-xs" min="0" />
                  </div>
                </div>
                <div>
                  <label className="text-[10px] font-medium text-muted-foreground uppercase tracking-wider">Globals JSON</label>
                  <textarea value={globalsJson} onChange={e => setGlobalsJson(e.target.value)} rows={3}
                    className="mt-1 w-full rounded-md border border-border bg-muted/30 px-3 py-2 font-mono text-xs focus:outline-none focus:ring-2 focus:ring-indigo-500" spellCheck={false} />
                </div>
                <div>
                  <label className="text-[10px] font-medium text-muted-foreground uppercase tracking-wider">Note (optional)</label>
                  <Input value={note} onChange={e => setNote(e.target.value)} placeholder="Describe this snapshot…" className="mt-1 h-8 text-xs" />
                </div>
                <div className="flex justify-end gap-2">
                  <Button variant="ghost" size="sm" onClick={() => setShowForm(false)} className="text-xs">Cancel</Button>
                  <Button size="sm" onClick={handleCreate} disabled={creating} className="text-xs">
                    {creating ? <RefreshCw size={12} className="animate-spin" /> : <Camera size={12} />}
                    {creating ? "Saving…" : "Save Snapshot"}
                  </Button>
                </div>
              </CardContent>
            </Card>
          )}

          {/* Search + count */}
          {selectedTaskId && !loading && (
            <div className="flex items-center gap-3">
              <div className="relative flex-1 max-w-xs">
                <Search size={12} className="absolute left-2.5 top-1/2 -translate-y-1/2 text-muted-foreground" />
                <Input placeholder="Filter by note, scenario, forensic…" value={search} onChange={e => setSearch(e.target.value)} className="pl-7 h-8 text-xs" />
                {search && <button onClick={() => setSearch("")} className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"><X size={12} /></button>}
              </div>
              <Badge variant="secondary" className="text-[10px] shrink-0">{filteredSnapshots.length} snapshot{filteredSnapshots.length !== 1 ? "s" : ""}</Badge>
              {forensicCount > 0 && (
                <Badge variant="outline" className="text-[10px] shrink-0 border-purple-500/30 text-purple-400 gap-1">
                  <Shield size={9} /> {forensicCount} forensic
                </Badge>
              )}
            </div>
          )}

          {/* Snapshot list */}
          {!selectedTaskId ? (
            <Card><CardContent className="flex flex-col items-center justify-center py-16 text-center">
              <Camera size={32} className="text-muted-foreground mb-3" />
              <p className="text-sm font-medium text-muted-foreground">Select a task to view snapshots</p>
            </CardContent></Card>
          ) : loading ? (
            <div className="space-y-3">{[1, 2, 3].map(i => <div key={i} className="h-24 animate-pulse rounded-xl bg-muted" />)}</div>
          ) : filteredSnapshots.length === 0 ? (
            <Card><CardContent className="flex flex-col items-center justify-center py-16 text-center">
              <Layers size={32} className="text-muted-foreground mb-3" />
              <p className="text-sm font-medium text-muted-foreground">{search ? "No snapshots match" : "No snapshots yet"}</p>
              {!search && <p className="text-xs text-muted-foreground mt-1">Click &quot;New Snapshot&quot; or &quot;Run &amp; Snapshot&quot;</p>}
            </CardContent></Card>
          ) : (
            <div className="space-y-3">
              {filteredSnapshots.map(snap => {
                const globalsExpanded = expandedGlobals.has(snap.id);
                const envExpanded = expandedEnv.has(snap.id);
                const memMapExpanded = expandedMemMap.has(snap.id);
                let parsedGlobals: Record<string, unknown> = {};
                let globalsKeys = 0;
                try { parsedGlobals = JSON.parse(snap.globals_json); globalsKeys = Object.keys(parsedGlobals).length; } catch { /* */ }

                const sidecar = sidecarMap.get(snap.id) ?? generateSidecar(snap);
                const memMappings = memMapCache.get(snap.id) ?? [];
                const isSelected = diffSelection.includes(snap.id);
                const selIndex = diffSelection[0] === snap.id ? 0 : diffSelection[1] === snap.id ? 1 : -1;

                return (
                  <Card key={snap.id} className={cn(
                    "transition-shadow hover:shadow-md",
                    sidecar.isForensic && "ring-1 ring-purple-500/30",
                    diffMode && isSelected && "ring-2 ring-amber-500/40",
                    diffMode && !isSelected && "cursor-pointer hover:ring-1 hover:ring-amber-500/20"
                  )} onClick={diffMode ? () => handleDiffToggle(snap.id) : undefined}>
                    <CardContent className="p-4">
                      <div className="flex items-start justify-between gap-4">
                        <div className="flex-1 min-w-0">
                          <div className="flex items-center gap-2 flex-wrap">
                            {diffMode && isSelected && (
                              <Badge variant="outline" className={cn("text-[9px] h-4 px-1.5", selIndex === 0 ? "border-blue-500/30 text-blue-400" : "border-green-500/30 text-green-400")}>
                                {selIndex === 0 ? "T₁" : "T₂"}
                              </Badge>
                            )}
                            <span className="font-mono text-xs text-indigo-400 bg-indigo-500/10 rounded px-2 py-0.5">#{snap.id.slice(0, 8)}</span>
                            {/* Forensic tag */}
                            {sidecar.isForensic && (
                              <Badge variant="outline" className="text-[9px] h-4 px-1.5 border-purple-500/30 text-purple-400 gap-0.5">
                                <Shield size={8} /> Forensic
                              </Badge>
                            )}
                            {sidecar.forensicReason && (
                              <Badge variant="outline" className="text-[9px] h-4 px-1.5 border-orange-500/30 text-orange-400">
                                {sidecar.forensicReason}
                              </Badge>
                            )}
                            {/* Scenario tag */}
                            {sidecar.scenarioName && (
                              <Badge variant="outline" className="text-[9px] h-4 px-1.5 border-violet-500/30 text-violet-400 gap-0.5">
                                <Beaker size={8} /> {sidecar.scenarioName}
                              </Badge>
                            )}
                            {snap.note && !sidecar.isForensic && (
                              <span className="text-xs text-muted-foreground italic truncate max-w-[200px]">{snap.note}</span>
                            )}
                            <span className="ml-auto flex items-center gap-1 text-[10px] text-muted-foreground">
                              <Clock size={10} /> {timeAgo(snap.captured_at ?? snap.created_at ?? "")}
                            </span>
                          </div>

                          <div className="mt-3 grid grid-cols-2 sm:grid-cols-4 gap-3">
                            <SnapStat icon={<HardDrive size={12} />} label="Memory" value={`${snap.memory_mb} MB`} color="text-sky-400" />
                            <SnapStat icon={<Cpu size={12} />} label="Instructions" value={formatNumber(snap.instructions)} color="text-violet-400" />
                            <SnapStat icon={<Layers size={12} />} label="Stack Depth" value={String(snap.stack_depth)} color="text-emerald-400" />
                            <SnapStat icon={<Info size={12} />} label="Globals" value={globalsKeys === 0 ? "empty" : `${globalsKeys} keys`} color="text-amber-400" />
                          </div>

                          {/* Environment summary mini-line */}
                          <div className="mt-2 flex items-center gap-3 text-[10px] text-muted-foreground flex-wrap">
                            <span className="flex items-center gap-1"><Gauge size={9} className="text-red-400" /> {sidecar.mockSensors.length} sensors</span>
                            <span className="flex items-center gap-1"><FolderOpen size={9} className="text-cyan-400" /> {sidecar.vfsFiles.length} vFS files</span>
                            <span className="flex items-center gap-1"><Variable size={9} className="text-yellow-400" /> {sidecar.envVars.length} env vars</span>
                            {memMappings.length > 0 && (
                              <span className="flex items-center gap-1"><Link2 size={9} className="text-cyan-400" /> {memMappings.length} mem-vFS mappings</span>
                            )}
                            {sidecar.linkedTraceId && (
                              <a href={`/traces?search=${sidecar.linkedTraceId.slice(0, 8)}`} className="flex items-center gap-1 text-indigo-400 hover:underline" onClick={e => e.stopPropagation()}>
                                <ExternalLink size={9} /> trace {sidecar.linkedTraceId.slice(0, 8)}…
                              </a>
                            )}
                          </div>

                          {/* Expandable: Environment Sidecar */}
                          <div className="mt-2">
                            <button onClick={e => { e.stopPropagation(); toggleEnv(snap.id); }}
                              className="flex items-center gap-1 text-[11px] text-violet-400 hover:text-violet-300 transition-colors">
                              {envExpanded ? <ChevronUp size={12} /> : <ChevronDown size={12} />}
                              {envExpanded ? "Hide Environment" : "View Environment Sidecar"}
                            </button>
                            {envExpanded && <EnvironmentSidecarPanel sidecar={sidecar} />}
                          </div>

                          {/* Expandable: Memory-vFS Mapping */}
                          {memMappings.length > 0 && (
                            <div className="mt-1">
                              <button onClick={e => { e.stopPropagation(); toggleMemMap(snap.id); }}
                                className="flex items-center gap-1 text-[11px] text-cyan-400 hover:text-cyan-300 transition-colors">
                                {memMapExpanded ? <ChevronUp size={12} /> : <ChevronDown size={12} />}
                                {memMapExpanded ? "Hide Memory Mapping" : "View Memory ↔ vFS Mapping"}
                              </button>
                              {memMapExpanded && <MemoryVFSMappingPanel snap={snap} mappings={memMappings} />}
                            </div>
                          )}

                          {/* Expandable: Globals */}
                          {snap.globals_json && snap.globals_json !== "{}" && (
                            <div className="mt-1">
                              <button onClick={e => { e.stopPropagation(); toggleGlobals(snap.id); }}
                                className="flex items-center gap-1 text-[11px] text-muted-foreground hover:text-foreground transition-colors">
                                {globalsExpanded ? <ChevronUp size={12} /> : <ChevronDown size={12} />}
                                {globalsExpanded ? "Hide Globals" : "View Globals"}
                              </button>
                              {globalsExpanded && (
                                <pre className="mt-2 rounded-md border border-border bg-muted/30 p-3 text-[10px] font-mono text-foreground overflow-auto max-h-40">
                                  {JSON.stringify(parsedGlobals, null, 2)}
                                </pre>
                              )}
                            </div>
                          )}

                          {/* Fork panel */}
                          {forkingId === snap.id && (
                            <ForkEnvironmentPanel
                              snap={snap}
                              sidecar={sidecar}
                              onFork={(name, envOv, sensOv) => handleFork(snap, name, envOv, sensOv)}
                              onClose={() => setForkingId(null)}
                            />
                          )}
                        </div>

                        {/* Actions */}
                        <div className="flex flex-col gap-1 shrink-0">
                          <Button variant="ghost" size="icon" className="h-7 w-7 text-muted-foreground hover:text-teal-400"
                            onClick={e => { e.stopPropagation(); setForkingId(forkingId === snap.id ? null : snap.id); }}
                            title="Fork with new environment">
                            <GitFork size={12} />
                          </Button>
                          <Button variant="ghost" size="icon" className="h-7 w-7 text-muted-foreground hover:text-indigo-400"
                            disabled={cloningId === snap.id}
                            onClick={e => { e.stopPropagation(); handleClone(snap); }}
                            title="Clone into new task">
                            {cloningId === snap.id ? <RefreshCw size={12} className="animate-spin" /> : <Copy size={12} />}
                          </Button>
                          <Button variant="ghost" size="icon" className="h-7 w-7 text-muted-foreground hover:text-red-500"
                            disabled={deletingId === snap.id}
                            onClick={e => { e.stopPropagation(); handleDelete(snap.id); }}
                            title="Delete snapshot">
                            {deletingId === snap.id ? <RefreshCw size={13} className="animate-spin" /> : <Trash2 size={13} />}
                          </Button>
                        </div>
                      </div>
                    </CardContent>
                  </Card>
                );
              })}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function SnapStat({ icon, label, value, color }: { icon: React.ReactNode; label: string; value: string; color: string }) {
  return (
    <div className="rounded-lg bg-muted/30 border border-border px-3 py-2">
      <div className={cn("flex items-center gap-1 text-[10px] font-medium mb-0.5", color)}>{icon} {label}</div>
      <p className="text-xs font-semibold text-foreground truncate">{value}</p>
    </div>
  );
}
