"use client";

/**
 * Metrics — Prometheus raw metrics + typed import/advanced stats.
 * Polls /metrics (Prometheus text) and /v2/imports/stats every 10s.
 */

import { useState, useEffect, useCallback } from "react";
import {
  BarChart3, RefreshCw, Download, Activity, Cpu, Layers,
  TrendingUp, Package, Info,
} from "lucide-react";
import { getPrometheusMetrics, getImportStats, type ImportStats } from "@/lib/api";
import { formatNumber, cn } from "@/lib/utils";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { toast } from "sonner";

// ── Parse Prometheus text into groups ───────────────────────────────

interface MetricLine {
  name: string;
  labels: string;
  value: string;
  help?: string;
}

function parsePrometheus(raw: string): MetricLine[] {
  const lines = raw.split("\n");
  const result: MetricLine[] = [];
  const helpMap: Record<string, string> = {};

  for (const line of lines) {
    if (line.startsWith("# HELP")) {
      const parts = line.slice(7).split(" ");
      helpMap[parts[0]] = parts.slice(1).join(" ");
    } else if (!line.startsWith("#") && line.trim()) {
      const match = line.match(/^([a-zA-Z_:][a-zA-Z0-9_:]*)([\{].*[\}])?\s+(.+)$/);
      if (match) {
        result.push({
          name: match[1],
          labels: match[2] ?? "",
          value: match[3],
          help: helpMap[match[1]],
        });
      }
    }
  }
  return result;
}

