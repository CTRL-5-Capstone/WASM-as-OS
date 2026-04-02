"use client";

/**
 * Command Center — the main high-density dashboard.
 * Layout: ResizablePanelGroup with live metrics (top), loaded modules (mid), xterm CLI (bottom).
 * Uses Recharts for streaming charts, Sonner for security toasts, xterm.js for CLI.
 */

import { useEffect, useState, useRef, useCallback } from "react";
import { Group, Panel, Separator } from "react-resizable-panels";
import {
  AreaChart, Area, LineChart, Line, BarChart, Bar,
  XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, Legend,
} from "recharts";
import {
  ShieldAlert, Cpu, MemoryStick, Activity, Zap,
  RefreshCw, CheckCircle, AlertCircle, FileCode,
  Clock, Terminal, Maximize2, Minimize2, BarChart3,
} from "lucide-react";
import { getTasks, getStats, checkHealth, type Task, type SystemStats } from "@/lib/api";
import { useSecurityAlert } from "@/lib/security-alerts";
import { cn, formatBytes, formatNumber } from "@/lib/utils";
import dynamic from "next/dynamic";

const XTerminal = dynamic(() => import("@/components/XTerminal"), { ssr: false });

// ─── Metric data point ──────────────────────────────────────────────

interface MetricPoint {
  t: string;
  cpu: number;
  mem: number;
  syscalls: number;
  tasks: number;
  ops: number;
}

function nowLabel() {
  const d = new Date();
  return `${d.getHours().toString().padStart(2, "0")}:${d.getMinutes().toString().padStart(2, "0")}:${d.getSeconds().toString().padStart(2, "0")}`;
}

function statusColor(s: string) {
  switch (s?.toLowerCase()) {
    case "running": return "text-emerald-400";
    case "completed": return "text-sky-400";
    case "failed": return "text-red-400";
    case "stopped": return "text-amber-400";
    default: return "text-slate-400";
  }
}

function statusDot(s: string) {
  switch (s?.toLowerCase()) {
    case "running": return "bg-emerald-400 animate-pulse";
    case "completed": return "bg-sky-400";
    case "failed": return "bg-red-400";
    default: return "bg-slate-500";
  }
}

// ─── CHART TOOLTIP ───────────────────────────────────────────────────

const ChartTooltip = ({ active, payload, label }: { active?: boolean; payload?: Array<{name: string; value: number; color: string}>; label?: string }) => {
  if (!active || !payload?.length) return null;
  return (
    <div className="rounded-lg border border-slate-700 bg-slate-900/95 px-3 py-2 text-xs shadow-lg">
      <p className="text-slate-400 mb-1 font-mono">{label}</p>
      {payload.map((p) => (
        <p key={p.name} style={{ color: p.color }}>
          {p.name}: <strong>{p.value}</strong>
        </p>
      ))}
    </div>
  );
};

// ─── MAIN COMPONENT ───────────────────────────────────────────────────

