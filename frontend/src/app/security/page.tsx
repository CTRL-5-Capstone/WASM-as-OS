"use client";

import { useEffect, useState, useRef, useCallback, Suspense } from "react";
import { useSearchParams } from "next/navigation";
import Link from "next/link";
import {
  ShieldAlert, Upload, FileCode, AlertTriangle, CheckCircle,
  Info, Search, X, Copy, ChevronRight, Layers, Globe,
  Cpu, HardDrive, Code2, BarChart3, ScrollText,
  ExternalLink, ArrowLeft,
} from "lucide-react";
import {
  getTasks, getTask, getTaskSecurity, type Task, type TaskDetail, type SecurityReport,
} from "@/lib/api";
import {
  parseWasm, analyseWasm, hexDump, diffModules,
  type WasmParseResult, type SecurityAnalysis, type SecurityFinding,
} from "@/lib/wasm-parser";
import { formatBytes, timeAgo, cn } from "@/lib/utils";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Separator } from "@/components/ui/separator";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Alert, AlertTitle, AlertDescription } from "@/components/ui/alert";
import { Skeleton } from "@/components/ui/skeleton";

// ─── Risk gauge ──────────────────────────────────────────────────────────────

function RiskGauge({ score, grade }: { score: number; grade: string }) {
  const pct = Math.min(100, Math.max(0, score));
  const colorStroke =
    grade === "A" ? "stroke-green-400" :
    grade === "B" ? "stroke-green-500" :
    grade === "C" ? "stroke-yellow-400" :
    grade === "D" ? "stroke-orange-400" :
                    "stroke-red-400";
  const colorText =
    grade === "A" ? "text-green-400" :
    grade === "B" ? "text-green-500" :
    grade === "C" ? "text-yellow-400" :
    grade === "D" ? "text-orange-400" :
                    "text-red-400";
  const circumference = 2 * Math.PI * 42;
  const dashoffset = circumference - (pct / 100) * circumference;
  return (
    <div className="flex flex-col items-center gap-1.5 shrink-0">
      <div className="relative h-20 w-20">
        <svg className="h-20 w-20 -rotate-90" viewBox="0 0 100 100">
          <circle cx="50" cy="50" r="42" fill="none" strokeWidth="8" className="stroke-border" />
          <circle cx="50" cy="50" r="42" fill="none" strokeWidth="8"
            strokeDasharray={circumference} strokeDashoffset={dashoffset}
            strokeLinecap="round" className={cn("transition-all duration-700", colorStroke)} />
        </svg>
        <div className="absolute inset-0 flex flex-col items-center justify-center">
          <span className={cn("text-xl font-bold leading-none", colorText)}>{grade}</span>
          <span className="text-[10px] text-muted-foreground mt-0.5">{score}/100</span>
        </div>
      </div>
      <p className="text-[10px] text-muted-foreground uppercase tracking-wider">Risk Score</p>
    </div>
  );
}

// ─── Finding row ─────────────────────────────────────────────────────────────

function FindingRow({ f }: { f: SecurityFinding }) {
  const cfg: Record<string, { bg: string; text: string; dot: string }> = {
    critical: { bg: "bg-red-950/40 border-red-900/40",       text: "text-red-300",    dot: "bg-red-400" },
    high:     { bg: "bg-orange-950/40 border-orange-900/40", text: "text-orange-300", dot: "bg-orange-400" },
    medium:   { bg: "bg-yellow-950/40 border-yellow-900/40", text: "text-yellow-300", dot: "bg-yellow-400" },
    info:     { bg: "bg-blue-950/20 border-blue-900/30",     text: "text-blue-300",   dot: "bg-blue-400" },
  };
  const s = cfg[f.level] ?? cfg.info;
  return (
    <div className={cn("flex items-start gap-3 rounded-lg border p-3", s.bg)}>
      <span className={cn("mt-1.5 h-1.5 w-1.5 shrink-0 rounded-full", s.dot)} />
      <div className="min-w-0 flex-1">
        <p className={cn("text-xs font-semibold", s.text)}>{f.title}</p>
        <p className="text-[11px] text-muted-foreground mt-0.5 leading-relaxed">{f.description}</p>
        {f.evidence && (
          <p className="text-[10px] font-mono text-muted-foreground/60 mt-1 truncate">evidence: {f.evidence}</p>
        )}
      </div>
      <Badge variant="outline" className={cn("shrink-0 text-[10px] h-4 px-1.5 uppercase font-bold border-0", s.text)}>
        {f.level}
      </Badge>
    </div>
  );
}

