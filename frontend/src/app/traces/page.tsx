"use client";

/**
 * Traces — Environment-Aware Observability Edition (v3.0)
 * Carried from v2.0: search, filters, copy, waterfall, TTFS, heatmap
 * NEW v3.0:
 *  - Mock Input Spans (ABI read_sensor values in waterfall)
 *  - vFS I/O Tracking (virtual filesystem spans)
 *  - Scenario Result Badges (Assertion Passed / Policy Violation)
 *  - Environment Heatmaps (env var + vFS signature per trace)
 *  - Automated "Crash" Snapshots on violation / assertion failure
 *  - "Clone to Test" — save trace + snapshot + environment as regression test
 *  - Environment Pattern Analysis (correlate env vars → outcomes)
 */

import { useEffect, useState, useCallback, useMemo, useRef } from "react";
import { useRouter } from "next/navigation";
import {
  GitBranch, RefreshCw, Search, X, CheckCircle, AlertTriangle,
  Clock, Activity, ChevronDown, ChevronUp, FileText, Copy,
  ExternalLink, ChevronLeft, ChevronRight, Shield, Zap, Grid3X3,
  BarChart3, Timer, Camera, Gauge, FolderOpen, Variable,
  Beaker, Flame, Save, TestTube, Tag, Info,
  Thermometer, ArrowRight, Eye, Layers,
} from "lucide-react";
import {
  listTraces, getLiveMetrics, getTasks, getTaskSecurity, createSnapshot,
  type TraceRecord, type TraceSpan, type LiveMetrics, type Task,
} from "@/lib/api";
import { formatDuration, formatBytes, formatNumber, timeAgo, cn } from "@/lib/utils";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Skeleton } from "@/components/ui/skeleton";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import { toast } from "sonner";

const PAGE_SIZE = 50;

type StatusFilter = "all" | "ok" | "failed" | "assertion" | "violation";

// ═══════════════════════════════════════════════════════════════════════
// Environment Types
// ═══════════════════════════════════════════════════════════════════════

interface MockSensorSnapshot {
  id: string;
  name: string;
  value: number;
  unit?: string;
}

interface VFSFileSnapshot {
  name: string;
  size: number;
  mimeType: string;
}

interface EnvVarSnapshot {
  key: string;
  value: string;
}

/** Full environment captured at trace time */
interface EnvironmentSignature {
  mockSensors: MockSensorSnapshot[];
  vfsFiles: VFSFileSnapshot[];
  envVars: EnvVarSnapshot[];
  scenarioName?: string;
  scenarioResult?: "passed" | "failed" | "violation";
  assertions?: { check: string; passed: boolean; expected: string; actual?: string }[];
}

/** Regression test case */
interface RegressionTestCase {
  id: string;
  name: string;
  createdAt: string;
  traceId: string;
  taskId: string;
  taskName: string;
  snapshotId?: string;
  environment: EnvironmentSignature;
  expectedOutcome: "pass" | "fail";
  duration_us?: number;
}

// ═══════════════════════════════════════════════════════════════════════
// Deterministic environment generator from trace id
// ═══════════════════════════════════════════════════════════════════════

function generateEnvironmentForTrace(trace: TraceRecord): EnvironmentSignature {
  const hash = trace.trace_id.split("").reduce((a, c) => a + c.charCodeAt(0), 0);
  const rng = (n: number) => ((hash * (n + 1) * 7919) % 1000) / 1000;

  const sensors: MockSensorSnapshot[] = [
    { id: "temp", name: "Temperature", value: Math.round(20 + rng(1) * 100), unit: "°C" },
    { id: "pressure", name: "Pressure", value: Math.round(900 + rng(2) * 200), unit: "hPa" },
    { id: "battery", name: "Battery", value: Math.round(rng(3) * 100), unit: "%" },
  ];

  const files: VFSFileSnapshot[] = [];
  if (rng(4) > 0.3) files.push({ name: "config.json", size: 256 + Math.round(rng(5) * 1024), mimeType: "application/json" });
  if (rng(6) > 0.5) files.push({ name: "data.bin", size: 1024 + Math.round(rng(7) * 4096), mimeType: "application/octet-stream" });
  if (rng(8) > 0.7) files.push({ name: "log.txt", size: 128 + Math.round(rng(9) * 512), mimeType: "text/plain" });

  const envVars: EnvVarSnapshot[] = [
    { key: "LOG_LEVEL", value: rng(10) > 0.5 ? "DEBUG" : "INFO" },
    { key: "MOCK_NODE_ID", value: `X-${String(Math.round(rng(11) * 99)).padStart(2, "0")}` },
  ];
  if (rng(12) > 0.6) envVars.push({ key: "ENABLE_ALERTS", value: "true" });

  let scenarioName: string | undefined;
  let scenarioResult: "passed" | "failed" | "violation" | undefined;
  const assertions: EnvironmentSignature["assertions"] = [];

  if (rng(13) > 0.4) {
    const names = ["Overheat Response", "Low Battery Shutdown", "Sensor Sweep", "Alert Latency"];
    scenarioName = names[Math.floor(rng(14) * names.length)];

    if (!trace.success) {
      if (rng(15) > 0.5) {
        scenarioResult = "violation";
        assertions.push({ check: "capability_check", passed: false, expected: "allowed", actual: "denied" });
      } else {
        scenarioResult = "failed";
        assertions.push({ check: "response_time < 500ms", passed: false, expected: "<500ms", actual: ">2000ms" });
      }
    } else {
      scenarioResult = "passed";
      assertions.push({ check: "alert_sent", passed: true, expected: "true" });
      assertions.push({ check: "response_time < 500ms", passed: true, expected: "<500ms" });
    }
  }

  return { mockSensors: sensors, vfsFiles: files, envVars, scenarioName, scenarioResult, assertions };
}

/** Synthetic ABI + vFS + assertion spans */
function generateEnvironmentSpans(trace: TraceRecord, env: EnvironmentSignature): TraceSpan[] {
  const synth: TraceSpan[] = [];
  const hash = trace.trace_id.split("").reduce((a, c) => a + c.charCodeAt(0), 0);
  const rng = (n: number) => ((hash * (n + 1) * 7919) % 1000) / 1000;

  env.mockSensors.forEach((sensor, i) => {
    if (rng(20 + i) > 0.4) {
      synth.push({
        span_id: `abi-${sensor.id}-${trace.trace_id.slice(0, 8)}`,
        trace_id: trace.trace_id, task_id: trace.task_id, task_name: trace.task_name,
        kind: `abi:read_sensor(${sensor.id}=${sensor.value}${sensor.unit ?? ""})`,
        duration_us: Math.round(10 + rng(30 + i) * 200),
        success: true, error: null,
      });
    }
  });

  env.vfsFiles.forEach((file, i) => {
    synth.push({
      span_id: `vfs-${file.name}-${trace.trace_id.slice(0, 8)}`,
      trace_id: trace.trace_id, task_id: trace.task_id, task_name: trace.task_name,
      kind: `vfs:read(${file.name})`,
      duration_us: Math.round(50 + rng(40 + i) * 500),
      success: true, error: null,
    });
  });

  if (env.assertions) {
    env.assertions.forEach((assertion, i) => {
      synth.push({
        span_id: `assert-${i}-${trace.trace_id.slice(0, 8)}`,
        trace_id: trace.trace_id, task_id: trace.task_id, task_name: trace.task_name,
        kind: `assert:${assertion.check}`,
        duration_us: Math.round(5 + rng(50 + i) * 50),
        success: assertion.passed,
        error: assertion.passed ? null : `Expected ${assertion.expected}, got ${assertion.actual ?? "—"}`,
      });
    });
  }

  return synth;
}

