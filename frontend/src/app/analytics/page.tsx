"use client";

import { useState, useEffect, useRef, useCallback } from "react";
import {
  getLiveMetrics,
  listTraces,
  getSchedulerStatus,
  type LiveMetrics,
  type TraceRecord,
  type SchedulerStatus,
} from "@/lib/api";
import { BarChart3, Activity, Clock, Zap, RefreshCw, TrendingUp } from "lucide-react";

// ─── Helpers ────────────────────────────────────────────────────────

function fmtUs(us: number) {
  if (us < 1000) return `${us.toFixed(0)}µs`;
  if (us < 1_000_000) return `${(us / 1000).toFixed(2)}ms`;
  return `${(us / 1_000_000).toFixed(3)}s`;
}

function pct(v: number) {
  return `${(v * 100).toFixed(1)}%`;
}

// ─── Mini sparkline (pure SVG) ─────────────────────────────────────

function Sparkline({ values, color = "#6366f1", height = 40 }: { values: number[]; color?: string; height?: number }) {
  if (values.length < 2) return null;
  const max = Math.max(...values, 1);
  const w = 200;
  const pts = values.map((v, i) => {
    const x = (i / (values.length - 1)) * w;
    const y = height - (v / max) * height;
    return `${x},${y}`;
  }).join(" ");
  return (
    <svg viewBox={`0 0 ${w} ${height}`} className="w-full" style={{ height }}>
      <polyline points={pts} fill="none" stroke={color} strokeWidth="2" strokeLinejoin="round" />
    </svg>
  );
}

// ─── Metric card ────────────────────────────────────────────────────

function MetricCard({
  label,
  value,
  sub,
  icon: Icon,
  color = "text-primary",
  history,
}: {
  label: string;
  value: string;
  sub?: string;
  icon: React.ElementType;
  color?: string;
  history?: number[];
}) {
  return (
    <div className="rounded-xl border border-border bg-card p-4 flex flex-col gap-2">
      <div className="flex items-center justify-between">
        <span className="text-xs text-muted-foreground font-medium uppercase tracking-wider">{label}</span>
        <Icon className={`h-4 w-4 ${color}`} />
      </div>
      <p className={`text-2xl font-bold ${color}`}>{value}</p>
      {sub && <p className="text-xs text-muted-foreground">{sub}</p>}
      {history && history.length > 1 && (
        <Sparkline values={history} color={color.includes("green") ? "#22c55e" : color.includes("red") ? "#ef4444" : "#6366f1"} />
      )}
    </div>
  );
}

// ─── Main page ──────────────────────────────────────────────────────

