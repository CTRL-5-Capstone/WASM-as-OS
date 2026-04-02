"use client";

import { useEffect, useState, useCallback, useRef } from "react";
import {
  Activity, Cpu, HardDrive, Zap, RefreshCw, Wifi, WifiOff,
  BarChart3, Terminal, TrendingUp, TrendingDown, Minus,
  Database, Clock, Package, AlertCircle, CheckCircle2,
} from "lucide-react";
import { getStats, getTasks, healthReady, getPrometheusMetrics,
  type SystemStats, type Task,
} from "@/lib/api";
import { formatNumber, cn } from "@/lib/utils";
import {
  AreaChart, Area, LineChart, Line, BarChart, Bar, Cell,
  XAxis, YAxis, Tooltip, ResponsiveContainer, CartesianGrid,
  ReferenceLine,
} from "recharts";
import { useTerminal, type TermLine } from "@/lib/terminal-context";
import { useWebSocket, type WsTaskEvent } from "@/lib/use-websocket";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Badge } from "@/components/ui/badge";


// ─── Types ─────────────────────────────────────────────────────────

interface Tick {
  t: string;
  running: number;
  total: number;
  failed: number;
  instructions: number;
  syscalls: number;
  // derived deltas
  instrDelta: number;
  syscallDelta: number;
}

interface TaskRow extends Task {
  trend: "up" | "down" | "flat";
}

// ─── Custom Tooltip ─────────────────────────────────────────────────

function ChartTip({ active, payload, label }: { active?: boolean; payload?: { name: string; value: number; color: string }[]; label?: string }) {
  if (!active || !payload?.length) return null;
  return (
    <div className="rounded-lg border border-border bg-card px-3 py-2 shadow-lg text-xs">
      <p className="text-muted-foreground mb-1 font-mono">{label}</p>
      {payload.map((p) => (
        <p key={p.name} style={{ color: p.color }} className="font-semibold">{p.name}: {typeof p.value === "number" ? p.value.toLocaleString() : p.value}</p>
      ))}
    </div>
  );
}

// ─── Sparkline KPI card ──────────────────────────────────────────────

function KPICard({
  label, value, sub, icon: Icon, color, bg, data, dataKey, trend,
}: {
  label: string; value: string | number; sub?: string;
  icon: React.ElementType; color: string; bg: string;
  data: Tick[]; dataKey: keyof Tick; trend?: "up" | "down" | "flat";
}) {
  const TrendIcon = trend === "up" ? TrendingUp : trend === "down" ? TrendingDown : Minus;
  return (
    <Card className="overflow-hidden">
      <CardContent className="p-4">
        <div className="flex items-start justify-between mb-2">
          <div className={cn("rounded-lg p-2", bg)}>
            <Icon size={15} className={color} />
          </div>
          {trend && (
            <TrendIcon size={12} className={cn(trend === "up" ? "text-emerald-500" : trend === "down" ? "text-red-500" : "text-muted-foreground")} />
          )}
        </div>
        <p className="text-2xl font-bold text-foreground leading-none">{value}</p>
        <p className="text-xs text-muted-foreground mt-0.5">{label}</p>
        {sub && <p className="text-[10px] text-muted-foreground mt-0.5">{sub}</p>}
        <div className="mt-2 -mx-1">
          <ResponsiveContainer width="100%" height={36}>
            <AreaChart data={data} margin={{ left: 0, right: 0, top: 2, bottom: 0 }}>
              <defs>
                <linearGradient id={`grad-${label}`} x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stopColor={color.replace("text-", "")} stopOpacity={0.3} />
                  <stop offset="100%" stopColor={color.replace("text-", "")} stopOpacity={0} />
                </linearGradient>
              </defs>
              <Area type="monotone" dataKey={dataKey as string} stroke="currentColor" fill={`url(#grad-${label})`} strokeWidth={1.5} dot={false} className={color} />
            </AreaChart>
          </ResponsiveContainer>
        </div>
      </CardContent>
    </Card>
  );
}

// ─── Main Page ──────────────────────────────────────────────────────

