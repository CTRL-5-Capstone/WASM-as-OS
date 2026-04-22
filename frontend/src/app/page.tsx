"use client";

import StatsGrid from "@/components/StatsGrid";
import TaskList from "@/components/TaskList";
import { useEffect, useState, useCallback } from "react";
import { useTerminal, type TermLine } from "@/lib/terminal-context";
import { Terminal, Zap, RefreshCw, Activity, Clock, Cpu, Layers } from "lucide-react";
import { cn } from "@/lib/utils";
import { getSchedulerStatus, type SchedulerStatus } from "@/lib/api";
import { useWebSocket } from "@/lib/use-websocket";

export default function DashboardPage() {
  const [recentLines, setRecentLines] = useState<TermLine[]>([]);
  const { subscribe } = useTerminal();
  const [scheduler, setScheduler] = useState<SchedulerStatus | null>(null);
  const [schedLoading, setSchedLoading] = useState(true);

  const fetchScheduler = useCallback(async () => {
    try {
      const s = await getSchedulerStatus();
      setScheduler(s);
    } catch {
      // backend may not be running yet
    } finally {
      setSchedLoading(false);
    }
  }, []);

  // refresh scheduler when a task event fires
  useWebSocket({
    silent: true,
    onEvent: fetchScheduler,
  });

  useEffect(() => {
    fetchScheduler();
    const id = setInterval(fetchScheduler, 8000);
    return () => clearInterval(id);
  }, [fetchScheduler]);

  useEffect(() => {
    const unsub = subscribe((line) => {
      setRecentLines((prev) => [...prev.slice(-9), line]);
    });
    return unsub;
  }, [subscribe]);

  return (
    <div className="space-y-8 max-w-7xl animate-fade-in">
      <div className="space-y-1">
        <h1 className="gradient-text text-2xl font-bold tracking-tight">
          Dashboard
        </h1>
        <p className="text-sm text-muted-foreground">
          Overview of your WebAssembly runtime environment
        </p>
      </div>

      <StatsGrid />
      <TaskList />

      {/* Scheduler Status */}
      <div className="rounded-lg border border-border bg-card overflow-hidden animate-fade-in">
        <div className="flex items-center gap-2 px-4 py-3 border-b border-border">
          <Layers size={14} strokeWidth={2.5} className="text-violet-400" />
          <span className="text-sm font-semibold text-foreground">Scheduler Status</span>
          {schedLoading && <RefreshCw size={12} className="animate-spin text-muted-foreground ml-auto" />}
          {!schedLoading && scheduler && (
            <span className={cn(
              "ml-auto text-[11px] font-medium px-2 py-0.5 rounded-full border",
              scheduler.running
                ? "text-green-400 border-green-500/30 bg-green-500/10"
                : "text-yellow-400 border-yellow-500/30 bg-yellow-500/10"
            )}>
              {scheduler.running ? "● Running" : "○ Idle"}
            </span>
          )}
        </div>
        <div className="p-4">
          {!scheduler ? (
            <p className="text-xs text-muted-foreground">
              {schedLoading ? "Loading scheduler info…" : "Scheduler data unavailable."}
            </p>
          ) : (
            <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
              {[
                { label: "Running",       value: scheduler.running ?? 0,       icon: Activity,    color: "text-emerald-400" },
                { label: "Queued",        value: scheduler.queued ?? 0,        icon: Layers,      color: "text-violet-400" },
                { label: "Max Workers",   value: scheduler.max_concurrent ?? 0, icon: Cpu,        color: "text-sky-400" },
                { label: "Slice (ms)",    value: scheduler.slice_ms ?? 0,      icon: Clock,       color: "text-amber-400" },
              ].map(({ label, value, icon: Icon, color }) => (
                <div key={label} className="rounded-lg bg-muted/30 border border-border p-3 flex flex-col gap-1">
                  <div className="flex items-center gap-1.5">
                    <Icon size={12} className={color} />
                    <span className="text-[11px] text-muted-foreground">{label}</span>
                  </div>
                  <p className="text-xl font-bold text-foreground">{value}</p>
                </div>
              ))}
            </div>
          )}
          {scheduler && scheduler.running > 0 && (
            <div className="mt-3 rounded-lg bg-emerald-500/5 border border-emerald-500/20 px-3 py-2 flex items-center gap-2 text-xs">
              <Cpu size={12} className="text-emerald-400 shrink-0" />
              <span className="text-emerald-300 font-mono">{scheduler.running} task(s) currently executing</span>
            </div>
          )}
        </div>
      </div>

      {/* Recent Terminal Activity */}
      <div className="rounded-lg border border-border bg-card overflow-hidden">
        <div className="flex items-center gap-2 px-4 py-3 border-b border-border">
          <Terminal size={14} strokeWidth={2.5} className="text-emerald-400" />
          <span className="text-sm font-semibold text-foreground">
            Recent Terminal Activity
          </span>
          <Zap size={12} strokeWidth={2.5} className="text-amber-400" />
        </div>
        <div className="p-0">
          {recentLines.length === 0 ? (
            <p className="text-xs text-muted-foreground px-4 py-6">
              Terminal commands will appear here in real time.
            </p>
          ) : (
            <div className="h-48 overflow-y-auto bg-background/50">
              <div className="p-3 font-mono text-xs space-y-0.5">
                {recentLines.map((line) => (
                  <div
                    key={line.id}
                    className={cn(
                      "whitespace-pre-wrap break-words",
                      line.type === "input"
                        ? "text-sky-400"
                        : line.type === "error"
                          ? "text-rose-400"
                          : line.type === "system"
                            ? "text-amber-400"
                            : line.type === "table"
                              ? "text-emerald-300"
                              : "text-muted-foreground"
                    )}
                  >
                    {line.text}
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