export default function AnalyticsPage() {
  const [metrics, setMetrics] = useState<LiveMetrics | null>(null);
  const [traces, setTraces] = useState<TraceRecord[]>([]);
  const [scheduler, setScheduler] = useState<SchedulerStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [lastUpdated, setLastUpdated] = useState<Date | null>(null);
  const [autoRefresh, setAutoRefresh] = useState(true);

  // Historical sparkline data
  const p50History = useRef<number[]>([]);
  const p95History = useRef<number[]>([]);
  const throughputHistory = useRef<number[]>([]);
  const [, forceRender] = useState(0);

  const load = useCallback(async () => {
    try {
      const [m, t, s] = await Promise.allSettled([
        getLiveMetrics(),
        listTraces(),
        getSchedulerStatus(),
      ]);

      if (m.status === "fulfilled") {
        setMetrics(m.value);
        p50History.current = [...p50History.current.slice(-29), m.value.p50_us];
        p95History.current = [...p95History.current.slice(-29), m.value.p95_us];
        throughputHistory.current = [...throughputHistory.current.slice(-29), m.value.throughput_per_min];
        forceRender(n => n + 1);
      }
      if (t.status === "fulfilled") setTraces(Array.isArray(t.value) ? t.value.slice(0, 50) : []);
      if (s.status === "fulfilled") setScheduler(s.value);
      setLastUpdated(new Date());
    } finally {
      setLoading(false);
    }
  }, []);

  // Also try WebSocket for live metrics push
  useEffect(() => {
    const BACKEND = (process.env.NEXT_PUBLIC_BACKEND_URL || "http://127.0.0.1:8080");
    const wsUrl = BACKEND.replace(/^https:\/\//, "wss://").replace(/^http:\/\//, "ws://") + "/ws";
    let ws: WebSocket | null = null;
    let dead = false;

    const connect = () => {
      if (dead) return;
      ws = new WebSocket(wsUrl);
      ws.onmessage = (ev) => {
        try {
          const msg = JSON.parse(ev.data);
          if (msg.type === "live_metrics") {
            const lm = msg as unknown as LiveMetrics;
            setMetrics(lm);
            p50History.current = [...p50History.current.slice(-29), lm.p50_us];
            p95History.current = [...p95History.current.slice(-29), lm.p95_us];
            throughputHistory.current = [...throughputHistory.current.slice(-29), lm.throughput_per_min];
            forceRender(n => n + 1);
          }
        } catch { /* ignore */ }
      };
      ws.onclose = () => { if (!dead) setTimeout(connect, 3000); };
    };

    connect();
    return () => { dead = true; ws?.close(); };
  }, []);

  useEffect(() => {
    load();
    if (!autoRefresh) return;
    const id = setInterval(load, 5000);
    return () => clearInterval(id);
  }, [load, autoRefresh]);

  return (
    <div className="space-y-6 max-w-6xl">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold gradient-text flex items-center gap-2">
            <BarChart3 className="h-6 w-6" /> Analytics & Tracing
          </h1>
          <p className="text-sm text-muted-foreground mt-1">
            Live latency percentiles, throughput, and execution traces
          </p>
        </div>
        <div className="flex items-center gap-3">
          {lastUpdated && (
            <span className="text-xs text-muted-foreground">
              Updated {lastUpdated.toLocaleTimeString()}
            </span>
          )}
          <button
            onClick={() => setAutoRefresh(r => !r)}
            className={`px-3 py-1.5 rounded-lg text-xs font-medium border transition-colors ${
              autoRefresh ? "bg-green-900/40 border-green-700 text-green-400" : "bg-secondary border-border text-muted-foreground"
            }`}
          >
            {autoRefresh ? "● Live" : "Paused"}
          </button>
          <button
            onClick={load}
            className="p-2 rounded-lg bg-secondary hover:bg-secondary/80 transition-colors"
          >
            <RefreshCw className="h-4 w-4" />
          </button>
        </div>
      </div>

      {/* Live Metrics Grid */}
      {loading && !metrics ? (
        <div className="text-center py-8 text-muted-foreground">Loading metrics…</div>
      ) : metrics ? (
        <>
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            <MetricCard
              label="P50 Latency"
              value={fmtUs(metrics.p50_us)}
              icon={Clock}
              color="text-blue-400"
              history={p50History.current}
            />
            <MetricCard
              label="P95 Latency"
              value={fmtUs(metrics.p95_us)}
              sub={`P99: ${fmtUs(metrics.p99_us)}`}
              icon={Activity}
              color="text-yellow-400"
              history={p95History.current}
            />
            <MetricCard
              label="Throughput"
              value={`${metrics.throughput_per_min.toFixed(1)}/min`}
              icon={TrendingUp}
              color="text-green-400"
              history={throughputHistory.current}
            />
            <MetricCard
              label="Success Rate"
              value={pct(metrics.success_rate)}
              sub={`Error rate: ${pct(metrics.error_rate)}`}
              icon={Zap}
              color={metrics.error_rate > 0.1 ? "text-red-400" : "text-green-400"}
            />
          </div>

          {/* Avg latency bar */}
          <div className="rounded-xl border border-border bg-card p-4">
            <p className="text-xs text-muted-foreground uppercase tracking-wider mb-3">Latency Distribution</p>
            <div className="space-y-2">
              {[
                { label: "P50", val: metrics.p50_us, max: metrics.p99_us, color: "bg-blue-500" },
                { label: "P95", val: metrics.p95_us, max: metrics.p99_us, color: "bg-yellow-500" },
                { label: "P99", val: metrics.p99_us, max: metrics.p99_us, color: "bg-red-500" },
                { label: "Avg", val: metrics.avg_us, max: metrics.p99_us, color: "bg-purple-500" },
              ].map(({ label, val, max, color }) => (
                <div key={label} className="flex items-center gap-3">
                  <span className="text-xs text-muted-foreground w-8">{label}</span>
                  <div className="flex-1 h-2 rounded-full bg-gray-800">
                    <div
                      className={`h-2 rounded-full ${color} transition-all duration-500`}
                      style={{ width: `${max > 0 ? Math.min(100, (val / max) * 100) : 0}%` }}
                    />
                  </div>
                  <span className="text-xs font-mono w-20 text-right">{fmtUs(val)}</span>
                </div>
              ))}
            </div>
          </div>
        </>
      ) : (
        <div className="rounded-xl border border-yellow-700 bg-yellow-950/30 p-4 text-yellow-300 text-sm">
          No live metrics available yet — execute some tasks to generate trace data.
        </div>
      )}

      {/* Scheduler Status */}
      {scheduler && (
        <div className="rounded-xl border border-border bg-card p-4">
          <p className="text-xs text-muted-foreground uppercase tracking-wider mb-3 flex items-center gap-2">
            <Zap className="h-3.5 w-3.5" /> Scheduler Status
          </p>
          <div className="grid grid-cols-2 md:grid-cols-5 gap-4">
            {[
              { label: "Queued", val: scheduler.queued },
              { label: "Running", val: scheduler.running },
              { label: "Max Concurrent", val: scheduler.max_concurrent },
              { label: "Time Slice", val: `${scheduler.slice_ms}ms` },
              { label: "Timeout", val: `${scheduler.timeout_secs}s` },
            ].map(({ label, val }) => (
              <div key={label} className="text-center">
                <p className="text-xl font-bold">{val}</p>
                <p className="text-xs text-muted-foreground">{label}</p>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Trace table */}
      <div>
        <h2 className="text-base font-semibold mb-3 flex items-center gap-2">
          <Activity className="h-4 w-4 text-primary" /> Recent Execution Traces
        </h2>
        {traces.length === 0 ? (
          <div className="rounded-xl border border-border bg-card p-8 text-center text-muted-foreground">
            No traces recorded yet.
          </div>
        ) : (
          <div className="rounded-xl border border-border bg-card overflow-hidden">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-border bg-muted/30">
                  <th className="px-4 py-2 text-left text-xs text-muted-foreground font-medium">Trace ID</th>
                  <th className="px-4 py-2 text-left text-xs text-muted-foreground font-medium">Task</th>
                  <th className="px-4 py-2 text-left text-xs text-muted-foreground font-medium">Status</th>
                  <th className="px-4 py-2 text-left text-xs text-muted-foreground font-medium">Duration</th>
                  <th className="px-4 py-2 text-left text-xs text-muted-foreground font-medium">Spans</th>
                </tr>
              </thead>
              <tbody>
                {traces.map((trace) => (
                  <tr key={trace.trace_id} className="border-b border-border/50 hover:bg-muted/20 transition-colors">
                    <td className="px-4 py-2 font-mono text-xs text-muted-foreground">
                      {trace.trace_id.slice(0, 12)}…
                    </td>
                    <td className="px-4 py-2 font-mono text-xs" title={trace.task_id}>{trace.task_name || trace.task_id.slice(0, 12)}</td>
                    <td className="px-4 py-2">
                      <span className={`px-2 py-0.5 rounded-full text-xs border ${
                        trace.success
                          ? "bg-green-900/40 text-green-400 border-green-700"
                          : "bg-red-900/40 text-red-400 border-red-700"
                      }`}>
                        {trace.success ? "OK" : "FAIL"}
                      </span>
                    </td>
                    <td className="px-4 py-2 text-xs font-mono">{trace.total_duration_us != null ? fmtUs(trace.total_duration_us) : "—"}</td>
                    <td className="px-4 py-2 text-xs text-muted-foreground">{trace.spans.length}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
}
