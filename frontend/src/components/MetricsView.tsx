"use client";

import { useEffect, useState, useCallback } from "react";
import { getPrometheusMetrics } from "@/lib/api";
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
  CartesianGrid,
} from "recharts";
import { RefreshCw } from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";

interface ParsedMetric {
  name: string;
  labels: Record<string, string>;
  value: number;
}

function parsePrometheus(text: string): ParsedMetric[] {
  const metrics: ParsedMetric[] = [];
  for (const line of text.split("\n")) {
    if (line.startsWith("#") || !line.trim()) continue;

    const match = line.match(
      /^([a-zA-Z_:][a-zA-Z0-9_:]*)(\{(.+?)\})?\s+([\d.eE+-]+|NaN|Inf|-Inf)$/
    );
    if (!match) continue;

    const name = match[1];
    const labels: Record<string, string> = {};
    if (match[3]) {
      for (const pair of match[3].split(",")) {
        const [k, v] = pair.split("=");
        if (k && v) labels[k.trim()] = v.replace(/"/g, "").trim();
      }
    }
    const value = parseFloat(match[4]);
    if (!isNaN(value)) {
      metrics.push({ name, labels, value });
    }
  }
  return metrics;
}

export default function MetricsView() {
  const [raw, setRaw] = useState("");
  const [parsed, setParsed] = useState<ParsedMetric[]>([]);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const text = await getPrometheusMetrics();
      setRaw(text);
      setParsed(parsePrometheus(text));
      setError(null);
    } catch {
      setError("Failed to fetch metrics");
    }
  }, []);

  useEffect(() => {
    refresh();
    const id = setInterval(refresh, 10000);
    return () => clearInterval(id);
  }, [refresh]);

  // Build chart data from task counters
  const taskCounters = parsed
    .filter((m) => m.name.includes("tasks_total") || m.name.includes("task_executions"))
    .map((m) => ({
      name: m.labels.status || m.labels.result || m.name.split("_").pop() || m.name,
      value: m.value,
    }));

  if (error) {
    return (
      <Card className="border-red-500/30 bg-red-500/10">
        <CardContent className="p-4 text-red-400 text-sm">
          {error}
        </CardContent>
      </Card>
    );
  }

  return (
    <div className="space-y-6">
      {/* Chart */}
      {taskCounters.length > 0 && (
        <Card>
          <CardHeader className="flex-row items-center justify-between space-y-0">
            <CardTitle className="text-sm text-foreground">Task & Execution Counters</CardTitle>
            <Button onClick={refresh} variant="ghost" size="sm" className="text-xs">
              <RefreshCw size={12} /> Refresh
            </Button>
          </CardHeader>
          <CardContent>
          <ResponsiveContainer width="100%" height={260}>
            <BarChart data={taskCounters}>
              <CartesianGrid strokeDasharray="3 3" stroke="#1e293b" />
              <XAxis dataKey="name" tick={{ fill: "#94a3b8", fontSize: 12 }} />
              <YAxis tick={{ fill: "#94a3b8", fontSize: 12 }} />
              <Tooltip
                contentStyle={{
                  backgroundColor: "hsl(var(--card))",
                  border: "1px solid hsl(var(--border))",
                  borderRadius: "8px",
                  color: "hsl(var(--foreground))",
                  fontSize: "12px",
                }}
              />
              <Bar dataKey="value" fill="#6366f1" radius={[4, 4, 0, 0]} />
            </BarChart>
          </ResponsiveContainer>
          </CardContent>
        </Card>
      )}

      {/* Raw metrics */}
      <Card>
        <CardHeader>
          <CardTitle className="text-sm text-foreground">Raw Prometheus Metrics</CardTitle>
        </CardHeader>
        <CardContent>
          <ScrollArea className="h-96 rounded-lg border border-border bg-black/40">
            <pre className="p-4 text-xs text-emerald-400 overflow-x-auto font-mono leading-relaxed">
              {raw || "Loading…"}
            </pre>
          </ScrollArea>
        </CardContent>
      </Card>
    </div>
  );
}