// ─── Section bar ─────────────────────────────────────────────────────────────

function SectionBar({ name, size, total }: { name: string; size: number; total: number }) {
  const pct = total > 0 ? Math.max(2, Math.round((size / total) * 100)) : 2;
  return (
    <div className="flex items-center gap-2 text-xs">
      <span className="w-20 shrink-0 text-muted-foreground font-medium truncate">{name}</span>
      <div className="flex-1 rounded-full bg-muted/40 h-1.5">
        <div className="h-1.5 rounded-full bg-primary/70 transition-all duration-500" style={{ width: `${pct}%` }} />
      </div>
      <span className="w-14 text-right text-muted-foreground shrink-0">{formatBytes(size)}</span>
    </div>
  );
}

// ─── String scanner ──────────────────────────────────────────────────────────

function StringScanner({ strings }: { strings: string[] }) {
  const [query, setQuery] = useState("");
  const filtered = query ? strings.filter((s) => s.toLowerCase().includes(query.toLowerCase())) : strings;
  const getStringClass = (s: string) => {
    if (/https?:\/\/|ftp:\/\//.test(s))               return "text-yellow-300";
    if (/\/etc\/|\/proc\/|C:\\/.test(s))               return "text-orange-300";
    if (/password|secret|token|key|api/i.test(s))      return "text-red-300";
    if (/\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}/.test(s)) return "text-yellow-300";
    return "text-foreground/70";
  };
  return (
    <div className="space-y-3">
      <div className="relative">
        <Search size={13} className="absolute left-2.5 top-1/2 -translate-y-1/2 text-muted-foreground" />
        <Input placeholder="Filter strings…" value={query} onChange={(e) => setQuery(e.target.value)} className="pl-8 h-8 text-xs font-mono" />
        {query && <button onClick={() => setQuery("")} className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"><X size={13} /></button>}
      </div>
      <p className="text-[11px] text-muted-foreground">
        {filtered.length} of {strings.length} strings{query && ` matching "${query}"`}
      </p>
      <ScrollArea className="h-48">
        <div className="space-y-0.5 font-mono text-xs">
          {filtered.slice(0, 200).map((s, i) => (
            <div key={i} className={cn("px-2 py-0.5 rounded hover:bg-muted/20", getStringClass(s))}>
              {s.length > 120 ? s.slice(0, 120) + "…" : s}
            </div>
          ))}
          {filtered.length > 200 && <div className="px-2 py-1 text-muted-foreground italic">…and {filtered.length - 200} more</div>}
        </div>
      </ScrollArea>
    </div>
  );
}

// ─── Hex viewer ──────────────────────────────────────────────────────────────

function HexViewer({ bytes }: { bytes: Uint8Array }) {
  const [maxBytes, setMaxBytes] = useState(512);
  const [copied, setCopied] = useState(false);
  const hex = hexDump(bytes, maxBytes);
  const copy = () => navigator.clipboard.writeText(hex).then(() => { setCopied(true); setTimeout(() => setCopied(false), 2000); });
  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <p className="text-[11px] text-muted-foreground">Showing first {Math.min(maxBytes, bytes.length)} of {bytes.length} bytes</p>
        <div className="flex gap-2">
          {maxBytes < bytes.length && <Button variant="ghost" size="sm" className="h-6 text-xs" onClick={() => setMaxBytes(maxBytes + 512)}>Show more</Button>}
          <Button variant="ghost" size="sm" className="h-6 text-xs" onClick={copy}><Copy size={11} />{copied ? "Copied!" : "Copy"}</Button>
        </div>
      </div>
      <ScrollArea className="h-56">
        <pre className="text-[10px] font-mono leading-5 text-foreground/80 whitespace-pre p-1">{hex}</pre>
      </ScrollArea>
    </div>
  );
}

// ─── Upload zone ─────────────────────────────────────────────────────────────

