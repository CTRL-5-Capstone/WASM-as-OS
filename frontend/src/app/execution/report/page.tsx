"use client";

import { useEffect, useState, Suspense } from "react";
import { useSearchParams, useRouter } from "next/navigation";
import {
  CheckCircle2, XCircle, Clock, Cpu, Activity, MemoryStick,
  ArrowLeft, Download, RefreshCw, AlertCircle, FileText,
} from "lucide-react";
import { getExecutionReport, type ExecutionReport } from "@/lib/api";
import { formatDuration, cn } from "@/lib/utils";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";

// ── Metric card ──────────────────────────────────────────────────────────────

function MetricCard({
  icon: Icon,
  label,
  value,
  sub,
  highlight,
}: {
  icon: React.ElementType;
  label: string;
  value: string | number;
  sub?: string;
  highlight?: "green" | "red" | "neutral";
}) {
  const accent =
    highlight === "green"
      ? "border-green-500/40 bg-green-500/5"
      : highlight === "red"
      ? "border-red-500/40 bg-red-500/5"
      : "border-border bg-card";

  return (
    <div className={cn("rounded-xl border p-4 flex items-start gap-3", accent)}>
      <div className="mt-0.5 p-2 rounded-lg bg-muted/40">
        <Icon size={15} className="text-muted-foreground" />
      </div>
      <div className="min-w-0">
        <p className="text-[11px] text-muted-foreground uppercase tracking-wider">{label}</p>
        <p className="text-base font-semibold leading-snug mt-0.5 truncate">{value}</p>
        {sub && <p className="text-[11px] text-muted-foreground mt-0.5">{sub}</p>}
      </div>
    </div>
  );
}

// ── Table row helper ─────────────────────────────────────────────────────────

function Row({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <tr className="border-b border-border/40 last:border-0">
      <td className="py-2.5 pr-6 text-[12px] font-medium text-muted-foreground whitespace-nowrap w-40">
        {label}
      </td>
      <td className="py-2.5 text-[12px] font-mono break-all">{children}</td>
    </tr>
  );
}

// ── Loading skeleton ─────────────────────────────────────────────────────────

function ReportSkeleton() {
  return (
    <div className="space-y-6">
      <Skeleton className="h-9 w-64" />
      <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
        {[...Array(4)].map((_, i) => <Skeleton key={i} className="h-20" />)}
      </div>
      <Skeleton className="h-48" />
    </div>
  );
}

// ── Inner page (reads search params) ─────────────────────────────────────────