export default function CommandCenterPage() {
  const { alert: secAlert, hazardous } = useSecurityAlert();

  const [stats, setStats] = useState<SystemStats | null>(null);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [health, setHealth] = useState<"ok" | "error" | "loading">("loading");
  const [metrics, setMetrics] = useState<MetricPoint[]>([]);
  const [cliExpanded, setCliExpanded] = useState(false);

  const prevInstr = useRef(0);
  const prevSys = useRef(0);

  // ─── Poll backend every 2s ──────────────────────────────────────

  const tick = useCallback(async () => {
    try {
      const [s, t] = await Promise.all([getStats(), getTasks()]);
      setStats(s);
      setTasks(t);
      setHealth("ok");

      const opsPerSec = s.total_instructions - prevInstr.current;
      const sysPerSec = s.total_syscalls - prevSys.current;
      prevInstr.current = s.total_instructions;
      prevSys.current = s.total_syscalls;

      // Simulate CPU% and memory from available metrics (normalised)
      const cpuEst = Math.min(100, Math.round((opsPerSec / 1_000_000) * 10 + s.running_tasks * 5));
      const memEst = Math.min(1024, Math.round(s.total_tasks * 12 + s.running_tasks * 48));

      setMetrics((prev) => {
        const next = [
          ...prev.slice(-59),
          {
            t: nowLabel(),
            cpu: cpuEst,
            mem: memEst,
            syscalls: Math.max(0, sysPerSec),
            tasks: s.total_tasks,
            ops: Math.max(0, opsPerSec),
          },
        ];
        return next;
      });

      // Check for failed tasks and fire alerts
      const newFailed = t.filter((tk) => tk.status === "failed");
      newFailed.forEach((tk) => {
        // Only alert once per task
        const alertedKey = `alerted_${tk.id}`;
        if (!sessionStorage.getItem(alertedKey)) {
          sessionStorage.setItem(alertedKey, "1");
          secAlert(`Task "${tk.name}" failed execution`, "warn");
        }
      });
    } catch {
      setHealth("error");
    }
  }, [secAlert]);

  useEffect(() => {
    tick();
    const id = setInterval(tick, 2000);
    return () => clearInterval(id);
  }, [tick]);

  // ─── Health check ──────────────────────────────────────────────

  useEffect(() => {
    checkHealth().then(() => setHealth("ok")).catch(() => setHealth("error"));
  }, []);

  // ─── Security alert callback for XTerminal ─────────────────────

  const handleTermAlert = useCallback((msg: string, level: "warn" | "severe") => {
    secAlert(msg, level);
    if (level === "severe") hazardous("CLI-reported", msg);
  }, [secAlert, hazardous]);

  const counts = tasks.reduce<Record<string, number>>((a, t) => { a[t.status] = (a[t.status] || 0) + 1; return a; }, {});

  return (
    <div className="h-[calc(100vh-80px)] flex flex-col gap-0 animate-fade-in">
      {/* ── Header bar ── */}
      <div className="flex items-center justify-between px-1 pb-3">
        <div>
          <h1 className="text-2xl font-bold gradient-text flex items-center gap-2">
            <BarChart3 size={22} /> Command Center
          </h1>
          <p className="text-xs text-slate-500 mt-0.5">Live telemetry · WebSocket CLI · Security monitoring</p>
        </div>
        <div className="flex items-center gap-3 text-xs">
          <div className={cn("flex items-center gap-1.5 rounded-full px-3 py-1 border font-medium",
            health === "ok"
              ? "bg-emerald-500/10 border-emerald-500/30 text-emerald-600"
              : health === "error"
                ? "bg-red-500/10 border-red-500/30 text-red-600"
                : "bg-slate-100 border-slate-200 text-slate-500"
          )}>
            <span className={cn("w-2 h-2 rounded-full", health === "ok" ? "bg-emerald-400 animate-pulse" : health === "error" ? "bg-red-400" : "bg-slate-400")} />
            {health === "ok" ? "Backend Online" : health === "error" ? "Backend Offline" : "Connecting…"}
          </div>
          <button
            onClick={tick}
            className="flex items-center gap-1 text-slate-500 hover:text-slate-700 transition-colors"
          >
            <RefreshCw size={13} /> Refresh
          </button>
        </div>
      </div>

      {/* ── KPI strip ── */}
      <div className="grid grid-cols-5 gap-2 mb-3">
        {[
          { label: "Total Modules", val: stats?.total_tasks ?? 0, icon: FileCode, color: "text-indigo-600", bg: "bg-indigo-50" },
          { label: "Running", val: counts["running"] ?? 0, icon: Activity, color: "text-emerald-600", bg: "bg-emerald-50" },
          { label: "Failed", val: counts["failed"] ?? 0, icon: AlertCircle, color: "text-red-600", bg: "bg-red-50" },
          { label: "Total Instructions", val: formatNumber(stats?.total_instructions ?? 0), icon: Cpu, color: "text-violet-600", bg: "bg-violet-50" },
          { label: "Total Syscalls", val: formatNumber(stats?.total_syscalls ?? 0), icon: Zap, color: "text-amber-600", bg: "bg-amber-50" },
        ].map(({ label, val, icon: Icon, color, bg }) => (
          <div key={label} className={cn("rounded-xl border border-slate-200 p-3 flex items-center gap-3", bg)}>
            <div className={cn("rounded-lg p-2", bg)}>
              <Icon size={16} className={color} />
            </div>
            <div>
              <p className={cn("text-lg font-bold", color)}>{val}</p>
              <p className="text-[11px] text-slate-500">{label}</p>
            </div>
          </div>
        ))}
      </div>

      {/* ── Resizable main area ── */}
      <div className="flex-1 min-h-0">
        <Group orientation="vertical" className="h-full gap-0">
          {/* Top: Charts + Module list */}
          <Panel defaultSize={55} minSize={30}>
            <Group orientation="horizontal" className="h-full">
              {/* Left: Charts */}
              <Panel defaultSize={65} minSize={35}>
                <div className="h-full bg-white rounded-xl border border-slate-200 overflow-hidden flex flex-col">
                  <div className="px-4 pt-3 pb-1 flex items-center justify-between border-b border-slate-100">
                    <h2 className="text-sm font-semibold text-slate-700 flex items-center gap-1.5"><Activity size={13} className="text-indigo-500" /> Live Telemetry</h2>
                    <span className="text-[10px] text-slate-400">2s refresh · last 60s</span>
                  </div>
                  <div className="flex-1 grid grid-rows-3 gap-0 p-2 min-h-0">
                    {/* CPU */}
                    <div className="min-h-0">
                      <p className="text-[10px] text-slate-500 px-2 mb-0.5 flex items-center gap-1"><Cpu size={10} /> CPU Load %</p>
                      <ResponsiveContainer width="100%" height="80%">
                        <AreaChart data={metrics} margin={{ left: -20, right: 4 }}>
                          <defs>
                            <linearGradient id="cpuGrad" x1="0" y1="0" x2="0" y2="1">
                              <stop offset="5%" stopColor="#6366f1" stopOpacity={0.3} />
                              <stop offset="95%" stopColor="#6366f1" stopOpacity={0} />
                            </linearGradient>
                          </defs>
                          <CartesianGrid strokeDasharray="3 3" stroke="#f1f5f9" />
                          <XAxis dataKey="t" tick={{ fontSize: 9, fill: "#94a3b8" }} interval="preserveStartEnd" />
                          <YAxis tick={{ fontSize: 9, fill: "#94a3b8" }} domain={[0, 100]} />
                          <Tooltip content={<ChartTooltip />} />
                          <Area type="monotone" dataKey="cpu" name="CPU%" stroke="#6366f1" fill="url(#cpuGrad)" strokeWidth={1.5} dot={false} />
                        </AreaChart>
                      </ResponsiveContainer>
                    </div>
                    {/* Memory */}
                    <div className="min-h-0">
                      <p className="text-[10px] text-slate-500 px-2 mb-0.5 flex items-center gap-1"><MemoryStick size={10} /> Memory Usage (MB)</p>
                      <ResponsiveContainer width="100%" height="80%">
                        <AreaChart data={metrics} margin={{ left: -20, right: 4 }}>
                          <defs>
                            <linearGradient id="memGrad" x1="0" y1="0" x2="0" y2="1">
                              <stop offset="5%" stopColor="#10b981" stopOpacity={0.3} />
                              <stop offset="95%" stopColor="#10b981" stopOpacity={0} />
                            </linearGradient>
                          </defs>
                          <CartesianGrid strokeDasharray="3 3" stroke="#f1f5f9" />
                          <XAxis dataKey="t" tick={{ fontSize: 9, fill: "#94a3b8" }} interval="preserveStartEnd" />
                          <YAxis tick={{ fontSize: 9, fill: "#94a3b8" }} />
                          <Tooltip content={<ChartTooltip />} />
                          <Area type="monotone" dataKey="mem" name="Mem(MB)" stroke="#10b981" fill="url(#memGrad)" strokeWidth={1.5} dot={false} />
                        </AreaChart>
                      </ResponsiveContainer>
                    </div>
                    {/* Syscall frequency */}
                    <div className="min-h-0">
                      <p className="text-[10px] text-slate-500 px-2 mb-0.5 flex items-center gap-1"><Zap size={10} /> Syscall Frequency</p>
                      <ResponsiveContainer width="100%" height="80%">
                        <LineChart data={metrics} margin={{ left: -20, right: 4 }}>
                          <CartesianGrid strokeDasharray="3 3" stroke="#f1f5f9" />
                          <XAxis dataKey="t" tick={{ fontSize: 9, fill: "#94a3b8" }} interval="preserveStartEnd" />
                          <YAxis tick={{ fontSize: 9, fill: "#94a3b8" }} />
                          <Tooltip content={<ChartTooltip />} />
                          <Line type="monotone" dataKey="syscalls" name="Syscalls/s" stroke="#f59e0b" strokeWidth={1.5} dot={false} />
                        </LineChart>
                      </ResponsiveContainer>
                    </div>
                  </div>
                </div>
              </Panel>

              <Separator className="w-2 flex items-center justify-center cursor-col-resize group">
                <div className="w-0.5 h-12 rounded bg-slate-200 group-hover:bg-indigo-400 transition-colors" />
              </Separator>

              {/* Right: Module list */}
              <Panel defaultSize={35} minSize={20}>
                <div className="h-full bg-white rounded-xl border border-slate-200 flex flex-col overflow-hidden">
                  <div className="px-4 pt-3 pb-2 border-b border-slate-100 flex items-center justify-between">
                    <h2 className="text-sm font-semibold text-slate-700 flex items-center gap-1.5">
                      <FileCode size={13} className="text-indigo-500" /> Loaded Modules
                    </h2>
                    <span className="text-[10px] text-slate-400">{tasks.length} total</span>
                  </div>
                  <div className="flex-1 overflow-y-auto">
                    {tasks.length === 0 ? (
                      <div className="p-6 text-center text-slate-400 text-xs">No modules loaded</div>
                    ) : (
                      tasks.slice(0, 50).map((t) => (
                        <div key={t.id} className="flex items-center gap-2 px-3 py-2 border-b border-slate-50 hover:bg-slate-50 transition-colors text-xs group">
                          <span className={cn("w-1.5 h-1.5 rounded-full shrink-0", statusDot(t.status))} />
                          <div className="min-w-0 flex-1">
                            <p className="text-slate-800 font-medium truncate">{t.name}</p>
                            <p className="text-slate-400 font-mono text-[10px]">{t.id.slice(0, 12)}</p>
                          </div>
                          <span className={cn("text-[10px] font-semibold shrink-0", statusColor(t.status))}>{t.status}</span>
                        </div>
                      ))
                    )}
                  </div>
                </div>
              </Panel>
            </Group>
          </Panel>

          <Separator className="h-2 flex items-center justify-center cursor-row-resize group my-1">
            <div className="h-0.5 w-16 rounded bg-slate-200 group-hover:bg-indigo-400 transition-colors" />
          </Separator>

          {/* Bottom: xterm.js CLI */}
          <Panel defaultSize={45} minSize={20}>
            <div className="h-full bg-slate-950 rounded-xl border border-slate-700 flex flex-col overflow-hidden">
              <div className="flex items-center justify-between px-4 py-2 border-b border-slate-800 shrink-0">
                <div className="flex items-center gap-2">
                  <Terminal size={14} className="text-indigo-400" />
                  <span className="text-xs font-mono font-semibold text-slate-300">Web-CLI</span>
                  <span className="text-[10px] text-slate-500 bg-slate-800 rounded px-1.5 py-0.5">xterm.js · WebSocket</span>
                  <span className="text-[10px] text-slate-500">Pipe support: cmd1 | cmd2</span>
                </div>
                <button
                  onClick={() => setCliExpanded((p) => !p)}
                  className="text-slate-500 hover:text-slate-300 transition-colors"
                  title={cliExpanded ? "Collapse" : "Expand"}
                >
                  {cliExpanded ? <Minimize2 size={13} /> : <Maximize2 size={13} />}
                </button>
              </div>
              <div className="flex-1 min-h-0 p-1">
                <XTerminal className="h-full" onSecurityAlert={handleTermAlert} />
              </div>
            </div>
          </Panel>
        </Group>
      </div>
    </div>
  );
}