function LocalUploadZone({ onParsed, label = "Drop .wasm here" }: { onParsed: (bytes: Uint8Array, name: string) => void; label?: string }) {
  const [dragging, setDragging] = useState(false);
  const fileRef = useRef<HTMLInputElement>(null);
  const handle = async (file: File) => { const buf = await file.arrayBuffer(); onParsed(new Uint8Array(buf), file.name); };
  return (
    <div
      onDragOver={(e) => { e.preventDefault(); setDragging(true); }}
      onDragLeave={() => setDragging(false)}
      onDrop={(e) => { e.preventDefault(); setDragging(false); const f = e.dataTransfer.files?.[0]; if (f) handle(f); }}
      onClick={() => fileRef.current?.click()}
      className={cn("flex items-center justify-center gap-3 rounded-xl border-2 border-dashed p-5 cursor-pointer transition-all",
        dragging ? "border-purple-400/60 bg-purple-400/8" : "border-border hover:border-purple-400/30 hover:bg-muted/20")}
    >
      <input ref={fileRef} type="file" accept=".wasm" className="hidden"
        onChange={(e) => { const f = e.target.files?.[0]; if (f) handle(f); e.target.value = ""; }} />
      <Upload size={18} className={dragging ? "text-purple-400" : "text-muted-foreground"} />
      <div>
        <p className="text-sm font-medium">{label}</p>
        <p className="text-[11px] text-muted-foreground">Analysed entirely in your browser</p>
      </div>
    </div>
  );
}

// ─── Derive capabilities from findings ───────────────────────────────────────

function findingsToCapabilities(findings: SecurityFinding[]) {
  const seen = new Set<string>();
  const out: { label: string; category: string }[] = [];
  for (const f of findings) {
    const key = `${f.category}::${f.title}`;
    if (!seen.has(key)) { seen.add(key); out.push({ label: f.title, category: f.category }); }
  }
  return out;
}

const CAP_COLORS: Record<string, string> = {
  "File I/O":    "text-orange-400 bg-orange-400/10 border-orange-400/20",
  "File System": "text-red-400 bg-red-400/10 border-red-400/20",
  "Network":     "text-red-400 bg-red-400/10 border-red-400/20",
  "Process":     "text-red-400 bg-red-400/10 border-red-400/20",
  "Timing":      "text-yellow-400 bg-yellow-400/10 border-yellow-400/20",
  "Entropy":     "text-purple-400 bg-purple-400/10 border-purple-400/20",
  "Environment": "text-yellow-400 bg-yellow-400/10 border-yellow-400/20",
  "Embedded Data":"text-orange-400 bg-orange-400/10 border-orange-400/20",
};

function CapBadge({ label, category }: { label: string; category: string }) {
  const cls = CAP_COLORS[category] ?? "text-muted-foreground bg-muted/20 border-border";
  return <span className={cn("inline-flex items-center rounded-full border px-2.5 py-0.5 text-[11px] font-medium", cls)}>{label}</span>;
}

// ─── Info alert shorthand ─────────────────────────────────────────────────────

function InfoAlert({ text }: { text: string }) {
  return <Alert variant="info"><Info size={14} /><AlertDescription>{text}</AlertDescription></Alert>;
}

// ═════════════════════════════════════════════════════════════════════════════
// Inner page
// ═════════════════════════════════════════════════════════════════════════════