function ExecutionReportInner() {
  const searchParams = useSearchParams();
  const router       = useRouter();
  const execId       = searchParams.get("id") ?? "";

  const [report,  setReport]  = useState<ExecutionReport | null>(null);
  const [loading, setLoading] = useState(true);
  const [error,   setError]   = useState<string | null>(null);

  const load = async () => {
    if (!execId) { setLoading(false); return; }
    setLoading(true);
    setError(null);
    try {
      const r = await getExecutionReport(execId);
      setReport(r);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load report");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { load(); }, [execId]); // eslint-disable-line react-hooks/exhaustive-deps

  // ── Download JSON report ────────────────────────────────────────────────
  const downloadJson = () => {
    if (!report) return;
    const blob = new Blob([JSON.stringify(report, null, 2)], { type: "application/json" });
    const url  = URL.createObjectURL(blob);
    const a    = Object.assign(document.createElement("a"), {
      href:     url,
      download: `execution-report-${execId}.json`,
    });
    a.click();
    URL.revokeObjectURL(url);
  };

  // ── Derived display values ──────────────────────────────────────────────
  const durationMs =
    report?.duration_us != null ? (report.duration_us / 1_000).toFixed(2) : null;
  const memoryMb =
    report?.memory_bytes != null
      ? (report.memory_bytes / (1024 * 1024)).toFixed(2)
      : null;

  const startedFmt = report?.started_at
    ? new Date(report.started_at).toLocaleString()
    : null;
  const completedFmt = report?.completed_at
    ? new Date(report.completed_at).toLocaleString()
    : null;

  if (!execId) {
    return (
      <div className="min-h-screen bg-background p-6 flex items-center justify-center">
        <Card>
          <CardContent className="py-12 text-center">
            <AlertCircle size={36} className="mx-auto text-muted-foreground mb-3" />
            <p className="text-base font-medium">No execution ID specified</p>
            <p className="text-sm text-muted-foreground mt-1">
              Navigate to a report via the Tasks or Traces page.
            </p>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-background p-6 space-y-6 max-w-5xl mx-auto">

      {/* ── Header ── */}
      <div className="flex items-center justify-between gap-4 flex-wrap">
        <div className="flex items-center gap-3">
          <Button variant="ghost" size="sm" onClick={() => router.back()} className="h-8 px-2">
            <ArrowLeft size={15} className="mr-1" /> Back
          </Button>
          <div className="flex items-center gap-2">
            <FileText size={18} className="text-muted-foreground" />
            <h1 className="text-lg font-semibold">Execution Report</h1>
          </div>
          <code className="text-xs bg-muted px-2 py-0.5 rounded font-mono text-muted-foreground">
            {execId}
          </code>
        </div>
        <div className="flex items-center gap-2">
          <Button variant="outline" size="sm" onClick={load} disabled={loading} className="h-8">
            <RefreshCw size={13} className={cn("mr-1.5", loading && "animate-spin")} />
            Refresh
          </Button>
          <Button
            variant="outline" size="sm"
            onClick={downloadJson}
            disabled={!report || !report.found}
            className="h-8"
          >
            <Download size={13} className="mr-1.5" />
            Download JSON
          </Button>
        </div>
      </div>

      {/* ── Error banner ── */}
      {error && (
        <Alert variant="destructive">
          <AlertCircle size={15} />
          <AlertTitle>Request failed</AlertTitle>
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}

      {/* ── Loading ── */}
      {loading && <ReportSkeleton />}

      {/* ── Not found ── */}
      {!loading && !error && report && !report.found && (
        <Card>
          <CardContent className="py-16 flex flex-col items-center gap-3 text-center">
            <AlertCircle size={36} className="text-muted-foreground" />
            <p className="text-base font-medium">Execution not found</p>
            <p className="text-sm text-muted-foreground max-w-sm">
              No execution record exists for ID <code className="font-mono">{execId}</code>.
              It may have been pruned or the ID is incorrect.
            </p>
            <Button variant="outline" size="sm" onClick={() => router.push("/traces")} className="mt-2">
              View all traces
            </Button>
          </CardContent>
        </Card>
      )}

      {/* ── Report body ── */}
      {!loading && !error && report?.found && (
        <>
          {/* Status banner */}
          <div
            className={cn(
              "flex items-center gap-3 rounded-xl border px-5 py-4",
              report.success
                ? "border-green-500/40 bg-green-500/5"
                : "border-red-500/40 bg-red-500/5",
            )}
          >
            {report.success ? (
              <CheckCircle2 size={22} className="text-green-400 shrink-0" />
            ) : (
              <XCircle size={22} className="text-red-400 shrink-0" />
            )}
            <div>
              <p className="font-semibold text-sm">
                {report.success ? "Execution succeeded" : "Execution failed"}
              </p>
              {!report.success && report.error && (
                <p className="text-xs text-muted-foreground mt-0.5">{report.error}</p>
              )}
            </div>
            <Badge
              variant={report.success ? "default" : "destructive"}
              className="ml-auto text-[11px] px-2.5"
            >
              {report.success ? "SUCCESS" : "FAILED"}
            </Badge>
          </div>

          {/* Metric grid */}
          <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
            <MetricCard
              icon={Clock}
              label="Duration"
              value={durationMs != null ? `${durationMs} ms` : "—"}
              sub={report.duration_us != null ? formatDuration(report.duration_us) : undefined}
              highlight="neutral"
            />
            <MetricCard
              icon={Cpu}
              label="Instructions"
              value={report.instructions?.toLocaleString() ?? "—"}
              highlight="neutral"
            />
            <MetricCard
              icon={Activity}
              label="Syscalls"
              value={report.syscalls?.toLocaleString() ?? "—"}
              highlight="neutral"
            />
            <MetricCard
              icon={MemoryStick}
              label="Peak Memory"
              value={memoryMb != null ? `${memoryMb} MB` : "—"}
              sub={report.memory_bytes != null ? `${report.memory_bytes.toLocaleString()} bytes` : undefined}
              highlight="neutral"
            />
          </div>

          {/* Detail table */}
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-semibold">Execution Details</CardTitle>
            </CardHeader>
            <CardContent className="pt-0">
              <table className="w-full">
                <tbody>
                  <Row label="Execution ID">
                    <span className="text-indigo-400">{report.execution_id}</span>
                  </Row>
                  {report.task_id && (
                    <Row label="Task ID">
                      <button
                        className="text-indigo-400 hover:underline"
                        onClick={() => router.push(`/tasks?id=${report.task_id}`)}
                      >
                        {report.task_id}
                      </button>
                    </Row>
                  )}
                  <Row label="Status">
                    <Badge
                      variant={report.success ? "default" : "destructive"}
                      className="text-[10px] h-4 px-1.5"
                    >
                      {report.success ? "Success" : "Failed"}
                    </Badge>
                  </Row>
                  <Row label="Started">
                    {startedFmt ?? <span className="text-muted-foreground">—</span>}
                  </Row>
                  <Row label="Completed">
                    {completedFmt ?? <span className="text-muted-foreground">—</span>}
                  </Row>
                  <Row label="Duration">
                    {durationMs != null
                      ? `${durationMs} ms (${report.duration_us?.toLocaleString()} µs)`
                      : <span className="text-muted-foreground">—</span>}
                  </Row>
                  <Row label="Instructions">
                    {report.instructions?.toLocaleString() ?? <span className="text-muted-foreground">—</span>}
                  </Row>
                  <Row label="Syscalls">
                    {report.syscalls?.toLocaleString() ?? <span className="text-muted-foreground">—</span>}
                  </Row>
                  <Row label="Peak Memory">
                    {memoryMb != null
                      ? `${memoryMb} MB (${report.memory_bytes?.toLocaleString()} bytes)`
                      : <span className="text-muted-foreground">—</span>}
                  </Row>
                  {report.error && (
                    <Row label="Error">
                      <span className="text-red-400">{report.error}</span>
                    </Row>
                  )}
                </tbody>
              </table>
            </CardContent>
          </Card>

          {/* Security link */}
          {report.task_id && (
            <div className="flex gap-2 justify-end">
              <Button
                variant="outline" size="sm"
                onClick={() => router.push(`/security?task=${report.task_id}`)}
              >
                Open in Security Hub
              </Button>
              <Button
                variant="outline" size="sm"
                onClick={() => router.push(`/traces?task=${report.task_id}`)}
              >
                View all traces for this task
              </Button>
            </div>
          )}
        </>
      )}
    </div>
  );
}

// ── Root export (wraps inner in Suspense for useSearchParams) ─────────────────

export default function ExecutionReportPage() {
  return (
    <Suspense fallback={<div className="min-h-screen bg-background p-6"><ReportSkeleton /></div>}>
      <ExecutionReportInner />
    </Suspense>
  );
}
