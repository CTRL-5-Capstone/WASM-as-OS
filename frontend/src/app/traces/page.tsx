"use client";

import { useEffect, useState, useCallback } from "react";
import { useRouter } from "next/navigation";
import {
  GitBranch, RefreshCw, Search, X, CheckCircle, AlertTriangle,
  Clock, Zap, Activity, ChevronDown, ChevronUp, FileText,
} from "lucide-react";
import {
  listTraces, getLiveMetrics, type TraceRecord, type LiveMetrics,
} from "@/lib/api";
import { formatDuration, timeAgo, cn } from "@/lib/utils";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Skeleton } from "@/components/ui/skeleton";

function LiveMetricsBar({ m }: { m: LiveMetrics }) {
  const items = [
    { label: "Success rate", value: `${(m.success_rate * 100).toFixed(1)}%`, ok: m.success_rate > 0.8 },
    { label: "p50",          value: formatDuration(m.p50_us),  ok: m.p50_us < 1_000_000 },
    { label: "p95",          value: formatDuration(m.p95_us),  ok: m.p95_us < 5_000_000 },
    { label: "p99",          value: formatDuration(m.p99_us),  ok: m.p99_us < 10_000_000 },
    { label: "Avg",          value: formatDuration(m.avg_us),  ok: m.avg_us < 2_000_000 },
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

function TraceRow({ trace }: { trace: TraceRecord }) {
  const [open, setOpen] = useState(false);
  const router = useRouter();
  return (
    <div className="border-b border-border/50">
      <div className="flex items-center">
        <button
          onClick={() => setOpen(!open)}
          className="flex-1 flex items-center gap-3 px-4 py-2.5 text-xs hover:bg-muted/20 transition-colors text-left"
        >
          {trace.success ? (
            <CheckCircle size={13} className="text-green-400 shrink-0" />
          ) : (
            <AlertTriangle size={13} className="text-red-400 shrink-0" />
          )}
          <span className="font-mono text-muted-foreground shrink-0 w-24 truncate" title={trace.trace_id}>
            {trace.trace_id.slice(0, 8)}…
          </span>
          <span className="font-medium flex-1 truncate" title={trace.task_id}>{trace.task_name || trace.task_id.slice(0, 12)}</span>
          <span className="text-muted-foreground shrink-0">{trace.spans.length} spans</span>
          <span className="font-medium shrink-0">{trace.total_duration_us != null ? formatDuration(trace.total_duration_us) : "—"}</span>
          <Badge variant={trace.success ? "default" : "destructive"} className="text-[10px] h-4 px-1.5 shrink-0">
            {trace.success ? "OK" : "FAIL"}
          </Badge>
          {open ? <ChevronUp size={13} className="shrink-0 text-muted-foreground" /> : <ChevronDown size={13} className="shrink-0 text-muted-foreground" />}
        </button>
        {/* Navigate to task detail */}
        <button
          title="View task"
          onClick={() => router.push(`/tasks?task=${trace.task_id}`)}
          className="px-3 py-2.5 text-muted-foreground hover:text-foreground transition-colors shrink-0"
        >
          <FileText size={13} />
        </button>
      </div>

      {open && (
        <div className="px-4 pb-3 bg-muted/10">
          <div className="space-y-1">
            {trace.spans.map((span, i) => {
              const widthPct =
                trace.total_duration_us != null && trace.total_duration_us > 0 &&
                span.duration_us != null
                  ? Math.max(4, Math.round((span.duration_us / trace.total_duration_us) * 100))
                  : 4;
              return (
                <div key={i} className="flex items-center gap-2 text-xs">
                  {span.success ? (
                    <CheckCircle size={11} className="text-green-400 shrink-0" />
                  ) : (
                    <AlertTriangle size={11} className="text-red-400 shrink-0" />
                  )}
                  <span className="w-20 shrink-0 font-medium text-muted-foreground capitalize">{span.kind}</span>
                  <div className="flex-1 h-1.5 bg-muted/40 rounded-full">
                    <div
                      className={cn("h-1.5 rounded-full transition-all", span.success ? "bg-green-400/70" : "bg-red-400/70")}
                      style={{ width: `${widthPct}%` }}
                    />
                  </div>
                  <span className="text-muted-foreground shrink-0 w-16 text-right">{span.duration_us != null ? formatDuration(span.duration_us) : "—"}</span>
                  {span.error && (
                    <span className="text-red-400 text-[10px] shrink-0 truncate max-w-24" title={span.error}>{span.error}</span>
                  )}
                </div>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
}

export default function TracesPage() {
  const [traces,  setTraces]  = useState<TraceRecord[]>([]);
  const [metrics, setMetrics] = useState<LiveMetrics | null>(null);
  const [loading, setLoading] = useState(true);
  const [search,  setSearch]  = useState("");

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [t, m] = await Promise.allSettled([listTraces(), getLiveMetrics()]);
      if (t.status === "fulfilled") setTraces(t.value);
      if (m.status === "fulfilled") setMetrics(m.value);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { load(); const id = setInterval(load, 15_000); return () => clearInterval(id); }, [load]);

  const filtered = search
    ? traces.filter((t) =>
        t.trace_id.includes(search) ||
        t.task_id.includes(search)
      )
    : traces;

  return (
    <div className="animate-fade-in space-y-5">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold gradient-text flex items-center gap-2">
            <GitBranch size={22} />
            Traces
          </h1>
          <p className="mt-0.5 text-sm text-muted-foreground">
            Distributed execution traces and span timings
          </p>
        </div>
        <Button onClick={load} variant="ghost" size="icon" className="h-9 w-9">
          <RefreshCw size={14} className={loading ? "animate-spin" : ""} />
        </Button>
      </div>

      {/* Live metrics */}
      {metrics && (
        <div>
          <p className="text-[11px] uppercase tracking-wider text-muted-foreground font-semibold mb-2">Live Metrics</p>
          <LiveMetricsBar m={metrics} />
        </div>
      )}

      {/* Summary */}
      <div className="grid grid-cols-3 gap-3">
        {[
          { icon: Activity, label: "Total Traces",  value: traces.length },
          { icon: CheckCircle, label: "Successful",  value: traces.filter((t) => t.success).length },
          { icon: AlertTriangle, label: "Failed",  value: traces.filter((t) => !t.success).length },
        ].map(({ icon: Icon, label, value }) => (
          <Card key={label} className="p-4">
            <div className="flex items-center gap-3">
              <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-primary/10">
                <Icon size={15} className="text-primary" />
              </div>
              <div>
                <p className="text-[11px] text-muted-foreground uppercase tracking-wider">{label}</p>
                <p className="text-lg font-bold">{value}</p>
              </div>
            </div>
          </Card>
        ))}
      </div>

      {/* Search */}
      <div className="relative max-w-sm">
        <Search size={13} className="absolute left-2.5 top-1/2 -translate-y-1/2 text-muted-foreground" />
        <Input
          placeholder="Search by trace/task ID…"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="pl-8 h-8 text-sm"
        />
        {search && (
          <button onClick={() => setSearch("")} className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground">
            <X size={13} />
          </button>
        )}
      </div>

      {/* Trace list */}
      <Card>
        {/* Column header */}
        <CardHeader className="pb-3 pt-4 px-4 border-b border-border">
          <div className="flex items-center gap-3 text-[10px] uppercase tracking-wider text-muted-foreground font-semibold">
            <span className="w-5 shrink-0" />
            <span className="w-24 shrink-0">Trace ID</span>
            <span className="flex-1">Task ID</span>
            <span className="shrink-0">Spans</span>
            <span className="shrink-0 w-20 text-right">Duration</span>
            <span className="shrink-0 w-12 text-right">Status</span>
            <span className="shrink-0 w-5" />
          </div>
        </CardHeader>
        <CardContent className="p-0">
          {loading ? (
            <div className="divide-y divide-border/50">
              {[...Array(6)].map((_, i) => (
                <div key={i} className="px-4 py-3"><Skeleton className="h-4 w-full" /></div>
              ))}
            </div>
          ) : filtered.length === 0 ? (
            <div className="flex items-center justify-center py-16 text-muted-foreground">
              <div className="text-center">
                <GitBranch size={32} className="mx-auto mb-3 opacity-20" />
                <p className="text-sm">No traces recorded yet</p>
                <p className="text-xs mt-1 opacity-70">Traces are recorded when WASM modules execute</p>
              </div>
            </div>
          ) : (
            <ScrollArea className="h-[500px]">
              {filtered.map((trace) => (
                <TraceRow key={trace.trace_id} trace={trace} />
              ))}
            </ScrollArea>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
