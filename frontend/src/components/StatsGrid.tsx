"use client";

import { useEffect, useState, useCallback } from "react";
import { getStats, type SystemStats } from "@/lib/api";
import { useWebSocket } from "@/lib/use-websocket";
import { formatNumber } from "@/lib/utils";
import {
  ListTodo,
  Play,
  XCircle,
  Cpu,
  Terminal as TermIcon,
  CheckCircle2,
} from "lucide-react";

const statCards = [
  {
    key: "total_tasks" as keyof SystemStats,
    label: "Total Tasks",
    icon: ListTodo,
    accent: "text-sky-400",
    glow: "shadow-sky-500/10",
  },
  {
    key: "running_tasks" as keyof SystemStats,
    label: "Running",
    icon: Play,
    accent: "text-emerald-400",
    glow: "shadow-emerald-500/10",
    pulse: true,
  },
  {
    key: "completed_tasks" as keyof SystemStats,
    label: "Completed",
    icon: CheckCircle2,
    accent: "text-blue-400",
    glow: "shadow-blue-500/10",
  },
  {
    key: "failed_tasks" as keyof SystemStats,
    label: "Failed",
    icon: XCircle,
    accent: "text-rose-400",
    glow: "shadow-rose-500/10",
  },
  {
    key: "total_instructions" as keyof SystemStats,
    label: "Instructions",
    icon: Cpu,
    accent: "text-violet-400",
    glow: "shadow-violet-500/10",
  },
  {
    key: "total_syscalls" as keyof SystemStats,
    label: "Syscalls",
    icon: TermIcon,
    accent: "text-amber-400",
    glow: "shadow-amber-500/10",
  },
];

export default function StatsGrid() {
  const [stats, setStats] = useState<SystemStats | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [flash, setFlash] = useState(false);

  const refresh = useCallback(async () => {
    try {
      const data = await getStats();
      setStats(data);
      setError(null);
    } catch {
      setError("Failed to load stats");
    }
  }, []);

  useEffect(() => {
    refresh();
    const id = setInterval(refresh, 8000);
    return () => clearInterval(id);
  }, [refresh]);

  // Refresh stats whenever a task event fires over WS
  useWebSocket({
    silent: true,
    onEvent: (evt) => {
      const type = evt.type?.toLowerCase();
      if (["task_completed", "task_failed", "task_started", "task_stopped",
           "completed", "failed", "started", "stopped"].includes(type)) {
        setFlash(true);
        setTimeout(() => setFlash(false), 600);
        refresh();
      }
    },
  });

  if (error) {
    return (
      <div className="rounded-lg border border-destructive/30 bg-destructive/10 p-4 text-destructive text-sm flex items-center gap-2">
        {error}
      </div>
    );
  }

  if (!stats) {
    return (
      <div
        role="status"
        aria-busy="true"
        aria-label="Loading system statistics"
        className="grid grid-cols-2 md:grid-cols-3 xl:grid-cols-6 gap-4"
      >
        {Array.from({ length: 6 }).map((_, i) => (
          <div
            key={i}
            className="h-24 rounded-lg border border-border bg-card animate-pulse"
          />
        ))}
        <span className="sr-only">Loading statistics…</span>
      </div>
    );
  }

  return (
    <section
      aria-label="System statistics"
      aria-live="polite"
      aria-atomic="false"
      className={`grid grid-cols-2 md:grid-cols-3 xl:grid-cols-6 gap-4 transition-opacity ${flash ? "opacity-70" : "opacity-100"}`}
    >
      {statCards.map(({ key, label, icon: Icon, accent, glow, pulse }) => {
        const value = stats[key] as number;
        return (
          <div
            key={key}
            role="region"
            aria-label={`${label}: ${formatNumber(value)}`}
            className={`group relative rounded-lg border border-border bg-card p-4 transition-all duration-200 hover:border-primary/30 hover:shadow-lg ${glow}`}
          >
            <div className="flex items-center justify-between mb-3">
              <div className="relative">
                <Icon
                  size={18}
                  strokeWidth={2.25}
                  aria-hidden="true"
                  className={`${accent} transition-transform group-hover:scale-110`}
                />
                {pulse && value > 0 && (
                  <span
                    aria-hidden="true"
                    className="absolute -top-0.5 -right-0.5 h-2 w-2 rounded-full bg-emerald-400 status-dot-running"
                  />
                )}
              </div>
              <span className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
                {label}
              </span>
            </div>
            <p className="text-2xl font-bold tracking-tight text-foreground">
              {formatNumber(value)}
            </p>
          </div>
        );
      })}
    </section>
  );
}