function SecurityHubInner() {
  const searchParams  = useSearchParams();
  const initialTaskId = searchParams.get("task") ?? "";

  const [tasks,         setTasks]         = useState<Task[]>([]);
  const [taskId,        setTaskId]        = useState(initialTaskId);
  const [taskFilter,    setTaskFilter]    = useState("");
  const [taskDetail,    setTaskDetail]    = useState<TaskDetail | null>(null);
  const [backendReport, setBackendReport] = useState<SecurityReport | null>(null);
  const [localBytes,    setLocalBytes]    = useState<Uint8Array | null>(null);
  const [localName,     setLocalName]     = useState("");
  const [parseResult,   setParseResult]   = useState<WasmParseResult | null>(null);
  const [analysis,      setAnalysis]      = useState<SecurityAnalysis | null>(null);
  const [compareName,   setCompareName]   = useState("");
  const [compareResult, setCompareResult] = useState<WasmParseResult | null>(null);
  const [loading,       setLoading]       = useState(false);
  const [activeTab,     setActiveTab]     = useState("overview");

  useEffect(() => { getTasks().then(setTasks).catch(() => {}); }, []);

  const loadTask = useCallback(async (id: string) => {
    setLoading(true);
    setLocalBytes(null); setParseResult(null); setAnalysis(null); setBackendReport(null);
    try {
      const [d, s] = await Promise.allSettled([getTask(id), getTaskSecurity(id)]);
      if (d.status === "fulfilled") setTaskDetail(d.value);
      if (s.status === "fulfilled") setBackendReport(s.value);
    } finally { setLoading(false); }
  }, []);

  useEffect(() => { if (initialTaskId) loadTask(initialTaskId); }, [initialTaskId, loadTask]);

  const handleTaskSelect = (id: string) => {
    setTaskId(id); setTaskFilter(""); setLocalBytes(null); setLocalName("");
    setParseResult(null); setAnalysis(null); loadTask(id); setActiveTab("overview");
  };

  const handleLocalFile = (bytes: Uint8Array, name: string) => {
    setLocalBytes(bytes); setLocalName(name); setTaskId(""); setTaskDetail(null); setBackendReport(null);
    const parsed = parseWasm(bytes.buffer as ArrayBuffer);
    setParseResult(parsed); setAnalysis(analyseWasm(parsed)); setActiveTab("overview");
  };

  const handleCompareFile = (bytes: Uint8Array, name: string) => {
    setCompareName(name); setCompareResult(parseWasm(bytes.buffer as ArrayBuffer)); setActiveTab("compare");
  };

  const displayName = localName || taskDetail?.task.name || "No module selected";
  const hasData     = !!parseResult || !!backendReport || !!taskDetail;
  const filteredTasks = tasks.filter((t) => !taskFilter || t.name.toLowerCase().includes(taskFilter.toLowerCase()));
  const caps = analysis ? findingsToCapabilities(analysis.findings) : [];
  const byLevel = {
    critical: analysis?.findings.filter((f) => f.level === "critical") ?? [],
    high:     analysis?.findings.filter((f) => f.level === "high")     ?? [],
    medium:   analysis?.findings.filter((f) => f.level === "medium")   ?? [],
    info:     analysis?.findings.filter((f) => f.level === "info")     ?? [],
  };

  return (
    <div className="animate-fade-in space-y-5">
      {/* Header */}
      <div className="flex items-center gap-3">
        <Link href="/tasks"><Button variant="ghost" size="icon" className="h-8 w-8"><ArrowLeft size={15} /></Button></Link>
        <div>
          <h1 className="text-2xl font-bold flex items-center gap-2">
            <ShieldAlert size={22} className="text-purple-400" />
            <span className="gradient-text">Security Hub</span>
          </h1>
          <p className="mt-0.5 text-sm text-muted-foreground">Binary inspection · Capability analysis · Risk scoring · Module diffing</p>
        </div>
      </div>

      <div className="grid grid-cols-1 xl:grid-cols-[280px_1fr] gap-4 items-start">
        {/* LEFT */}
        <div className="space-y-3">
          <Card>
            <CardHeader className="pb-2 pt-4 px-4">
              <CardTitle className="text-xs uppercase tracking-wider text-muted-foreground font-semibold">From Tasks</CardTitle>
            </CardHeader>
            <CardContent className="px-2 pb-3">
              <div className="px-2 mb-2">
                <div className="relative">
                  <Search size={12} className="absolute left-2.5 top-1/2 -translate-y-1/2 text-muted-foreground" />
                  <Input placeholder="Filter…" value={taskFilter} onChange={(e) => setTaskFilter(e.target.value)} className="pl-7 h-7 text-xs" />
                </div>
              </div>
              <ScrollArea className="h-44">
                {filteredTasks.length === 0
                  ? <p className="px-3 py-4 text-xs text-muted-foreground text-center">No tasks</p>
                  : filteredTasks.map((t) => (
                    <button key={t.id} onClick={() => handleTaskSelect(t.id)}
                      className={cn("w-full flex items-center gap-2 rounded-md px-3 py-1.5 text-left text-xs transition-colors",
                        taskId === t.id ? "bg-primary/10 text-primary" : "text-muted-foreground hover:bg-muted/30 hover:text-foreground")}>
                      <FileCode size={13} className="shrink-0" />
                      <span className="truncate">{t.name}</span>
                    </button>
                  ))}
              </ScrollArea>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="pb-2 pt-4 px-4">
              <CardTitle className="text-xs uppercase tracking-wider text-muted-foreground font-semibold">Local File</CardTitle>
            </CardHeader>
            <CardContent className="px-4 pb-4">
              <LocalUploadZone onParsed={handleLocalFile} />
            </CardContent>
          </Card>

          {(localBytes || parseResult) && (
            <Card>
              <CardHeader className="pb-2 pt-4 px-4">
                <CardTitle className="text-xs uppercase tracking-wider text-muted-foreground font-semibold">Compare Against</CardTitle>
              </CardHeader>
              <CardContent className="px-4 pb-4">
                <LocalUploadZone onParsed={handleCompareFile} label="Drop second .wasm here" />
                {compareName && <p className="mt-2 text-xs text-muted-foreground truncate">vs. {compareName}</p>}
              </CardContent>
            </Card>
          )}
        </div>

        {/* RIGHT */}
        <div className="min-w-0">
          {loading ? (
            <Card className="space-y-4 p-6">
              <Skeleton className="h-6 w-1/3" /><Skeleton className="h-24 w-full" /><Skeleton className="h-24 w-full" />
            </Card>
          ) : !hasData ? (
            <Card className="flex flex-col items-center justify-center py-24 text-center text-muted-foreground">
              <ShieldAlert size={48} className="mb-4 opacity-15 text-purple-400" />
              <p className="text-sm font-medium">Select a task or drop a local .wasm</p>
              <p className="text-xs mt-1 opacity-70">Risk scores · Imports/exports · Hex dump · String extraction · Module diff</p>
            </Card>
          ) : (
            <Card>
              <CardHeader className="pb-3">
                <div className="flex items-start justify-between gap-3">
                  <div className="min-w-0 flex-1">
                    <CardTitle className="text-base font-semibold truncate">{displayName}</CardTitle>
                    {parseResult && (
                      <p className="text-xs text-muted-foreground mt-0.5">
                        WASM v{parseResult.version} · {parseResult.sections.length} sections · {parseResult.functionCount} functions · {parseResult.globalCount} globals
                      </p>
                    )}
                    {!parseResult && taskDetail && (
                      <p className="text-xs text-muted-foreground mt-0.5">
                        {formatBytes(taskDetail.task.file_size_bytes)} · Uploaded {timeAgo(taskDetail.task.created_at)}
                      </p>
                    )}
                    {analysis && (
                      <div className="flex gap-1.5 mt-2 flex-wrap">
                        {byLevel.critical.length > 0 && <span className="risk-critical text-[10px] font-bold px-2 py-0.5 rounded-full">{byLevel.critical.length} Critical</span>}
                        {byLevel.high.length     > 0 && <span className="risk-high text-[10px] font-bold px-2 py-0.5 rounded-full">{byLevel.high.length} High</span>}
                        {byLevel.medium.length   > 0 && <span className="risk-medium text-[10px] font-bold px-2 py-0.5 rounded-full">{byLevel.medium.length} Medium</span>}
                        {byLevel.info.length     > 0 && <span className="risk-info text-[10px] font-bold px-2 py-0.5 rounded-full">{byLevel.info.length} Info</span>}
                        {analysis.findings.length === 0 && <span className="text-[10px] font-bold px-2 py-0.5 rounded-full text-green-400 bg-green-400/10">✓ Clean</span>}
                      </div>
                    )}
                  </div>
                  {analysis && <RiskGauge score={analysis.riskScore} grade={analysis.grade} />}
                </div>
              </CardHeader>

              <Separator />

              <CardContent className="pt-4">
                <Tabs value={activeTab} onValueChange={setActiveTab}>
                  <div className="overflow-x-auto mb-4">
                    <TabsList className="h-8 text-xs inline-flex">
                      <TabsTrigger value="overview"     className="text-xs px-3 h-7">Overview</TabsTrigger>
                      <TabsTrigger value="imports"      className="text-xs px-3 h-7">Imports/Exports</TabsTrigger>
                      <TabsTrigger value="capabilities" className="text-xs px-3 h-7">Capabilities</TabsTrigger>
                      <TabsTrigger value="findings"     className="text-xs px-3 h-7">
                        Findings{analysis && analysis.findings.length > 0 ? ` (${analysis.findings.length})` : ""}
                      </TabsTrigger>
                      <TabsTrigger value="sections"     className="text-xs px-3 h-7">Sections</TabsTrigger>
                      <TabsTrigger value="strings"      className="text-xs px-3 h-7">Strings</TabsTrigger>
                      <TabsTrigger value="hex"          className="text-xs px-3 h-7">Hex Dump</TabsTrigger>
                      {compareResult && <TabsTrigger value="compare" className="text-xs px-3 h-7">Diff</TabsTrigger>}
                      {backendReport && <TabsTrigger value="backend" className="text-xs px-3 h-7">Backend</TabsTrigger>}
                    </TabsList>
                  </div>

                  {/* Overview */}
                  <TabsContent value="overview" className="space-y-4 mt-0">
                    {parseResult ? (
                      <>
                        <div className="grid grid-cols-2 sm:grid-cols-4 gap-2">
                          {[
                            { icon: Code2,        label: "Functions",   value: parseResult.functionCount },
                            { icon: Cpu,          label: "Globals",     value: parseResult.globalCount },
                            { icon: HardDrive,    label: "Data segs",   value: parseResult.dataSegments },
                            { icon: Layers,       label: "Sections",    value: parseResult.sections.length },
                            { icon: Globe,        label: "Imports",     value: parseResult.imports.length },
                            { icon: ExternalLink, label: "Exports",     value: parseResult.exports.length },
                            { icon: HardDrive,    label: "Memory",      value: parseResult.memoryCount },
                            { icon: ScrollText,   label: "Custom secs", value: parseResult.customSections.length },
                          ].map(({ icon: Icon, label, value }) => (
                            <div key={label} className="flex items-center gap-2 rounded-lg bg-muted/30 border border-border px-3 py-2">
                              <Icon size={13} className="text-muted-foreground shrink-0" />
                              <div>
                                <p className="text-[10px] text-muted-foreground uppercase tracking-wider">{label}</p>
                                <p className="text-xs font-semibold">{value}</p>
                              </div>
                            </div>
                          ))}
                        </div>
                        {!parseResult.valid && (
                          <Alert variant="destructive">
                            <AlertTriangle size={14} />
                            <AlertTitle>Invalid WASM</AlertTitle>
                            <AlertDescription>Invalid magic header — not a valid WASM binary.</AlertDescription>
                          </Alert>
                        )}
                        {parseResult.customSections.length > 0 && (
                          <div>
                            <p className="text-[11px] uppercase tracking-wider text-muted-foreground font-semibold mb-2">Custom Sections</p>
                            <div className="flex flex-wrap gap-1.5">
                              {parseResult.customSections.map((s, i) => <Badge key={i} variant="secondary" className="text-[11px]">{s}</Badge>)}
                            </div>
                          </div>
                        )}
                      </>
                    ) : taskDetail ? (
                      <div className="space-y-3">
                        <InfoAlert text="Drop the same .wasm into Local File above for full binary analysis." />
                        {backendReport && (
                          <div className="space-y-2">
                            <p className="text-xs font-medium text-muted-foreground uppercase tracking-wider">Backend Security Report</p>
                            {Object.entries(backendReport).map(([k, v]) => (
                              <div key={k} className="flex justify-between text-xs border-b border-border/50 pb-1">
                                <span className="text-muted-foreground">{k}</span>
                                <span className="font-medium">{String(v)}</span>
                              </div>
                            ))}
                          </div>
                        )}
                      </div>
                    ) : null}
                  </TabsContent>

                  {/* Imports/Exports */}
                  <TabsContent value="imports" className="mt-0 space-y-4">
                    {parseResult ? (
                      <>
                        <div>
                          <p className="text-[11px] uppercase tracking-wider text-muted-foreground font-semibold mb-2">Imports ({parseResult.imports.length})</p>
                          {parseResult.imports.length === 0 ? <p className="text-xs text-muted-foreground">No imports</p> : (
                            <ScrollArea className="h-48">
                              <div className="divide-y divide-border/50">
                                {parseResult.imports.map((imp, i) => (
                                  <div key={i} className="flex items-center gap-2 py-1.5 px-1 text-xs">
                                    <Badge variant="outline" className="text-[10px] h-4 px-1.5 shrink-0 border-blue-500/30 text-blue-400">{imp.kindName}</Badge>
                                    <span className="text-muted-foreground shrink-0">{imp.module}</span>
                                    <ChevronRight size={11} className="text-muted-foreground/50 shrink-0" />
                                    <span className="font-medium font-mono truncate">{imp.name}</span>
                                  </div>
                                ))}
                              </div>
                            </ScrollArea>
                          )}
                        </div>
                        <Separator />
                        <div>
                          <p className="text-[11px] uppercase tracking-wider text-muted-foreground font-semibold mb-2">Exports ({parseResult.exports.length})</p>
                          {parseResult.exports.length === 0 ? <p className="text-xs text-muted-foreground">No exports</p> : (
                            <ScrollArea className="h-48">
                              <div className="divide-y divide-border/50">
                                {parseResult.exports.map((exp, i) => (
                                  <div key={i} className="flex items-center gap-2 py-1.5 px-1 text-xs">
                                    <Badge variant="outline" className="text-[10px] h-4 px-1.5 shrink-0 border-green-500/30 text-green-400">{exp.kindName}</Badge>
                                    <span className="font-medium font-mono">{exp.name}</span>
                                    <span className="ml-auto text-muted-foreground shrink-0">#{exp.index}</span>
                                  </div>
                                ))}
                              </div>
                            </ScrollArea>
                          )}
                        </div>
                      </>
                    ) : <InfoAlert text="Upload local file to inspect imports and exports." />}
                  </TabsContent>

                  {/* Capabilities */}
                  <TabsContent value="capabilities" className="mt-0 space-y-4">
                    {analysis ? (
                      caps.length === 0 ? (
                        <div className="flex items-center gap-2 text-xs text-green-400 py-4">
                          <CheckCircle size={14} />No suspicious capabilities detected
                        </div>
                      ) : (
                        <>
                          <p className="text-[11px] uppercase tracking-wider text-muted-foreground font-semibold">Capabilities detected from import names</p>
                          {Array.from(new Set(caps.map((c) => c.category))).map((cat) => {
                            const items = caps.filter((c) => c.category === cat);
                            return (
                              <div key={cat}>
                                <p className="text-[11px] text-muted-foreground font-medium mb-1.5">{cat}</p>
                                <div className="flex flex-wrap gap-1.5">{items.map((c, i) => <CapBadge key={i} label={c.label} category={c.category} />)}</div>
                              </div>
                            );
                          })}
                        </>
                      )
                    ) : <InfoAlert text="Upload local file to analyse capabilities." />}
                  </TabsContent>

                  {/* Findings */}
                  <TabsContent value="findings" className="mt-0 space-y-3">
                    {analysis ? (
                      analysis.findings.length === 0 ? (
                        <Alert variant="success">
                          <CheckCircle size={14} /><AlertTitle>No Issues Found</AlertTitle>
                          <AlertDescription>This module passed all security checks.</AlertDescription>
                        </Alert>
                      ) : (
                        <ScrollArea className="h-[420px]">
                          <div className="space-y-2 pr-1">
                            {(["critical", "high", "medium", "info"] as const).flatMap((level) =>
                              (byLevel[level] ?? []).map((f, i) => <FindingRow key={`${level}-${i}`} f={f} />)
                            )}
                          </div>
                        </ScrollArea>
                      )
                    ) : <InfoAlert text="Upload local file to run security analysis." />}
                  </TabsContent>

                  {/* Sections */}
                  <TabsContent value="sections" className="mt-0 space-y-3">
                    {parseResult ? (() => {
                      const total = parseResult.sections.reduce((s, sec) => s + sec.length, 0);
                      return (
                        <>
                          <p className="text-[11px] uppercase tracking-wider text-muted-foreground font-semibold">Section Layout ({parseResult.sections.length} sections)</p>
                          <div className="space-y-2">
                            {parseResult.sections.map((sec, i) => <SectionBar key={i} name={sec.name} size={sec.length} total={total} />)}
                          </div>
                          <Separator />
                          <div className="divide-y divide-border/50">
                            {parseResult.sections.map((sec, i) => (
                              <div key={i} className="flex items-center justify-between py-1.5 text-xs">
                                <div className="flex items-center gap-2">
                                  <Badge variant="outline" className="text-[10px] h-4 px-1.5 font-mono">§{sec.id}</Badge>
                                  <span className="font-medium">{sec.name}</span>
                                </div>
                                <div className="flex items-center gap-4 text-muted-foreground">
                                  <span>offset {sec.offset}</span>
                                  <span className="font-medium text-foreground">{formatBytes(sec.length)}</span>
                                </div>
                              </div>
                            ))}
                          </div>
                        </>
                      );
                    })() : <InfoAlert text="Upload local file to view section layout." />}
                  </TabsContent>

                  {/* Strings */}
                  <TabsContent value="strings" className="mt-0 space-y-3">
                    {parseResult ? (
                      parseResult.strings.length === 0
                        ? <p className="text-xs text-muted-foreground py-4 text-center">No printable strings in data segments</p>
                        : <>
                            <p className="text-[11px] text-muted-foreground">
                              <span className="font-semibold text-foreground">{parseResult.strings.length}</span> strings extracted. <span className="text-yellow-400">Coloured = suspicious pattern</span>
                            </p>
                            <StringScanner strings={parseResult.strings} />
                          </>
                    ) : <InfoAlert text="Upload local file to scan for embedded strings." />}
                  </TabsContent>

                  {/* Hex dump */}
                  <TabsContent value="hex" className="mt-0">
                    {localBytes ? <HexViewer bytes={localBytes} /> : <InfoAlert text="Upload local file to view hex dump." />}
                  </TabsContent>

                  {/* Diff */}
                  {compareResult && (
                    <TabsContent value="compare" className="mt-0 space-y-4">
                      {parseResult ? (() => {
                        const beforeA = analysis ?? { findings: [], riskScore: 0, grade: "A" as const };
                        const afterA  = analyseWasm(compareResult);
                        const diff    = diffModules(parseResult, compareResult, beforeA, afterA);
                        const delta   = afterA.riskScore - beforeA.riskScore;
                        return (
                          <div className="space-y-4">
                            <div className="grid grid-cols-2 gap-3">
                              {[
                                { label: localName,   r: parseResult,   a: beforeA },
                                { label: compareName, r: compareResult, a: afterA },
                              ].map(({ label, r, a }, i) => (
                                <Card key={i} className="p-3">
                                  <p className="text-[11px] text-muted-foreground font-medium mb-2 truncate">{i === 0 ? "A:" : "B:"} {label}</p>
                                  <div className="space-y-1 text-xs">
                                    {[["Functions", r.functionCount], ["Imports", r.imports.length], ["Exports", r.exports.length], ["Risk score", `${a.riskScore} (${a.grade})`]].map(([k, v]) => (
                                      <div key={String(k)} className="flex justify-between"><span className="text-muted-foreground">{k}</span><span>{v}</span></div>
                                    ))}
                                  </div>
                                </Card>
                              ))}
                            </div>
                            <Separator />
                            {diff.addedImports.length > 0 && (
                              <div>
                                <p className="text-[11px] uppercase tracking-wider text-green-400 font-semibold mb-1.5">+ Added imports</p>
                                {diff.addedImports.map((imp, i) => <div key={i} className="text-xs font-mono text-green-300 py-0.5">+ {imp.module}.{imp.name} ({imp.kindName})</div>)}
                              </div>
                            )}
                            {diff.removedImports.length > 0 && (
                              <div>
                                <p className="text-[11px] uppercase tracking-wider text-red-400 font-semibold mb-1.5">− Removed imports</p>
                                {diff.removedImports.map((imp, i) => <div key={i} className="text-xs font-mono text-red-300 py-0.5">− {imp.module}.{imp.name} ({imp.kindName})</div>)}
                              </div>
                            )}
                            {diff.newFindings.length > 0 && (
                              <div>
                                <p className="text-[11px] uppercase tracking-wider text-red-400 font-semibold mb-1.5">New findings in B</p>
                                {diff.newFindings.map((f, i) => <FindingRow key={i} f={f} />)}
                              </div>
                            )}
                            {diff.resolvedFindings.length > 0 && (
                              <div>
                                <p className="text-[11px] uppercase tracking-wider text-green-400 font-semibold mb-1.5">Resolved in B</p>
                                {diff.resolvedFindings.map((f, i) => <FindingRow key={i} f={f} />)}
                              </div>
                            )}
                            {delta !== 0 && (
                              <Alert variant={delta > 0 ? "destructive" : "success"}>
                                <BarChart3 size={14} />
                                <AlertTitle>Risk score {delta > 0 ? "increased" : "decreased"} by {Math.abs(delta)}</AlertTitle>
                                <AlertDescription>{delta > 0 ? "Module B is riskier than A." : "Module B has lower risk than A."}</AlertDescription>
                              </Alert>
                            )}
                          </div>
                        );
                      })() : <InfoAlert text="Load Module A first to compare." />}
                    </TabsContent>
                  )}

                  {/* Backend */}
                  {backendReport && (
                    <TabsContent value="backend" className="mt-0 space-y-3">
                      <p className="text-[11px] uppercase tracking-wider text-muted-foreground font-semibold">Backend Security Report</p>
                      <div className="rounded-lg border border-border overflow-hidden">
                        {Object.entries(backendReport).map(([k, v], i) => (
                          <div key={k} className={cn("flex justify-between gap-3 px-3 py-2 text-xs", i % 2 === 0 ? "bg-muted/10" : "")}>
                            <span className="text-muted-foreground font-medium">{k}</span>
                            <span className="font-mono text-foreground text-right break-all">{String(v)}</span>
                          </div>
                        ))}
                      </div>
                    </TabsContent>
                  )}
                </Tabs>
              </CardContent>
            </Card>
          )}
        </div>
      </div>
    </div>
  );
}

export default function SecurityPage() {
  return (
    <Suspense fallback={<div className="animate-fade-in space-y-5"><Skeleton className="h-8 w-48" /><Skeleton className="h-64 w-full" /></div>}>
      <SecurityHubInner />
    </Suspense>
  );
}