export default function MetricsPage() {
  const [raw, setRaw] = useState<string>("");
  const [metrics, setMetrics] = useState<MetricLine[]>([]);
  const [importStats, setImportStats] = useState<ImportStats | null>(null);
  const [loading, setLoading] = useState(true);
  const [filter, setFilter] = useState("");
  const [lastUpdated, setLastUpdated] = useState<Date | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [promText, iStats] = await Promise.allSettled([
        getPrometheusMetrics(),
        getImportStats(),
      ]);

      if (promText.status === "fulfilled") {
        setRaw(promText.value);
        setMetrics(parsePrometheus(promText.value));
      } else {
        setRaw("# Prometheus endpoint unavailable");
        setMetrics([]);
      }

      if (iStats.status === "fulfilled") {
        setImportStats(iStats.value);
      }

      setLastUpdated(new Date());
    } catch {
      toast.error("Failed to load metrics");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
    const id = setInterval(load, 15_000);
    return () => clearInterval(id);
  }, [load]);

  const filteredMetrics = metrics.filter((m) =>
    !filter || m.name.toLowerCase().includes(filter.toLowerCase())
  );

  const downloadRaw = () => {
    const blob = new Blob([raw], { type: "text/plain" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `wasmos-metrics-${Date.now()}.txt`;
    a.click();
    URL.revokeObjectURL(url);
  };

  // Group metrics by prefix
  const groups = filteredMetrics.reduce<Record<string, MetricLine[]>>((acc, m) => {
    const prefix = m.name.split("_").slice(0, 2).join("_");
    if (!acc[prefix]) acc[prefix] = [];
    acc[prefix].push(m);
    return acc;
  }, {});

  return (
    <div className="animate-fade-in space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold gradient-text flex items-center gap-2">
            <BarChart3 size={26} /> Metrics
          </h1>
          <p className="text-sm text-slate-500 mt-1">
            Live Prometheus metrics + WASM import analysis
            {lastUpdated && (
              <span className="ml-2 text-slate-400">
                · updated {lastUpdated.toLocaleTimeString()}
              </span>
            )}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Button variant="outline" size="sm" onClick={downloadRaw} className="text-xs" disabled={!raw}>
            <Download size={13} /> Export
          </Button>
          <Button variant="outline" size="sm" onClick={load} disabled={loading} className="text-xs">
            <RefreshCw size={13} className={cn(loading && "animate-spin")} /> Refresh
          </Button>
        </div>
      </div>

      {/* Import stats cards — derived from actual ImportStats shape */}
      {importStats && (
        <div className="grid grid-cols-2 sm:grid-cols-4 gap-4">
          <StatCard
            icon={<Package size={16} />}
            label="Tasks Scanned"
            value={formatNumber(importStats.total_tasks_scanned)}
            color="indigo"
          />
          <StatCard
            icon={<Layers size={16} />}
            label="WASI Modules"
            value={formatNumber(
              importStats.modules.filter((m) => m.name.toLowerCase().includes("wasi")).length
            )}
            color="sky"
          />
          <StatCard
            icon={<Cpu size={16} />}
            label="Custom Host Modules"
            value={formatNumber(
              importStats.modules.filter((m) => !m.name.toLowerCase().includes("wasi")).length
            )}
            color="violet"
          />
          <StatCard
            icon={<TrendingUp size={16} />}
            label="Unique Namespaces"
            value={formatNumber(importStats.modules.length)}
            color="emerald"
          />
        </div>
      )}

      {/* Module namespace breakdown */}
      {importStats && importStats.modules.length > 0 && (
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-slate-700 flex items-center gap-2">
              <Activity size={14} className="text-indigo-500" /> Import Namespace Usage
            </CardTitle>
          </CardHeader>
          <CardContent className="p-0">
            <div className="overflow-x-auto">
              <table className="w-full text-xs">
                <thead className="border-b border-slate-100">
                  <tr className="text-left text-[10px] uppercase text-slate-500">
                    <th className="px-4 py-2">Namespace</th>
                    <th className="px-4 py-2">Tasks Using</th>
                    <th className="px-4 py-2">Enabled</th>
                    <th className="px-4 py-2">Share</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-slate-50">
                  {[...importStats.modules]
                    .sort((a, b) => b.task_count - a.task_count)
                    .slice(0, 15)
                    .map((mod, i) => {
                      const max = importStats.modules[0]?.task_count ?? 1;
                      const pct = Math.round((mod.task_count / max) * 100);
                      return (
                        <tr key={i} className="hover:bg-slate-50">
                          <td className="px-4 py-2 font-mono text-indigo-600">{mod.name}</td>
                          <td className="px-4 py-2 font-semibold text-slate-700">
                            {formatNumber(mod.task_count)}
                          </td>
                          <td className="px-4 py-2">
                            <span
                              className={`inline-flex items-center rounded-full px-2 py-0.5 text-[10px] font-medium ${
                                mod.enabled
                                  ? "bg-emerald-50 text-emerald-700"
                                  : "bg-red-50 text-red-600"
                              }`}
                            >
                              {mod.enabled ? "allowed" : "blocked"}
                            </span>
                          </td>
                          <td className="px-4 py-2 w-32">
                            <div className="flex items-center gap-2">
                              <div className="flex-1 h-1.5 rounded-full bg-slate-100">
                                <div
                                  className="h-1.5 rounded-full bg-indigo-400 transition-all"
                                  style={{ width: `${pct}%` }}
                                />
                              </div>
                              <span className="text-slate-400 w-8 text-right">{pct}%</span>
                            </div>
                          </td>
                        </tr>
                      );
                    })}
                </tbody>
              </table>
            </div>
          </CardContent>
        </Card>
      )}

      {/* Prometheus metrics */}
      <div>
        <div className="flex items-center justify-between mb-3">
          <h2 className="text-sm font-semibold text-slate-700 flex items-center gap-1.5">
            <Info size={13} className="text-indigo-500" /> Prometheus Metrics
            {metrics.length > 0 && (
              <span className="ml-1 text-[10px] rounded-full bg-slate-100 text-slate-600 px-2 py-0.5">
                {metrics.length} series
              </span>
            )}
          </h2>
          <Input
            placeholder="Filter by name…"
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            className="h-8 w-48 text-xs"
          />
        </div>

        {loading && metrics.length === 0 ? (
          <div className="space-y-3">
            {[1, 2, 3].map((i) => (
              <div key={i} className="h-20 animate-pulse rounded-xl bg-slate-100" />
            ))}
          </div>
        ) : metrics.length === 0 ? (
          <Card>
            <CardContent className="py-12 text-center">
              <BarChart3 size={32} className="mx-auto text-slate-200 mb-3" />
              <p className="text-sm text-slate-400">No Prometheus metrics available</p>
              <p className="text-xs text-slate-300 mt-1">
                Make sure the backend is running with metrics enabled
              </p>
            </CardContent>
          </Card>
        ) : (
          <div className="space-y-3">
            {Object.entries(groups).map(([prefix, lines]) => (
              <Card key={prefix}>
                <CardHeader className="pb-1">
                  <CardTitle className="text-xs font-mono text-slate-600">{prefix}_*</CardTitle>
                </CardHeader>
                <CardContent className="p-0">
                  <div className="overflow-x-auto">
                    <table className="w-full text-xs">
                      <tbody className="divide-y divide-slate-50">
                        {lines.map((m, i) => (
                          <tr key={i} className="hover:bg-slate-50">
                            <td className="px-4 py-1.5 font-mono text-slate-700 whitespace-nowrap">
                              {m.name}
                            </td>
                            <td className="px-4 py-1.5 font-mono text-slate-500 text-[10px] max-w-xs truncate">
                              {m.labels}
                            </td>
                            <td className="px-4 py-1.5 font-semibold text-indigo-700 text-right whitespace-nowrap">
                              {m.value}
                            </td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                </CardContent>
              </Card>
            ))}
          </div>
        )}
      </div>

      {/* Raw text toggle */}
      {raw && (
        <details className="group">
          <summary className="cursor-pointer text-xs font-medium text-slate-500 hover:text-slate-700 select-none">
            Raw Prometheus text
          </summary>
          <pre className="mt-2 max-h-64 overflow-auto rounded-xl bg-slate-950 px-4 py-3 text-[10px] text-emerald-400 font-mono leading-relaxed">
            {raw}
          </pre>
        </details>
      )}
    </div>
  );
}

function StatCard({
  icon, label, value, color,
}: {
  icon: React.ReactNode;
  label: string;
  value: string;
  color: "indigo" | "sky" | "violet" | "emerald";
}) {
  const colors = {
    indigo: "bg-indigo-50 border-indigo-100 text-indigo-600",
    sky:    "bg-sky-50 border-sky-100 text-sky-600",
    violet: "bg-violet-50 border-violet-100 text-violet-600",
    emerald:"bg-emerald-50 border-emerald-100 text-emerald-600",
  };
  return (
    <div className={cn("rounded-xl border p-4", colors[color])}>
      <div className="flex items-center gap-1.5 text-[10px] font-semibold uppercase tracking-wider mb-1.5">
        {icon} {label}
      </div>
      <p className="text-2xl font-bold text-slate-900">{value}</p>
    </div>
  );
}