export default function MonitorPage() {
  const [stats, setStats] = useState<SystemStats | null>(null);
  const [tasks, setTasks] = useState<TaskRow[]>([]);
  const [online, setOnline] = useState(true);
  const [dbReady, setDbReady] = useState(true);
  const [history, setHistory] = useState<Tick[]>([]);
  const [prometheus, setPrometheus] = useState<string>("");
  const [showProm, setShowProm] = useState(false);
  const [termFeed, setTermFeed] = useState<TermLine[]>([]);
  const [wsEvents, setWsEvents] = useState<WsTaskEvent[]>([]);
  const prevStats = useRef<SystemStats | null>(null);
  const refreshRef = useRef<(() => void) | null>(null);
  const { subscribe } = useTerminal();

  // Live WebSocket events
  const { status: wsStatus } = useWebSocket({
    silent: true, // toasts are handled by Navbar
    onEvent: (evt) => {
      setWsEvents((prev) => [evt, ...prev].slice(0, 50));
      // Trigger a fast refresh when a task changes state
      if (["task_completed", "task_failed", "task_started", "task_stopped"].includes(evt.type)) {
        refreshRef.current?.();
      }
    },
  });

  useEffect(() => {
    const unsub = subscribe((line) => {
      setTermFeed((prev) => [...prev.slice(-79), line]);
    });
    return unsub;
  }, [subscribe]);

  const refresh = useCallback(async () => {
    try {
      const [s, t] = await Promise.all([getStats(), getTasks()]);
      setOnline(true);

      // Compute per-tick deltas for instruction/syscall rate
      const prev = prevStats.current;
      const instrDelta = prev ? Math.max(0, s.total_instructions - prev.total_instructions) : 0;
      const syscallDelta = prev ? Math.max(0, s.total_syscalls - prev.total_syscalls) : 0;
      prevStats.current = s;

      setStats(s);

      // Enrich tasks with trend
      setTasks((prev) =>
        (t as Task[]).map((task) => {
          const old = prev.find((p) => p.id === task.id);
          const trend: "up" | "down" | "flat" =
            !old ? "flat"
            : task.status === "running" && old.status !== "running" ? "up"
            : task.status === "failed" && old.status !== "failed" ? "down"
            : "flat";
          return { ...task, trend };
        })
      );

      setHistory((prev) => [
        ...prev.slice(-59),
        {
          t: new Date().toLocaleTimeString("en-US", { hour12: false, hour: "2-digit", minute: "2-digit", second: "2-digit" }),
          running: s.running_tasks,
          total: s.total_tasks,
          failed: s.failed_tasks,
          instructions: s.total_instructions,
          syscalls: s.total_syscalls,
          instrDelta,
          syscallDelta,
        },
      ]);
    } catch {
      setOnline(false);
    }
    try { await healthReady(); setDbReady(true); } catch { setDbReady(false); }
  }, []);

  // Keep refreshRef current so WebSocket callback can call it
  useEffect(() => { refreshRef.current = refresh; }, [refresh]);

  useEffect(() => {
    refresh();
    const id = setInterval(refresh, 2000);
    return () => clearInterval(id);
  }, [refresh]);

  const loadPrometheus = async () => {
    if (!showProm) {
      try { setPrometheus(await getPrometheusMetrics()); } catch { setPrometheus("# Backend offline"); }
    }
    setShowProm((p) => !p);
  };

  const failed  = tasks.filter((t) => t.status === "failed");
  const running = tasks.filter((t) => t.status === "running");
  const done    = tasks.filter((t) => t.status === "completed");

  const latestTick = history[history.length - 1];
  const instrRate = latestTick?.instrDelta ?? 0;
  const syscallRate = latestTick?.syscallDelta ?? 0;

  // Status-bar distribution for BarChart
  const statusDist = [
    { name: "Running",   count: running.length,  fill: "#22c55e" },
    { name: "Completed", count: done.length,      fill: "#6366f1" },
    { name: "Failed",    count: failed.length,    fill: "#ef4444" },
    { name: "Pending",   count: tasks.filter((t) => t.status === "pending").length,  fill: "#94a3b8" },
    { name: "Stopped",   count: tasks.filter((t) => t.status === "stopped").length,  fill: "#f59e0b" },
  ];

  return (
    <div className="animate-fade-in space-y-6">

      {/* ── Header ── */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold gradient-text">Monitor</h1>
          <p className="mt-1 text-sm text-muted-foreground">Real-time system telemetry · 2 s refresh</p>
        </div>
        <div className="flex items-center gap-3">
          <button onClick={refresh} className="flex items-center gap-1.5 rounded-lg border border-border px-3 py-1.5 text-xs text-muted-foreground hover:bg-muted/30 transition-colors">
            <RefreshCw size={12} /> Refresh
          </button>
          <div className="flex items-center gap-1.5 text-xs">
            {online ? <Wifi size={13} className="text-emerald-500" /> : <WifiOff size={13} className="text-red-500" />}
            <span className={online ? "text-emerald-600 font-medium" : "text-red-500 font-medium"}>{online ? "Connected" : "Offline"}</span>
          </div>
          <div className="flex items-center gap-1.5 text-xs">
            <span className={cn("h-2 w-2 rounded-full", dbReady ? "bg-emerald-400 animate-pulse" : "bg-red-400")} />
            <span className={dbReady ? "text-emerald-600 font-medium" : "text-red-500 font-medium"}>DB {dbReady ? "Ready" : "Down"}</span>
          </div>
        </div>
      </div>

      {/* ── KPI strip ── */}
      <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 lg:grid-cols-5">
        <KPICard label="Running" value={stats?.running_tasks ?? 0} icon={Activity} color="text-emerald-500" bg="bg-emerald-500/10" data={history} dataKey="running" trend={running.length > 0 ? "up" : "flat"} />
        <KPICard label="Total Tasks" value={stats?.total_tasks ?? 0} icon={Package} color="text-primary" bg="bg-primary/10" data={history} dataKey="total" />
        <KPICard label="Failed" value={stats?.failed_tasks ?? 0} icon={AlertCircle} color="text-red-500" bg="bg-rose-500/10" data={history} dataKey="failed" trend={failed.length > 0 ? "down" : "flat"} />
        <KPICard label="Instr/tick" value={formatNumber(instrRate)} sub="Instructions executed" icon={Zap} color="text-violet-500" bg="bg-violet-500/10" data={history} dataKey="instrDelta" />
        <KPICard label="Syscalls/tick" value={formatNumber(syscallRate)} sub="Syscalls executed" icon={HardDrive} color="text-amber-500" bg="bg-amber-500/10" data={history} dataKey="syscallDelta" />
      </div>

      {/* ── Main charts ── */}
      <div className="grid gap-6 lg:grid-cols-2">

        {/* Running tasks timeline */}
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-semibold text-foreground flex items-center gap-2">
              <Activity size={14} className="text-emerald-500" /> Running Tasks Timeline
            </CardTitle>
          </CardHeader>
          <CardContent>
            <ResponsiveContainer width="100%" height={180}>
              <AreaChart data={history} margin={{ left: -20, right: 4 }}>
                <defs>
                  <linearGradient id="runGrad" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="0%" stopColor="#22c55e" stopOpacity={0.25} />
                    <stop offset="100%" stopColor="#22c55e" stopOpacity={0} />
                  </linearGradient>
                </defs>
                <CartesianGrid strokeDasharray="3 3" stroke="#1e293b" />
                <XAxis dataKey="t" tick={{ fill: "#94a3b8", fontSize: 9 }} interval="preserveStartEnd" />
                <YAxis tick={{ fill: "#94a3b8", fontSize: 9 }} allowDecimals={false} />
                <Tooltip content={<ChartTip />} />
                <Area type="monotone" dataKey="running" name="Running" stroke="#22c55e" fill="url(#runGrad)" strokeWidth={2} dot={false} />
                <Area type="monotone" dataKey="failed" name="Failed" stroke="#ef4444" fill="transparent" strokeWidth={1.5} strokeDasharray="4 2" dot={false} />
              </AreaChart>
            </ResponsiveContainer>
          </CardContent>
        </Card>

        {/* Status distribution */}
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-semibold text-foreground flex items-center gap-2">
              <BarChart3 size={14} className="text-primary" /> Task Status Distribution
            </CardTitle>
          </CardHeader>
          <CardContent>
            <ResponsiveContainer width="100%" height={180}>
              <BarChart data={statusDist} margin={{ left: -20, right: 4 }}>
                <CartesianGrid strokeDasharray="3 3" stroke="#1e293b" />
                <XAxis dataKey="name" tick={{ fill: "#94a3b8", fontSize: 9 }} />
                <YAxis tick={{ fill: "#94a3b8", fontSize: 9 }} allowDecimals={false} />
                <Tooltip content={<ChartTip />} />
                <Bar dataKey="count" name="Tasks" radius={[4, 4, 0, 0]}>
                  {statusDist.map((entry, i) => (
                    <Cell key={i} fill={entry.fill} />
                  ))}
                </Bar>
              </BarChart>
            </ResponsiveContainer>
          </CardContent>
        </Card>

        {/* Instruction rate */}
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-semibold text-foreground flex items-center gap-2">
              <Zap size={14} className="text-violet-500" /> Instruction Rate (per tick)
            </CardTitle>
          </CardHeader>
          <CardContent>
            <ResponsiveContainer width="100%" height={180}>
              <LineChart data={history} margin={{ left: -20, right: 4 }}>
                <CartesianGrid strokeDasharray="3 3" stroke="#1e293b" />
                <XAxis dataKey="t" tick={{ fill: "#94a3b8", fontSize: 9 }} interval="preserveStartEnd" />
                <YAxis tick={{ fill: "#94a3b8", fontSize: 9 }} />
                <Tooltip content={<ChartTip />} />
                <ReferenceLine y={0} stroke="#334155" />
                <Line type="monotone" dataKey="instrDelta" name="Instructions/tick" stroke="#8b5cf6" strokeWidth={2} dot={false} />
              </LineChart>
            </ResponsiveContainer>
          </CardContent>
        </Card>

        {/* Syscall rate */}
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-semibold text-foreground flex items-center gap-2">
              <HardDrive size={14} className="text-amber-500" /> Syscall Rate (per tick)
            </CardTitle>
          </CardHeader>
          <CardContent>
            <ResponsiveContainer width="100%" height={180}>
              <AreaChart data={history} margin={{ left: -20, right: 4 }}>
                <defs>
                  <linearGradient id="sysGrad" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="0%" stopColor="#f59e0b" stopOpacity={0.25} />
                    <stop offset="100%" stopColor="#f59e0b" stopOpacity={0} />
                  </linearGradient>
                </defs>
                <CartesianGrid strokeDasharray="3 3" stroke="#1e293b" />
                <XAxis dataKey="t" tick={{ fill: "#94a3b8", fontSize: 9 }} interval="preserveStartEnd" />
                <YAxis tick={{ fill: "#94a3b8", fontSize: 9 }} />
                <Tooltip content={<ChartTip />} />
                <Area type="monotone" dataKey="syscallDelta" name="Syscalls/tick" stroke="#f59e0b" fill="url(#sysGrad)" strokeWidth={2} dot={false} />
              </AreaChart>
            </ResponsiveContainer>
          </CardContent>
        </Card>
      </div>

      {/* ── Live task list ── */}
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-sm font-semibold text-foreground flex items-center gap-2">
            <RefreshCw size={13} className="text-sky-500 animate-spin" /> Live Module Status
            <span className="ml-auto text-xs font-normal text-muted-foreground">{tasks.length} modules</span>
          </CardTitle>
        </CardHeader>
        <CardContent>
          {tasks.length === 0 ? (
            <p className="text-sm text-muted-foreground py-4 text-center">No modules loaded — upload a WASM file via Tasks</p>
          ) : (
            <div className="grid gap-2 sm:grid-cols-2 xl:grid-cols-3">
              {tasks.map((t) => (
                <div key={t.id} className={cn(
                  "flex items-center gap-3 rounded-xl border px-3 py-2.5 transition-all",
                  t.status === "running"   ? "border-emerald-500/30 bg-emerald-500/10/50" :
                  t.status === "failed"    ? "border-rose-500/30 bg-rose-500/10/50" :
                  t.status === "completed" ? "border-primary/20 bg-primary/10/30" :
                  "border-border bg-muted/30"
                )}>
                  <span className={cn("h-2.5 w-2.5 rounded-full shrink-0",
                    t.status === "running"   ? "bg-emerald-400 animate-pulse" :
                    t.status === "failed"    ? "bg-red-400" :
                    t.status === "completed" ? "bg-indigo-400" :
                    t.status === "pending"   ? "bg-amber-400" : "bg-slate-400"
                  )} />
                  <div className="min-w-0 flex-1">
                    <p className="truncate text-sm font-medium text-foreground">{t.name}</p>
                    <p className="text-[10px] font-mono text-muted-foreground">{t.id.slice(0, 12)}</p>
                  </div>
                  <Badge variant="outline" className={cn("text-[10px] shrink-0",
                    t.status === "running"   ? "border-emerald-300 text-emerald-400" :
                    t.status === "failed"    ? "border-red-300 text-rose-400" :
                    t.status === "completed" ? "border-primary/30 text-primary" :
                    "border-border text-muted-foreground"
                  )}>
                    {t.status}
                  </Badge>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      {/* ── Failed tasks panel ── */}
      {failed.length > 0 && (
        <Card className="border-rose-500/30">
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-semibold text-rose-400 flex items-center gap-2">
              <AlertCircle size={14} /> Failed Modules ({failed.length})
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-1.5">
              {failed.map((t) => (
                <div key={t.id} className="flex items-center gap-2 text-sm">
                  <span className="h-1.5 w-1.5 rounded-full bg-red-400 shrink-0" />
                  <span className="font-medium text-foreground">{t.name}</span>
                  <span className="font-mono text-xs text-muted-foreground">{t.id.slice(0, 10)}</span>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      )}

      {/* ── Prometheus raw metrics ── */}
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-sm font-semibold text-foreground flex items-center gap-2">
            <Database size={14} className="text-muted-foreground" /> Prometheus Metrics
            <button
              onClick={loadPrometheus}
              className="ml-auto text-xs text-indigo-600 hover:underline font-normal"
            >
              {showProm ? "Hide" : "Load"} raw metrics
            </button>
          </CardTitle>
        </CardHeader>
        {showProm && (
          <CardContent>
            <ScrollArea className="h-64 rounded-lg border border-border bg-slate-950">
              <pre className="p-3 font-mono text-[11px] text-emerald-400 whitespace-pre-wrap">
                {prometheus || "# loading…"}
              </pre>
            </ScrollArea>
          </CardContent>
        )}
      </Card>

      {/* ── Live WebSocket Events ── */}
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-sm font-semibold text-foreground flex items-center gap-2">
            <Wifi size={14} className={cn(
              wsStatus === "connected" ? "text-emerald-500 animate-pulse" : "text-muted-foreground"
            )} />
            Live Task Events
            <span className={cn(
              "ml-1 text-[10px] font-semibold rounded-full px-2 py-0.5",
              wsStatus === "connected" ? "bg-emerald-500/15 text-emerald-400"
              : wsStatus === "connecting" ? "bg-amber-500/15 text-amber-400"
              : "bg-muted text-muted-foreground"
            )}>
              {wsStatus}
            </span>
            <span className="ml-auto text-xs font-normal text-muted-foreground">{wsEvents.length} events</span>
          </CardTitle>
        </CardHeader>
        <CardContent>
          {wsEvents.length === 0 ? (
            <p className="text-sm text-muted-foreground">
              No live events yet — WebSocket {wsStatus === "connected" ? "connected, waiting for task activity" : `(${wsStatus})`}
            </p>
          ) : (
            <ScrollArea className="h-48 rounded-lg border border-border">
              <div className="divide-y divide-border">
                {wsEvents.map((evt, i) => (
                  <div key={i} className="flex items-start gap-3 px-3 py-2 text-xs hover:bg-muted/30">
                    <span className={cn(
                      "w-1.5 h-1.5 rounded-full shrink-0 mt-1.5",
                      evt.type === "task_completed" ? "bg-emerald-400"
                      : evt.type === "task_failed"   ? "bg-red-400"
                      : evt.type === "task_started"  ? "bg-sky-400"
                      : evt.type === "task_stopped"  ? "bg-amber-400"
                      : "bg-slate-300"
                    )} />
                    <div className="flex-1 min-w-0">
                      <span className={cn(
                        "font-semibold",
                        evt.type === "task_completed" ? "text-emerald-400"
                        : evt.type === "task_failed"   ? "text-rose-400"
                        : evt.type === "task_started"  ? "text-sky-700"
                        : "text-foreground"
                      )}>
                        {evt.type.replace("task_", "")}
                      </span>
                      {evt.task_name && (
                        <span className="ml-2 text-muted-foreground">{evt.task_name}</span>
                      )}
                      {evt.error && (
                        <span className="ml-2 text-red-500 truncate">{evt.error}</span>
                      )}
                    </div>
                    {evt.timestamp && (
                      <span className="text-muted-foreground whitespace-nowrap">
                        {new Date(evt.timestamp).toLocaleTimeString()}
                      </span>
                    )}
                  </div>
                ))}
              </div>
            </ScrollArea>
          )}
        </CardContent>
      </Card>

      {/* ── Terminal activity feed ── */}
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-sm font-semibold text-foreground flex items-center gap-2">
            <Terminal size={14} className="text-emerald-500" /> Terminal Activity Feed
            <span className="ml-auto text-xs font-normal text-muted-foreground">{termFeed.length} lines</span>
          </CardTitle>
        </CardHeader>
        <CardContent>
          {termFeed.length === 0 ? (
            <p className="text-sm text-muted-foreground">No terminal activity yet — run commands in the Terminal or Command Center CLI.</p>
          ) : (
            <ScrollArea className="h-56 rounded-lg border border-border bg-slate-950">
              <div className="p-3 font-mono text-xs space-y-0.5">
                {termFeed.map((line) => (
                  <div key={line.id} className={cn("whitespace-pre-wrap break-words",
                    line.type === "input"  ? "text-sky-400" :
                    line.type === "error"  ? "text-red-400" :
                    line.type === "system" ? "text-amber-400" :
                    line.type === "table"  ? "text-emerald-300" : "text-muted-foreground"
                  )}>
                    <span className="text-muted-foreground mr-2 select-none">{new Date(line.ts).toLocaleTimeString()}</span>
                    {line.text}
                  </div>
                ))}
              </div>
            </ScrollArea>
          )}
        </CardContent>
      </Card>

    </div>
  );
}