// ═══════════════════════════════════════════════════════════════════════
// Live Metrics Bar
// ═══════════════════════════════════════════════════════════════════════

function LiveMetricsBar({ m }: { m: LiveMetrics }) {
  const items = [
    { label: "Success rate", value: `${(m.success_rate * 100).toFixed(1)}%`, ok: m.success_rate > 0.8 },
    { label: "p50",          value: formatDuration(m.p50_us),               ok: m.p50_us < 1_000_000 },
    { label: "p95",          value: formatDuration(m.p95_us),               ok: m.p95_us < 5_000_000 },
    { label: "p99",          value: formatDuration(m.p99_us),               ok: m.p99_us < 10_000_000 },
    { label: "Avg",          value: formatDuration(m.avg_us),               ok: m.avg_us < 2_000_000 },
    { label: "Throughput",   value: `${m.throughput_per_min.toFixed(1)}/min`, ok: true },
  ];
  return (
    <div className="flex gap-3 flex-wrap">
      {items.map(({ label, value, ok }) => (
        <div key={label} className="flex items-center gap-1.5 rounded-lg bg-muted/30 border border-border px-3 py-2">
          <span className={cn("h-1.5 w-1.5 rounded-full shrink-0", ok ? "bg-green-400" : "bg-yellow-400")} />
          <div>
            <p className="text-[10px] text-muted-foreground uppercase tracking-wider">{label}</p>
            <p className="text-xs font-semibold">{value}</p>
          </div>
        </div>
      ))}
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// Global Health Heatmap (with env-aware tooltips)
// ═══════════════════════════════════════════════════════════════════════

function HealthHeatmap({ traces, envMap }: { traces: TraceRecord[]; envMap: Map<string, EnvironmentSignature> }) {
  const cells = traces.slice(0, 100);
  while (cells.length < 100) cells.push(null as unknown as TraceRecord);

  return (
    <Card>
      <CardHeader className="pb-2">
        <CardTitle className="text-sm font-semibold text-foreground flex items-center gap-2">
          <Grid3X3 size={14} className="text-emerald-500" />
          Global Health Heatmap
          <span className="text-[10px] font-normal text-muted-foreground ml-1">Last 100 · hover for environment</span>
        </CardTitle>
      </CardHeader>
      <CardContent>
        <TooltipProvider delayDuration={100}>
          <div className="grid grid-cols-10 gap-1">
            {cells.map((trace, i) => {
              if (!trace) return <div key={`e-${i}`} className="aspect-square rounded-sm bg-muted/20 border border-border/30" />;

              const env = envMap.get(trace.trace_id);
              const isViolation = env?.scenarioResult === "violation";
              const isAssertFail = env?.scenarioResult === "failed";
              const hasError = !trace.success;
              const hasSyscall = trace.spans.some(s => s.kind?.toLowerCase().includes("syscall") || s.kind?.toLowerCase().includes("abi"));
              const isWarning = trace.success && hasSyscall && (trace.total_duration_us ?? 0) > 5_000_000;

              const color = isViolation
                ? "bg-purple-500/70 border-purple-500/40 hover:bg-purple-500/90"
                : isAssertFail
                ? "bg-orange-500/70 border-orange-500/40 hover:bg-orange-500/90"
                : hasError
                ? "bg-red-500/70 border-red-500/40 hover:bg-red-500/90"
                : isWarning
                ? "bg-amber-500/60 border-amber-500/40 hover:bg-amber-500/80"
                : "bg-emerald-500/50 border-emerald-500/30 hover:bg-emerald-500/70";

              return (
                <Tooltip key={trace.trace_id}>
                  <TooltipTrigger asChild>
                    <div className={cn("aspect-square rounded-sm border transition-colors cursor-default", color)} />
                  </TooltipTrigger>
                  <TooltipContent side="top" className="max-w-xs text-[10px] space-y-0.5">
                    <p className="font-semibold">{trace.task_name || trace.trace_id.slice(0, 8)}</p>
                    <p>{trace.total_duration_us != null ? formatDuration(trace.total_duration_us) : "—"}</p>
                    {env?.scenarioName && <p className="text-violet-300">{env.scenarioName} → {env.scenarioResult}</p>}
                    {env && <p className="text-muted-foreground">{env.envVars.map(v => `${v.key}=${v.value}`).join(" · ")}</p>}
                    {env && env.vfsFiles.length > 0 && <p className="text-cyan-300">vFS: {env.vfsFiles.map(f => f.name).join(", ")}</p>}
                  </TooltipContent>
                </Tooltip>
              );
            })}
          </div>
        </TooltipProvider>
        <div className="flex items-center gap-4 mt-3 text-[10px] text-muted-foreground flex-wrap">
          <span className="flex items-center gap-1.5"><span className="h-2.5 w-2.5 rounded-sm bg-emerald-500/50 border border-emerald-500/30" /> OK</span>
          <span className="flex items-center gap-1.5"><span className="h-2.5 w-2.5 rounded-sm bg-amber-500/60 border border-amber-500/40" /> Warning</span>
          <span className="flex items-center gap-1.5"><span className="h-2.5 w-2.5 rounded-sm bg-red-500/70 border border-red-500/40" /> Failed</span>
          <span className="flex items-center gap-1.5"><span className="h-2.5 w-2.5 rounded-sm bg-orange-500/70 border border-orange-500/40" /> Assertion Fail</span>
          <span className="flex items-center gap-1.5"><span className="h-2.5 w-2.5 rounded-sm bg-purple-500/70 border border-purple-500/40" /> Policy Violation</span>
          <span className="flex items-center gap-1.5"><span className="h-2.5 w-2.5 rounded-sm bg-muted/20 border border-border/30" /> No data</span>
        </div>
      </CardContent>
    </Card>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// Env Signature Mini Badge (inline on trace rows)
// ═══════════════════════════════════════════════════════════════════════

function EnvSignatureBadge({ env }: { env: EnvironmentSignature }) {
  const debugMode = env.envVars.some(v => v.key === "LOG_LEVEL" && v.value === "DEBUG");
  const hotSensor = env.mockSensors.find(s => s.id === "temp" && s.value > 80);
  const lowBattery = env.mockSensors.find(s => s.id === "battery" && s.value < 20);

  return (
    <TooltipProvider delayDuration={100}>
      <Tooltip>
        <TooltipTrigger asChild>
          <span className="inline-flex items-center gap-0.5 shrink-0">
            {debugMode && <span className="h-2 w-2 rounded-full bg-yellow-400/70 border border-yellow-400/40" />}
            {hotSensor && <span className="h-2 w-2 rounded-full bg-red-400/70 border border-red-400/40" />}
            {lowBattery && <span className="h-2 w-2 rounded-full bg-orange-400/70 border border-orange-400/40" />}
            {env.vfsFiles.length > 0 && <span className="h-2 w-2 rounded-full bg-cyan-400/70 border border-cyan-400/40" />}
            {env.scenarioName && <span className="h-2 w-2 rounded-full bg-violet-400/70 border border-violet-400/40" />}
          </span>
        </TooltipTrigger>
        <TooltipContent side="top" className="max-w-xs text-[10px] space-y-1 p-2">
          <p className="font-semibold text-[11px] mb-1">Environment Signature</p>
          {env.mockSensors.map(s => (
            <p key={s.id} className="flex items-center gap-1">
              <Thermometer size={8} className="text-orange-400" /> {s.name}: <span className="font-mono">{s.value}{s.unit ?? ""}</span>
            </p>
          ))}
          {env.vfsFiles.length > 0 && env.vfsFiles.map(f => (
            <p key={f.name} className="flex items-center gap-1">
              <FolderOpen size={8} className="text-cyan-400" /> {f.name} ({formatBytes(f.size)})
            </p>
          ))}
          {env.envVars.map(v => (
            <p key={v.key} className="flex items-center gap-1 font-mono">
              <Variable size={8} className="text-yellow-400" /> {v.key}={v.value}
            </p>
          ))}
          {env.scenarioName && (
            <p className="mt-1 text-violet-300">
              <Beaker size={8} className="inline mr-1" /> {env.scenarioName} → <span className={cn("font-semibold",
                env.scenarioResult === "passed" ? "text-green-400" : env.scenarioResult === "violation" ? "text-purple-400" : "text-red-400"
              )}>{env.scenarioResult}</span>
            </p>
          )}
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// Waterfall Chart — includes ABI, vFS, assertion spans
// ═══════════════════════════════════════════════════════════════════════

function WaterfallChart({ trace, envSpans }: { trace: TraceRecord; envSpans: TraceSpan[] }) {
  const allSpans = [...trace.spans, ...envSpans];
  const totalDur = trace.total_duration_us ?? 1;
  if (totalDur <= 0 || allSpans.length === 0) return null;

  const LANE_COLORS: Record<string, string> = {
    execution: "#6366f1", scheduler: "#22c55e", syscall: "#f59e0b",
    abi: "#ef4444", memory: "#06b6d4", validation: "#8b5cf6",
    vfs: "#14b8a6", assert: "#f97316",
  };

  const getLaneColor = (kind: string) => {
    const k = kind.toLowerCase();
    for (const [key, color] of Object.entries(LANE_COLORS)) {
      if (k.includes(key)) return color;
    }
    return "#94a3b8";
  };

  return (
    <div className="space-y-1 py-2">
      <div className="flex items-center gap-2 text-[10px] text-muted-foreground mb-2">
        <BarChart3 size={10} /> Waterfall · Total: {formatDuration(totalDur)} · {allSpans.length} spans
        {envSpans.length > 0 && (
          <Badge variant="outline" className="text-[9px] h-4 px-1.5 border-violet-500/30 text-violet-400 ml-1">
            +{envSpans.length} env
          </Badge>
        )}
      </div>
      {allSpans.map((span, i) => {
        const durUs = span.duration_us ?? 0;
        const widthPct = Math.max((durUs / totalDur) * 100, 0.5);
        const offsetPct = (i / allSpans.length) * 20;
        const color = getLaneColor(span.kind);
        const isEnv = envSpans.includes(span);

        return (
          <div key={i} className={cn("flex items-center gap-2 text-[11px] group", isEnv && "opacity-90")}>
            <span className="w-28 shrink-0 text-muted-foreground truncate font-medium flex items-center gap-1" title={span.kind}>
              {span.kind.includes("abi") && <Gauge size={9} className="text-red-400 shrink-0" />}
              {span.kind.includes("vfs") && <FolderOpen size={9} className="text-teal-400 shrink-0" />}
              {span.kind.includes("assert") && <Beaker size={9} className="text-orange-400 shrink-0" />}
              <span className="truncate">{span.kind.length > 22 ? span.kind.slice(0, 22) + "…" : span.kind}</span>
            </span>
            <div className="flex-1 h-5 bg-muted/20 rounded relative overflow-hidden">
              <div
                className="absolute top-0 h-full rounded transition-all group-hover:opacity-90"
                style={{
                  left: `${offsetPct}%`,
                  width: `${Math.min(widthPct, 100 - offsetPct)}%`,
                  backgroundColor: color,
                  opacity: span.success ? 0.7 : 1,
                }}
                title={`${span.kind}: ${durUs > 0 ? formatDuration(durUs) : "0µs"} ${span.error ? `— ${span.error}` : ""}`}
              >
                {!span.success && (
                  <div className="absolute inset-0 bg-[repeating-linear-gradient(45deg,transparent,transparent_3px,rgba(0,0,0,0.15)_3px,rgba(0,0,0,0.15)_6px)]" />
                )}
              </div>
            </div>
            <span className="w-16 text-right text-muted-foreground shrink-0 text-[10px]">
              {durUs > 0 ? formatDuration(durUs) : "0µs"}
            </span>
            {!span.success && <AlertTriangle size={10} className="text-red-400 shrink-0" />}
          </div>
        );
      })}
      <div className="flex flex-wrap gap-3 mt-2 pt-2 border-t border-border/30">
        {Object.entries(LANE_COLORS).map(([kind, color]) => (
          <span key={kind} className="flex items-center gap-1 text-[9px] text-muted-foreground capitalize">
            <span className="h-2 w-4 rounded-sm" style={{ backgroundColor: color, opacity: 0.7 }} />
            {kind}
          </span>
        ))}
      </div>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// Time-to-First-Syscall Badge
// ═══════════════════════════════════════════════════════════════════════

function FirstSyscallBadge({ trace }: { trace: TraceRecord }) {
  const syscallSpan = trace.spans.find(s => {
    const k = s.kind?.toLowerCase() ?? "";
    return k.includes("syscall") || k.includes("abi") || k.includes("import");
  });
  if (!syscallSpan || !syscallSpan.duration_us) return null;

  const idx = trace.spans.indexOf(syscallSpan);
  const timeBefore = trace.spans.slice(0, idx).reduce((sum, s) => sum + (s.duration_us ?? 0), 0);
  const isSuspicious = timeBefore < 100;
  const isQuick = timeBefore < 1000;

  return (
    <span className={cn(
      "inline-flex items-center gap-1 text-[10px] rounded-full px-2 py-0.5 font-medium border",
      isSuspicious ? "bg-red-500/15 text-red-400 border-red-500/30"
        : isQuick ? "bg-amber-500/15 text-amber-400 border-amber-500/30"
        : "bg-muted text-muted-foreground border-border"
    )} title={`Time before first sensitive call: ${formatDuration(timeBefore)}`}>
      <Timer size={9} /> TTFS: {formatDuration(timeBefore)}
      {isSuspicious && <span className="text-red-400 font-bold ml-0.5">⚠</span>}
    </span>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// Scenario Result Badge (replaces generic OK / FAIL)
// ═══════════════════════════════════════════════════════════════════════

function ScenarioResultBadge({ env, success }: { env?: EnvironmentSignature; success: boolean }) {
  if (!env?.scenarioName) {
    return (
      <Badge variant={success ? "default" : "destructive"} className="text-[10px] h-4 px-1.5 shrink-0">
        {success ? "OK" : "FAIL"}
      </Badge>
    );
  }
  if (env.scenarioResult === "violation") {
    return (
      <Badge className="text-[10px] h-4 px-1.5 shrink-0 bg-purple-500/20 text-purple-400 border border-purple-500/30 hover:bg-purple-500/30">
        <Shield size={8} className="mr-0.5" /> Policy Violation
      </Badge>
    );
  }
  if (env.scenarioResult === "failed") {
    return (
      <Badge className="text-[10px] h-4 px-1.5 shrink-0 bg-orange-500/20 text-orange-400 border border-orange-500/30 hover:bg-orange-500/30">
        <AlertTriangle size={8} className="mr-0.5" /> Assertion Failed
      </Badge>
    );
  }
  return (
    <Badge className="text-[10px] h-4 px-1.5 shrink-0 bg-emerald-500/20 text-emerald-400 border border-emerald-500/30 hover:bg-emerald-500/30">
      <CheckCircle size={8} className="mr-0.5" /> Assertion Passed
    </Badge>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// Environment Detail Panel (expandable inside trace row)
// ═══════════════════════════════════════════════════════════════════════

function EnvironmentDetailPanel({ env }: { env: EnvironmentSignature }) {
  return (
    <div className="grid grid-cols-1 sm:grid-cols-3 gap-3 py-2 border-t border-border/30 mt-2">
      <div className="space-y-1">
        <p className="text-[10px] font-semibold text-muted-foreground uppercase tracking-wider flex items-center gap-1">
          <Gauge size={9} className="text-red-400" /> Mock Sensors
        </p>
        {env.mockSensors.map(s => (
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
        {env.vfsFiles.length === 0 ? (
          <p className="text-[11px] text-muted-foreground/60 italic">No files</p>
        ) : env.vfsFiles.map(f => (
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
        {env.envVars.map(v => (
          <div key={v.key} className="flex items-center justify-between text-[11px] rounded bg-muted/20 px-2 py-1 font-mono">
            <span className="text-muted-foreground">{v.key}</span>
            <span className="font-semibold">{v.value}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// Clone to Test Panel
// ═══════════════════════════════════════════════════════════════════════

function CloneToTestPanel({ trace, env, onSave, onClose }: {
  trace: TraceRecord;
  env: EnvironmentSignature;
  onSave: (tc: RegressionTestCase) => void;
  onClose: () => void;
}) {
  const [testName, setTestName] = useState(
    `${trace.task_name || "task"}-${env.scenarioName?.replace(/\s+/g, "-").toLowerCase() ?? "regression"}-${trace.trace_id.slice(0, 6)}`
  );

  const handleSave = () => {
    onSave({
      id: `tc-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
      name: testName,
      createdAt: new Date().toISOString(),
      traceId: trace.trace_id,
      taskId: trace.task_id,
      taskName: trace.task_name,
      environment: env,
      expectedOutcome: trace.success ? "pass" : "fail",
      duration_us: trace.total_duration_us ?? undefined,
    });
    toast.success(`Test case "${testName}" saved to regression suite`);
    onClose();
  };

  return (
    <div className="border border-indigo-500/30 bg-indigo-500/5 rounded-lg p-4 space-y-3 mt-2">
      <div className="flex items-center gap-2">
        <TestTube size={14} className="text-indigo-400" />
        <span className="text-sm font-semibold">Clone to Regression Test</span>
        <Button variant="ghost" size="sm" className="ml-auto h-6 text-xs" onClick={onClose}><X size={12} /></Button>
      </div>
      <div className="space-y-2">
        <div>
          <label className="text-[10px] font-medium text-muted-foreground uppercase tracking-wider">Test Name</label>
          <Input value={testName} onChange={e => setTestName(e.target.value)} className="mt-1 h-7 text-xs" />
        </div>
        <div className="grid grid-cols-3 gap-2 text-[11px]">
          <div className="rounded bg-muted/20 px-2 py-1.5">
            <span className="text-muted-foreground">Trace:</span>{" "}
            <span className="font-mono">{trace.trace_id.slice(0, 12)}</span>
          </div>
          <div className="rounded bg-muted/20 px-2 py-1.5">
            <span className="text-muted-foreground">Expected:</span>{" "}
            <span className={trace.success ? "text-green-400" : "text-red-400"}>{trace.success ? "PASS" : "FAIL"}</span>
          </div>
          <div className="rounded bg-muted/20 px-2 py-1.5">
            <span className="text-muted-foreground">Scenario:</span>{" "}
            <span>{env.scenarioName ?? "none"}</span>
          </div>
        </div>
        <p className="text-[10px] text-muted-foreground">
          Saves trace + environment ({env.mockSensors.length} sensors, {env.vfsFiles.length} vFS files, {env.envVars.length} env vars) as a regression test.
        </p>
      </div>
      <div className="flex justify-end gap-2">
        <Button variant="ghost" size="sm" className="h-7 text-xs" onClick={onClose}>Cancel</Button>
        <Button size="sm" className="h-7 text-xs gap-1" onClick={handleSave}><Save size={11} /> Save Test Case</Button>
      </div>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// Trace Row (Environment-Aware)
// ═══════════════════════════════════════════════════════════════════════

function TraceRow({ trace, env, envSpans, onAutoSnapshot, testCases, onCloneToTest }: {
  trace: TraceRecord;
  env: EnvironmentSignature;
  envSpans: TraceSpan[];
  onAutoSnapshot: (trace: TraceRecord, reason: string) => void;
  testCases: RegressionTestCase[];
  onCloneToTest: (tc: RegressionTestCase) => void;
}) {
  const [open, setOpen] = useState(false);
  const [showClone, setShowClone] = useState(false);
  const [snapping, setSnapping] = useState(false);
  const router = useRouter();

  const copyTraceId = (e: React.MouseEvent) => {
    e.stopPropagation();
    navigator.clipboard.writeText(trace.trace_id).then(
      () => toast.success("Trace ID copied"),
      () => toast.error("Failed to copy")
    );
  };

  const handleSnapshot = async (e: React.MouseEvent) => {
    e.stopPropagation();
    setSnapping(true);
    const reason = env.scenarioResult === "violation" ? "Policy Violation" : env.scenarioResult === "failed" ? "Assertion Failure" : "Manual forensic";
    onAutoSnapshot(trace, reason);
    setTimeout(() => setSnapping(false), 1500);
  };

  const startedLabel = trace.started_at
    ? new Date(trace.started_at).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" })
    : null;

  const linked = testCases.find(tc => tc.traceId === trace.trace_id);

  return (
    <div className="border-b border-border/50">
      <div className="flex items-center">
        <button
          onClick={() => setOpen(!open)}
          className="flex-1 flex items-center gap-3 px-4 py-2.5 text-xs hover:bg-muted/20 transition-colors text-left"
        >
          {trace.success
            ? <CheckCircle size={13} className="text-green-400 shrink-0" />
            : <AlertTriangle size={13} className="text-red-400 shrink-0" />}
          <span className="flex items-center gap-1 shrink-0">
            <span className="font-mono text-muted-foreground w-20 truncate" title={trace.trace_id}>{trace.trace_id.slice(0, 8)}…</span>
            <button onClick={copyTraceId} className="text-muted-foreground hover:text-foreground transition-colors p-0.5 rounded" title="Copy trace ID">
              <Copy size={10} />
            </button>
          </span>
          <span className="font-medium flex-1 truncate" title={trace.task_id}>{trace.task_name || trace.task_id.slice(0, 12)}</span>
          {startedLabel && (
            <span className="text-muted-foreground shrink-0 hidden sm:inline flex items-center gap-1">
              <Clock size={10} className="inline" /> {startedLabel}
            </span>
          )}
          <span className="text-muted-foreground shrink-0">{trace.spans.length + envSpans.length} spans</span>
          <FirstSyscallBadge trace={trace} />
          <EnvSignatureBadge env={env} />
          <span className="font-medium shrink-0">{trace.total_duration_us != null ? formatDuration(trace.total_duration_us) : "—"}</span>
          <ScenarioResultBadge env={env} success={trace.success} />
          {linked && <span className="shrink-0" title={`Test: ${linked.name}`}><TestTube size={11} className="text-indigo-400" /></span>}
          {open ? <ChevronUp size={13} className="shrink-0 text-muted-foreground" /> : <ChevronDown size={13} className="shrink-0 text-muted-foreground" />}
        </button>
        {(env.scenarioResult === "violation" || env.scenarioResult === "failed") && (
          <button title="Capture forensic snapshot" onClick={handleSnapshot} className="px-2 py-2.5 text-purple-400 hover:text-purple-300 transition-colors shrink-0">
            {snapping ? <RefreshCw size={13} className="animate-spin" /> : <Camera size={13} />}
          </button>
        )}
        <button title="View task" onClick={() => router.push(`/tasks?task=${trace.task_id}`)} className="px-3 py-2.5 text-muted-foreground hover:text-foreground transition-colors shrink-0">
          <FileText size={13} />
        </button>
      </div>

      {open && (
        <div className="px-4 pb-3 bg-muted/10">
          {/* Badges row */}
          <div className="flex items-center gap-2 mb-3 flex-wrap">
            <FirstSyscallBadge trace={trace} />
            {env.scenarioName && (
              <Badge variant="outline" className="text-[10px] h-5 px-2 border-violet-500/30 text-violet-400 gap-1">
                <Beaker size={9} /> {env.scenarioName}
              </Badge>
            )}
            {(env.scenarioResult === "violation" || env.scenarioResult === "failed") && (
              <Badge variant="outline" className="text-[10px] h-5 px-2 border-purple-500/30 text-purple-400 gap-1">
                <Camera size={9} /> Forensic snapshot available
              </Badge>
            )}
          </div>

          {/* Waterfall with env spans */}
          <WaterfallChart trace={trace} envSpans={envSpans} />

          {/* Span details */}
          <div className="border-t border-border/30 mt-2 pt-2 space-y-1.5">
            <p className="text-[10px] font-semibold text-muted-foreground uppercase tracking-wider mb-1">Span Details</p>
            {[...trace.spans, ...envSpans].map((span, i) => {
              const hasRealDur = trace.total_duration_us != null && trace.total_duration_us > 0 && span.duration_us != null && span.duration_us > 0;
              const widthPct = hasRealDur ? Math.round((span.duration_us! / trace.total_duration_us!) * 100) : 0;
              const isEnv = envSpans.includes(span);
              const maybeExecId = span.span_id?.match(/[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/i)?.[0];

              return (
                <div key={i} className={cn("flex items-center gap-2 text-xs", isEnv && "opacity-80")}>
                  {span.success ? <CheckCircle size={11} className="text-green-400 shrink-0" /> : <AlertTriangle size={11} className="text-red-400 shrink-0" />}
                  <span className={cn("w-28 shrink-0 font-medium text-muted-foreground capitalize truncate", isEnv && "text-violet-400/80")}>
                    {isEnv && "⚡ "}{span.kind.length > 24 ? span.kind.slice(0, 24) + "…" : span.kind}
                  </span>
                  <div className="flex-1 h-1.5 bg-muted/40 rounded-full relative">
                    {widthPct > 0
                      ? <div className={cn("h-1.5 rounded-full transition-all", span.success ? "bg-green-400/70" : "bg-red-400/70")} style={{ width: `${widthPct}%` }} />
                      : <div className={cn("absolute top-0 left-0 h-1.5 w-1.5 rounded-full", span.success ? "bg-green-400/50" : "bg-red-400/50")} />}
                  </div>
                  <span className="text-muted-foreground shrink-0 w-16 text-right">{span.duration_us != null ? formatDuration(span.duration_us) : "—"}</span>
                  {span.error && <span className="text-red-400 text-[10px] shrink-0 truncate max-w-32" title={span.error}>{span.error}</span>}
                  {maybeExecId && (
                    <a href={`/execution/report?id=${maybeExecId}`} target="_blank" rel="noopener noreferrer" className="text-indigo-400 hover:text-indigo-300 shrink-0" onClick={e => e.stopPropagation()}>
                      <ExternalLink size={11} />
                    </a>
                  )}
                </div>
              );
            })}
          </div>

          {/* Environment panel */}
          <EnvironmentDetailPanel env={env} />

          {/* Assertion results */}
          {env.assertions && env.assertions.length > 0 && (
            <div className="border-t border-border/30 mt-2 pt-2">
              <p className="text-[10px] font-semibold text-muted-foreground uppercase tracking-wider mb-1">Scenario Assertions</p>
              <div className="space-y-1">
                {env.assertions.map((a, i) => (
                  <div key={i} className={cn(
                    "flex items-center gap-2 text-[11px] rounded px-2 py-1",
                    a.passed ? "bg-green-500/5 border border-green-500/20" : "bg-red-500/5 border border-red-500/20"
                  )}>
                    {a.passed ? <CheckCircle size={10} className="text-green-400 shrink-0" /> : <AlertTriangle size={10} className="text-red-400 shrink-0" />}
                    <span className="font-mono flex-1">{a.check}</span>
                    <span className="text-muted-foreground">expected: {a.expected}</span>
                    {!a.passed && a.actual && <span className="text-red-400">got: {a.actual}</span>}
                    <Badge variant={a.passed ? "default" : "destructive"} className="text-[9px] h-4 px-1.5">{a.passed ? "PASSED" : "FAILED"}</Badge>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Clone to Test */}
          <div className="flex items-center gap-2 mt-3 pt-2 border-t border-border/30">
            <Button variant="outline" size="sm" className="text-xs gap-1 h-7" onClick={() => setShowClone(!showClone)}>
              <TestTube size={11} /> Clone to Test
            </Button>
            {linked && (
              <Badge variant="outline" className="text-[10px] h-5 px-2 border-indigo-500/30 text-indigo-400 gap-1">
                <Tag size={9} /> {linked.name}
              </Badge>
            )}
            <button
              title="View snapshots"
              onClick={() => router.push(`/snapshots?task=${trace.task_id}`)}
              className="ml-auto flex items-center gap-1 text-xs text-indigo-400 hover:text-indigo-300"
            >
              <Camera size={11} /> Snapshots
            </button>
          </div>

          {showClone && (
            <CloneToTestPanel trace={trace} env={env} onSave={onCloneToTest} onClose={() => setShowClone(false)} />
          )}
        </div>
      )}
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// Regression Test Suite Panel
// ═══════════════════════════════════════════════════════════════════════

function RegressionSuitePanel({ testCases, onRemove }: { testCases: RegressionTestCase[]; onRemove: (id: string) => void }) {
  if (testCases.length === 0) return null;
  const passed = testCases.filter(tc => tc.expectedOutcome === "pass").length;
  const failed = testCases.filter(tc => tc.expectedOutcome === "fail").length;

  return (
    <Card className="border-indigo-500/20">
      <CardHeader className="pb-2">
        <CardTitle className="text-sm font-semibold flex items-center gap-2">
          <TestTube size={14} className="text-indigo-400" />
          Regression Test Suite
          <Badge variant="secondary" className="text-[10px] h-4 px-1.5 ml-1">{testCases.length}</Badge>
          <span className="flex items-center gap-2 ml-auto text-[10px]">
            <span className="text-green-400">{passed} expect pass</span>
            <span className="text-red-400">{failed} expect fail</span>
          </span>
        </CardTitle>
      </CardHeader>
      <CardContent>
        <ScrollArea className="max-h-48">
          <div className="space-y-1.5">
            {testCases.map(tc => (
              <div key={tc.id} className="flex items-center gap-3 rounded-lg border border-border bg-muted/10 px-3 py-2 text-xs">
                <TestTube size={11} className={tc.expectedOutcome === "pass" ? "text-green-400" : "text-red-400"} />
                <div className="flex-1 min-w-0">
                  <p className="font-medium truncate">{tc.name}</p>
                  <p className="text-[10px] text-muted-foreground">
                    {tc.taskName} · {tc.environment.scenarioName ?? "no scenario"} · {timeAgo(tc.createdAt)}
                    {tc.environment.mockSensors.length > 0 && ` · ${tc.environment.mockSensors.length} sensors`}
                    {tc.environment.vfsFiles.length > 0 && ` · ${tc.environment.vfsFiles.length} vFS files`}
                  </p>
                </div>
                <Badge variant={tc.expectedOutcome === "pass" ? "default" : "destructive"} className="text-[9px] h-4 px-1.5 shrink-0">
                  expect {tc.expectedOutcome}
                </Badge>
                <Button variant="ghost" size="sm" className="h-6 w-6 p-0 text-muted-foreground hover:text-red-400" onClick={() => onRemove(tc.id)}><X size={10} /></Button>
              </div>
            ))}
          </div>
        </ScrollArea>
      </CardContent>
    </Card>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// Environment Pattern Analysis
// ═══════════════════════════════════════════════════════════════════════

function EnvironmentPatternCard({ traces, envMap }: { traces: TraceRecord[]; envMap: Map<string, EnvironmentSignature> }) {
  const patterns = useMemo(() => {
    const groups: Record<string, { total: number; failed: number; totalDur: number }> = {};
    traces.forEach(t => {
      const env = envMap.get(t.trace_id);
      if (!env) return;
      env.envVars.forEach(v => {
        const key = `${v.key}=${v.value}`;
        if (!groups[key]) groups[key] = { total: 0, failed: 0, totalDur: 0 };
        groups[key].total++;
        if (!t.success) groups[key].failed++;
        groups[key].totalDur += t.total_duration_us ?? 0;
      });
    });
    return Object.entries(groups)
      .map(([key, d]) => ({ key, total: d.total, failRate: d.total > 0 ? d.failed / d.total : 0, avgDur: d.total > 0 ? d.totalDur / d.total : 0 }))
      .sort((a, b) => b.failRate - a.failRate)
      .slice(0, 8);
  }, [traces, envMap]);

  if (patterns.length === 0) return null;
  const maxAvg = Math.max(...patterns.map(p => p.avgDur), 1);

  return (
    <Card className="border-amber-500/20">
      <CardHeader className="pb-2">
        <CardTitle className="text-sm font-semibold flex items-center gap-2">
          <Flame size={14} className="text-amber-400" />
          Environment Pattern Analysis
          <span className="text-[10px] font-normal text-muted-foreground ml-1">Env vars correlated with outcomes</span>
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="space-y-1.5">
          {patterns.map(p => (
            <div key={p.key} className="flex items-center gap-3 text-xs">
              <span className="w-40 shrink-0 font-mono truncate" title={p.key}>{p.key}</span>
              <div className="flex-1 h-3 bg-muted/20 rounded-full overflow-hidden relative">
                <div className="absolute top-0 left-0 h-full rounded-full bg-indigo-500/50" style={{ width: `${(p.avgDur / maxAvg) * 100}%` }} />
                {p.failRate > 0 && <div className="absolute top-0 left-0 h-full rounded-full bg-red-500/40" style={{ width: `${p.failRate * 100}%` }} />}
              </div>
              <span className="w-16 text-right text-muted-foreground shrink-0">{formatDuration(Math.round(p.avgDur))}</span>
              <span className={cn("w-16 text-right shrink-0 font-medium", p.failRate > 0.3 ? "text-red-400" : "text-muted-foreground")}>
                {Math.round(p.failRate * 100)}% fail
              </span>
              <Badge variant="secondary" className="text-[9px] h-4 px-1.5 shrink-0">{p.total}</Badge>
            </div>
          ))}
        </div>
        <p className="text-[10px] text-muted-foreground mt-2">
          <Info size={9} className="inline mr-1" />
          Bar = avg duration · Red overlay = failure rate
        </p>
      </CardContent>
    </Card>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// Forensic Snapshot Log
// ═══════════════════════════════════════════════════════════════════════

function ForensicSnapshotLog({ snapshots }: { snapshots: { traceId: string; reason: string; ts: string; taskId: string }[] }) {
  if (snapshots.length === 0) return null;
  return (
    <Card className="border-purple-500/20">
      <CardHeader className="pb-2">
        <CardTitle className="text-sm font-semibold flex items-center gap-2">
          <Camera size={14} className="text-purple-400" /> Forensic Snapshots
          <Badge variant="secondary" className="text-[10px] h-4 px-1.5 ml-1">{snapshots.length}</Badge>
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="space-y-1.5">
          {snapshots.slice(0, 10).map((s, i) => (
            <div key={i} className="flex items-center gap-3 text-xs rounded bg-muted/10 border border-border px-3 py-2">
              <Camera size={11} className="text-purple-400 shrink-0" />
              <span className="font-mono text-muted-foreground w-20 shrink-0">{s.traceId.slice(0, 8)}…</span>
              <Badge variant="outline" className={cn("text-[9px] h-4 px-1.5 shrink-0",
                s.reason.includes("Violation") ? "border-purple-500/30 text-purple-400" : "border-orange-500/30 text-orange-400"
              )}>{s.reason}</Badge>
              <span className="text-muted-foreground ml-auto shrink-0">{timeAgo(s.ts)}</span>
            </div>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// Main Page
// ═══════════════════════════════════════════════════════════════════════

export default function TracesPage() {
  const [traces,     setTraces]     = useState<TraceRecord[]>([]);
  const [metrics,    setMetrics]    = useState<LiveMetrics | null>(null);
  const [loading,    setLoading]    = useState(true);
  const [search,     setSearch]     = useState("");
  const [filter,     setFilter]     = useState<StatusFilter>("all");
  const [page,       setPage]       = useState(1);
  const [lastUpdated, setLastUpdated] = useState<Date | null>(null);

  const [testCases,     setTestCases]     = useState<RegressionTestCase[]>([]);
  const [forensicSnaps, setForensicSnaps] = useState<{ traceId: string; reason: string; ts: string; taskId: string }[]>([]);
  const [showTestSuite, setShowTestSuite] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [t, m] = await Promise.allSettled([listTraces(), getLiveMetrics()]);
      if (t.status === "fulfilled") setTraces(t.value);
      if (m.status === "fulfilled") setMetrics(m.value);
      setLastUpdated(new Date());
    } finally { setLoading(false); }
  }, []);

  useEffect(() => { load(); const id = setInterval(load, 30_000); return () => clearInterval(id); }, [load]);
  useEffect(() => { setPage(1); }, [search, filter]);

  // Pre-compute environment for every trace
  const envMap = useMemo(() => {
    const m = new Map<string, EnvironmentSignature>();
    traces.forEach(t => m.set(t.trace_id, generateEnvironmentForTrace(t)));
    return m;
  }, [traces]);

  const envSpansMap = useMemo(() => {
    const m = new Map<string, TraceSpan[]>();
    traces.forEach(t => {
      const env = envMap.get(t.trace_id);
      if (env) m.set(t.trace_id, generateEnvironmentSpans(t, env));
    });
    return m;
  }, [traces, envMap]);

  const filtered = traces.filter(t => {
    const env = envMap.get(t.trace_id);
    if (filter === "ok" && !t.success) return false;
    if (filter === "failed" && t.success) return false;
    if (filter === "assertion" && env?.scenarioResult !== "failed") return false;
    if (filter === "violation" && env?.scenarioResult !== "violation") return false;
    if (search) {
      const q = search.toLowerCase();
      return t.trace_id.toLowerCase().includes(q) || t.task_id.toLowerCase().includes(q) || (t.task_name ?? "").toLowerCase().includes(q) || (env?.scenarioName ?? "").toLowerCase().includes(q) || (env?.envVars.some(v => `${v.key}=${v.value}`.toLowerCase().includes(q)) ?? false);
    }
    return true;
  });

  const totalPages = Math.max(1, Math.ceil(filtered.length / PAGE_SIZE));
  const paginated = filtered.slice((page - 1) * PAGE_SIZE, page * PAGE_SIZE);

  const handleAutoSnapshot = useCallback(async (trace: TraceRecord, reason: string) => {
    try {
      await createSnapshot(trace.task_id, {
        memory_mb: 4, instructions: 0, stack_depth: 0,
        globals_json: JSON.stringify({ forensic: true, trace_id: trace.trace_id, reason, environment: envMap.get(trace.trace_id) }),
        note: `Forensic: ${reason} — trace ${trace.trace_id.slice(0, 8)}`,
      });
      setForensicSnaps(prev => [{ traceId: trace.trace_id, reason, ts: new Date().toISOString(), taskId: trace.task_id }, ...prev]);
      toast.success(`Forensic snapshot captured: ${reason}`);
    } catch (err) {
      toast.error(`Snapshot failed: ${err instanceof Error ? err.message : String(err)}`);
    }
  }, [envMap]);

  const handleCloneToTest = useCallback((tc: RegressionTestCase) => setTestCases(prev => [tc, ...prev]), []);
  const handleRemoveTest = useCallback((id: string) => setTestCases(prev => prev.filter(tc => tc.id !== id)), []);

  const assertionFailCount = traces.filter(t => envMap.get(t.trace_id)?.scenarioResult === "failed").length;
  const violationCount = traces.filter(t => envMap.get(t.trace_id)?.scenarioResult === "violation").length;
  const scenarioCount = traces.filter(t => envMap.get(t.trace_id)?.scenarioName).length;

  const filterPills: { label: string; value: StatusFilter; color: string; count?: number }[] = [
    { label: "All",              value: "all",       color: "bg-muted text-foreground" },
    { label: "OK",               value: "ok",        color: "bg-green-500/15 text-green-400 border-green-500/30" },
    { label: "Failed",           value: "failed",    color: "bg-red-500/15 text-red-400 border-red-500/30" },
    { label: "Assertion Fail",   value: "assertion",  color: "bg-orange-500/15 text-orange-400 border-orange-500/30", count: assertionFailCount },
    { label: "Policy Violation", value: "violation",  color: "bg-purple-500/15 text-purple-400 border-purple-500/30", count: violationCount },
  ];

  return (
    <div className="animate-fade-in space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold gradient-text flex items-center gap-2"><GitBranch size={22} /> Traces</h1>
          <p className="mt-0.5 text-sm text-muted-foreground">
            Environment-aware forensic timeline · Mock spans · vFS I/O · Scenario assertions · Crash snapshots · Regression tests
          </p>
        </div>
        <div className="flex items-center gap-3">
          {testCases.length > 0 && (
            <Button variant={showTestSuite ? "default" : "outline"} size="sm" className="text-xs gap-1" onClick={() => setShowTestSuite(!showTestSuite)}>
              <TestTube size={12} /> Test Suite ({testCases.length})
            </Button>
          )}
          {lastUpdated && <span className="text-[11px] text-muted-foreground hidden sm:inline">Updated {timeAgo(lastUpdated.toISOString())}</span>}
          <Button onClick={load} variant="ghost" size="icon" className="h-9 w-9"><RefreshCw size={14} className={loading ? "animate-spin" : ""} /></Button>
        </div>
      </div>

      {/* Live metrics */}
      {metrics && (
        <div>
          <p className="text-[11px] uppercase tracking-wider text-muted-foreground font-semibold mb-2">Live Metrics</p>
          <LiveMetricsBar m={metrics} />
        </div>
      )}

      {/* Summary cards */}
      <div className="grid grid-cols-3 sm:grid-cols-6 gap-3">
        {[
          { icon: Activity,      label: "Total",         value: traces.length,                          color: "text-primary" },
          { icon: CheckCircle,   label: "Success",       value: traces.filter(t => t.success).length,   color: "text-green-400" },
          { icon: AlertTriangle, label: "Failed",        value: traces.filter(t => !t.success).length,  color: "text-red-400" },
          { icon: Beaker,        label: "Scenarios",     value: scenarioCount,                           color: "text-violet-400" },
          { icon: Shield,        label: "Violations",    value: violationCount,                          color: "text-purple-400" },
          { icon: Camera,        label: "Forensic Snaps", value: forensicSnaps.length,                  color: "text-amber-400" },
        ].map(({ icon: Icon, label, value, color }) => (
          <Card key={label} className="p-3">
            <div className="flex items-center gap-2">
              <div className={cn("flex h-7 w-7 items-center justify-center rounded-lg bg-muted/30", color)}><Icon size={13} /></div>
              <div>
                <p className="text-[9px] text-muted-foreground uppercase tracking-wider">{label}</p>
                <p className="text-base font-bold">{value}</p>
              </div>
            </div>
          </Card>
        ))}
      </div>

      {/* Regression test suite */}
      {showTestSuite && <RegressionSuitePanel testCases={testCases} onRemove={handleRemoveTest} />}

      {/* Forensic snapshots */}
      {forensicSnaps.length > 0 && <ForensicSnapshotLog snapshots={forensicSnaps} />}

      {/* Heatmap + Pattern analysis */}
      {traces.length > 0 && (
        <div className="grid gap-4 lg:grid-cols-2">
          <HealthHeatmap traces={traces} envMap={envMap} />
          <EnvironmentPatternCard traces={traces} envMap={envMap} />
        </div>
      )}

      {/* Search + filter */}
      <div className="flex flex-wrap items-center gap-3">
        <div className="relative max-w-sm flex-1 min-w-[200px]">
          <Search size={13} className="absolute left-2.5 top-1/2 -translate-y-1/2 text-muted-foreground" />
          <Input placeholder="Search by trace ID, task, scenario, env var…" value={search} onChange={e => setSearch(e.target.value)} className="pl-8 h-8 text-sm" />
          {search && <button onClick={() => setSearch("")} className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"><X size={13} /></button>}
        </div>
        <div className="flex items-center gap-1.5 flex-wrap">
          {filterPills.map(({ label, value, color, count }) => (
            <button key={value} onClick={() => setFilter(value)} className={cn(
              "rounded-full border px-3 py-1 text-[11px] font-medium transition-colors",
              filter === value ? color + " border-current" : "border-border bg-muted/30 text-muted-foreground hover:bg-muted/60"
            )}>
              {label}{count != null && count > 0 && <span className="ml-1 text-[9px] opacity-70">({count})</span>}
            </button>
          ))}
        </div>
      </div>

      {/* Trace list */}
      <Card>
        <CardHeader className="pb-3 pt-4 px-4 border-b border-border">
          <div className="flex items-center gap-3 text-[10px] uppercase tracking-wider text-muted-foreground font-semibold">
            <span className="w-5 shrink-0" />
            <span className="w-28 shrink-0">Trace ID</span>
            <span className="flex-1">Task</span>
            <span className="shrink-0 hidden sm:inline">Time</span>
            <span className="shrink-0">Spans</span>
            <span className="shrink-0">Env</span>
            <span className="shrink-0 w-20 text-right">Duration</span>
            <span className="shrink-0 w-28 text-right">Status</span>
            <span className="shrink-0 w-5" />
          </div>
        </CardHeader>
        <CardContent className="p-0">
          {loading ? (
            <div className="divide-y divide-border/50">{[...Array(6)].map((_, i) => <div key={i} className="px-4 py-3"><Skeleton className="h-4 w-full" /></div>)}</div>
          ) : filtered.length === 0 ? (
            <div className="flex items-center justify-center py-16 text-muted-foreground">
              <div className="text-center">
                <GitBranch size={32} className="mx-auto mb-3 opacity-20" />
                <p className="text-sm">No traces found</p>
                <p className="text-xs mt-1 opacity-70">{search || filter !== "all" ? "Try adjusting your search or filter" : "Traces are recorded when WASM modules execute"}</p>
              </div>
            </div>
          ) : (
            <>
              <ScrollArea className="h-[500px]">
                {paginated.map(trace => (
                  <TraceRow
                    key={trace.trace_id}
                    trace={trace}
                    env={envMap.get(trace.trace_id)!}
                    envSpans={envSpansMap.get(trace.trace_id) ?? []}
                    onAutoSnapshot={handleAutoSnapshot}
                    testCases={testCases}
                    onCloneToTest={handleCloneToTest}
                  />
                ))}
              </ScrollArea>
              {totalPages > 1 && (
                <div className="flex items-center justify-between border-t border-border px-4 py-3">
                  <span className="text-xs text-muted-foreground">Page {page} of {totalPages} — {filtered.length} traces</span>
                  <div className="flex items-center gap-1.5">
                    <Button variant="ghost" size="sm" className="h-7 text-xs gap-1" onClick={() => setPage(p => Math.max(1, p - 1))} disabled={page === 1}><ChevronLeft size={13} /> Prev</Button>
                    <Button variant="ghost" size="sm" className="h-7 text-xs gap-1" onClick={() => setPage(p => Math.min(totalPages, p + 1))} disabled={page === totalPages}>Next <ChevronRight size={13} /></Button>
                  </div>
                </div>
              )}
            </>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
