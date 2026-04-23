"use client";

import { useEffect, useState, useCallback, useRef } from "react";
import {
  ScrollText, RefreshCw, Search, X, ChevronLeft, ChevronRight,
  User, Shield, Activity, Clock, Globe, FileText,
  Play, Pause, SkipForward, SkipBack, Rewind, FastForward,
  RotateCcw, Eye, ExternalLink, Timer,
} from "lucide-react";
import { getAuditLogs, type AuditLog } from "@/lib/api";
import { timeAgo, cn } from "@/lib/utils";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";

const ACTION_COLORS: Record<string, string> = {
  create:  "text-green-400 bg-green-400/10 border-green-400/20",
  delete:  "text-red-400 bg-red-400/10 border-red-400/20",
  update:  "text-blue-400 bg-blue-400/10 border-blue-400/20",
  execute: "text-yellow-400 bg-yellow-400/10 border-yellow-400/20",
  login:   "text-purple-400 bg-purple-400/10 border-purple-400/20",
  stop:    "text-orange-400 bg-orange-400/10 border-orange-400/20",
};

function actionColor(action: string) {
  const key = action?.split("_")[0]?.toLowerCase() ?? "";
  return ACTION_COLORS[key] ?? "text-muted-foreground bg-muted/20 border-border";
}

const ROLE_ICON = (role: string) => {
  if (role?.toLowerCase().includes("admin")) return <Shield size={12} className="text-red-400" />;
  if (role?.toLowerCase().includes("user"))  return <User   size={12} className="text-blue-400" />;
  return <User size={12} className="text-muted-foreground" />;
};

// ─── Action → Navigation mapping ────────────────────────────────────

function actionToRoute(log: AuditLog): string | null {
  const action = log.action?.toLowerCase() ?? "";
  const resource = log.resource?.toLowerCase() ?? "";
  if (action.includes("execute") || action.includes("start") || action.includes("stop"))
    return resource ? `/tasks` : "/tasks";
  if (action.includes("snapshot")) return "/snapshots";
  if (action.includes("trace")) return "/traces";
  if (action.includes("tenant") || action.includes("rbac")) return "/rbac";
  if (action.includes("login") || action.includes("auth") || action.includes("token"))
    return "/security";
  if (action.includes("upload") || action.includes("create") || action.includes("delete"))
    return "/tasks";
  return null;
}

// ─── Playback Timeline ──────────────────────────────────────────────

function PlaybackTimeline({
  logs,
  cursor,
  onSeek,
}: {
  logs: AuditLog[];
  cursor: number;
  onSeek: (idx: number) => void;
}) {
  if (logs.length === 0) return null;

  // Build mini-timeline markers
  const total = logs.length;
  return (
    <div className="relative h-8 rounded-lg bg-muted/20 border border-border overflow-hidden">
      {/* Progress fill */}
      <div
        className="absolute left-0 top-0 h-full bg-indigo-500/20 transition-all duration-300"
        style={{ width: `${((cursor + 1) / total) * 100}%` }}
      />
      {/* Event markers */}
      <div className="absolute inset-0 flex items-center">
        {logs.map((log, i) => {
          const left = total <= 1 ? 50 : (i / (total - 1)) * 100;
          const actionKey = log.action?.split("_")[0]?.toLowerCase() ?? "";
          const dotColor =
            actionKey === "delete" ? "bg-red-400"
            : actionKey === "execute" ? "bg-amber-400"
            : actionKey === "create" ? "bg-emerald-400"
            : actionKey === "login" ? "bg-purple-400"
            : "bg-blue-400";
          return (
            <button
              key={log.id}
              onClick={() => onSeek(i)}
              className={cn(
                "absolute h-3 w-3 -translate-x-1/2 rounded-full transition-all border-2",
                i === cursor
                  ? "scale-150 border-white shadow-lg z-10 " + dotColor
                  : i < cursor
                    ? "border-transparent opacity-60 " + dotColor
                    : "border-transparent opacity-30 bg-muted-foreground"
              )}
              style={{ left: `${left}%` }}
              title={`${log.action} – ${log.user_name || "system"}`}
            />
          );
        })}
      </div>
      {/* Current position label */}
      <div className="absolute bottom-0 right-2 text-[9px] text-muted-foreground">
        {cursor + 1} / {total}
      </div>
    </div>
  );
}

// ─── Playback Detail Card ───────────────────────────────────────────

function PlaybackDetailCard({ log, index }: { log: AuditLog; index: number }) {
  const route = actionToRoute(log);
  const actionKey = log.action?.split("_")[0]?.toLowerCase() ?? "";
  const borderColor =
    actionKey === "delete" ? "border-red-500/30"
    : actionKey === "execute" ? "border-amber-500/30"
    : actionKey === "create" ? "border-emerald-500/30"
    : actionKey === "login" ? "border-purple-500/30"
    : "border-indigo-500/30";

  return (
    <Card className={cn("transition-all duration-300 animate-in fade-in slide-in-from-bottom-2", borderColor, "border-2")}>
      <CardContent className="p-5">
        <div className="flex items-start justify-between mb-4">
          <div className="flex items-center gap-3">
            <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-primary/10">
              <span className="text-lg font-bold text-primary">#{index + 1}</span>
            </div>
            <div>
              <div className="flex items-center gap-2">
                <span className={cn(
                  "rounded-full border px-2.5 py-0.5 text-xs font-semibold",
                  ACTION_COLORS[actionKey] ?? "text-muted-foreground bg-muted/20 border-border"
                )}>
                  {log.action}
                </span>
                {log.resource && (
                  <span className="text-sm font-mono text-muted-foreground">{log.resource}</span>
                )}
              </div>
              <p className="text-xs text-muted-foreground mt-0.5">
                {new Date(log.ts).toLocaleString()} ({timeAgo(log.ts)})
              </p>
            </div>
          </div>
          {route && (
            <a
              href={route}
              className="flex items-center gap-1 text-xs text-indigo-400 hover:text-indigo-300 transition-colors"
            >
              <ExternalLink size={11} /> Go to context
            </a>
          )}
        </div>

        {/* Details grid */}
        <div className="grid grid-cols-2 sm:grid-cols-4 gap-4">
          <div className="flex items-center gap-2">
            <User size={13} className="text-muted-foreground" />
            <div>
              <p className="text-[10px] text-muted-foreground uppercase">User</p>
              <p className="text-sm font-medium">{log.user_name || "—"}</p>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <Shield size={13} className="text-muted-foreground" />
            <div>
              <p className="text-[10px] text-muted-foreground uppercase">Role</p>
              <p className="text-sm font-medium">{log.role || "—"}</p>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <Globe size={13} className="text-muted-foreground" />
            <div>
              <p className="text-[10px] text-muted-foreground uppercase">IP Address</p>
              <p className="text-sm font-mono">{log.ip_addr || "—"}</p>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <Clock size={13} className="text-muted-foreground" />
            <div>
              <p className="text-[10px] text-muted-foreground uppercase">Tenant</p>
              <p className="text-sm font-mono">{log.tenant_id?.slice(0, 12) || "global"}</p>
            </div>
          </div>
        </div>

        {/* Visual timeline context */}
        <div className="mt-4 pt-3 border-t border-border">
          <div className="flex items-center gap-2 text-xs text-muted-foreground">
            <Timer size={11} />
            <span>Event timestamp: <span className="font-mono text-foreground">{new Date(log.ts).toISOString()}</span></span>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

export default function AuditPage() {
  const [logs,    setLogs]    = useState<AuditLog[]>([]);
  const [total,   setTotal]   = useState(0);
  const [page,    setPage]    = useState(1);
  const [loading, setLoading] = useState(true);
  const [search,  setSearch]  = useState("");
  const [action,  setAction]  = useState("all");

  // ── Playback state ──
  const [playbackMode, setPlaybackMode] = useState(false);
  const [playbackCursor, setPlaybackCursor] = useState(0);
  const [playing, setPlaying] = useState(false);
  const [playSpeed, setPlaySpeed] = useState(1500); // ms between steps
  const playIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const PER_PAGE = 50;

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const res = await getAuditLogs({
        page,
        per_page: PER_PAGE,
        action: action !== "all" ? action : undefined,
      });
      setLogs(res.logs);
      setTotal(res.total);
    } catch {
      setLogs([]);
    } finally {
      setLoading(false);
    }
  }, [page, action]);

  useEffect(() => { load(); }, [load]);

  const filtered = search
    ? logs.filter((l) =>
        l.action?.toLowerCase().includes(search.toLowerCase()) ||
        l.user_name?.toLowerCase().includes(search.toLowerCase()) ||
        l.resource?.toLowerCase().includes(search.toLowerCase())
      )
    : logs;

  const totalPages = Math.max(1, Math.ceil(total / PER_PAGE));

  // ── Playback controls ──
  const playbackLogs = filtered.length > 0 ? filtered : logs;

  const startPlayback = () => {
    setPlaybackMode(true);
    setPlaybackCursor(0);
    setPlaying(false);
  };

  const stopPlayback = () => {
    setPlaybackMode(false);
    setPlaying(false);
    if (playIntervalRef.current) clearInterval(playIntervalRef.current);
    playIntervalRef.current = null;
  };

  const togglePlay = () => {
    if (playing) {
      setPlaying(false);
      if (playIntervalRef.current) clearInterval(playIntervalRef.current);
      playIntervalRef.current = null;
    } else {
      setPlaying(true);
    }
  };

  const stepForward = () => {
    setPlaybackCursor((c) => Math.min(playbackLogs.length - 1, c + 1));
  };

  const stepBackward = () => {
    setPlaybackCursor((c) => Math.max(0, c - 1));
  };

  const jumpToStart = () => { setPlaybackCursor(0); setPlaying(false); };
  const jumpToEnd = () => { setPlaybackCursor(Math.max(0, playbackLogs.length - 1)); setPlaying(false); };

  // Auto-advance effect
  useEffect(() => {
    if (!playing || !playbackMode) return;
    playIntervalRef.current = setInterval(() => {
      setPlaybackCursor((c) => {
        if (c >= playbackLogs.length - 1) {
          setPlaying(false);
          return c;
        }
        return c + 1;
      });
    }, playSpeed);
    return () => { if (playIntervalRef.current) clearInterval(playIntervalRef.current); };
  }, [playing, playbackMode, playSpeed, playbackLogs.length]);

  const metaCards = [
    { icon: ScrollText, label: "Total Events", value: total },
    { icon: User,       label: "Unique Users", value: new Set(logs.map((l) => l.user_name)).size },
    { icon: Activity,   label: "Actions",      value: new Set(logs.map((l) => l.action)).size },
  ];

  return (
    <div className="animate-fade-in space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold gradient-text flex items-center gap-2">
            <ScrollText size={22} />
            Audit Log
          </h1>
          <p className="mt-0.5 text-sm text-muted-foreground">
            Complete tamper-evident log of all system actions
          </p>
        </div>
        <div className="flex items-center gap-2">
          {!playbackMode ? (
            <Button
              onClick={startPlayback}
              variant="outline"
              size="sm"
              className="h-8 gap-1.5 text-xs"
              disabled={logs.length === 0}
            >
              <Play size={12} /> Replay
            </Button>
          ) : (
            <Button onClick={stopPlayback} variant="outline" size="sm" className="h-8 gap-1.5 text-xs border-red-500/30 text-red-400 hover:bg-red-500/10">
              <X size={12} /> Exit Replay
            </Button>
          )}
          <Button onClick={load} variant="ghost" size="icon" className="h-8 w-8">
            <RefreshCw size={14} className={loading ? "animate-spin" : ""} />
          </Button>
        </div>
      </div>

      {/* ── Playback Mode ── */}
      {playbackMode && playbackLogs.length > 0 && (
        <Card className="border-indigo-500/30 bg-indigo-500/5">
          <CardContent className="p-4 space-y-4">
            {/* Playback header */}
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <Eye size={14} className="text-indigo-400" />
                <span className="text-sm font-semibold text-foreground">Audit Replay</span>
                <Badge variant="outline" className="text-[10px] border-indigo-500/30 text-indigo-400">
                  {playing ? "Playing" : "Paused"}
                </Badge>
              </div>
              <div className="flex items-center gap-2">
                <span className="text-[10px] text-muted-foreground">Speed:</span>
                <select
                  value={playSpeed}
                  onChange={(e) => setPlaySpeed(Number(e.target.value))}
                  className="h-6 rounded border border-border bg-background px-1.5 text-[10px] text-foreground"
                >
                  <option value={3000}>0.5×</option>
                  <option value={1500}>1×</option>
                  <option value={750}>2×</option>
                  <option value={375}>4×</option>
                </select>
              </div>
            </div>

            {/* Timeline */}
            <PlaybackTimeline
              logs={playbackLogs}
              cursor={playbackCursor}
              onSeek={(i) => { setPlaybackCursor(i); setPlaying(false); }}
            />

            {/* Transport controls */}
            <div className="flex items-center justify-center gap-1">
              <TooltipProvider delayDuration={200}>
                {[
                  { icon: Rewind,      action: jumpToStart,  label: "Jump to start", disabled: playbackCursor === 0 },
                  { icon: SkipBack,    action: stepBackward,  label: "Step back",     disabled: playbackCursor === 0 },
                  { icon: playing ? Pause : Play, action: togglePlay, label: playing ? "Pause" : "Play", disabled: false },
                  { icon: SkipForward, action: stepForward,   label: "Step forward",  disabled: playbackCursor >= playbackLogs.length - 1 },
                  { icon: FastForward, action: jumpToEnd,     label: "Jump to end",   disabled: playbackCursor >= playbackLogs.length - 1 },
                ].map(({ icon: Icon, action: act, label, disabled }) => (
                  <Tooltip key={label}>
                    <TooltipTrigger asChild>
                      <Button
                        variant="ghost"
                        size="icon"
                        className={cn("h-8 w-8", label.includes("Play") || label.includes("Pause") ? "h-10 w-10 bg-indigo-500/15" : "")}
                        onClick={act}
                        disabled={disabled}
                      >
                        <Icon size={label.includes("Play") || label.includes("Pause") ? 16 : 13} />
                      </Button>
                    </TooltipTrigger>
                    <TooltipContent side="bottom" className="text-[10px]">{label}</TooltipContent>
                  </Tooltip>
                ))}
              </TooltipProvider>
            </div>

            {/* Current event detail */}
            <PlaybackDetailCard log={playbackLogs[playbackCursor]} index={playbackCursor} />
          </CardContent>
        </Card>
      )}

      {/* Summary cards */}
      <div className="grid grid-cols-3 gap-3">
        {metaCards.map(({ icon: Icon, label, value }) => (
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

      {/* Filters */}
      <div className="flex gap-2 items-center flex-wrap">
        <div className="relative flex-1 min-w-48">
          <Search size={13} className="absolute left-2.5 top-1/2 -translate-y-1/2 text-muted-foreground" />
          <Input
            placeholder="Filter by user, action, resource…"
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
        <Select value={action} onValueChange={(v) => { setAction(v); setPage(1); }}>
          <SelectTrigger className="h-8 w-40 text-xs">
            <SelectValue placeholder="Filter action" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All actions</SelectItem>
            <SelectItem value="create">create</SelectItem>
            <SelectItem value="delete">delete</SelectItem>
            <SelectItem value="update">update</SelectItem>
            <SelectItem value="execute">execute</SelectItem>
            <SelectItem value="login">login</SelectItem>
            <SelectItem value="stop">stop</SelectItem>
          </SelectContent>
        </Select>
        {search && (
          <span className="text-xs text-muted-foreground">{filtered.length} results</span>
        )}
      </div>

      {/* Table */}
      <Card>
        <CardHeader className="pb-3 pt-4 px-4 border-b border-border">
          <div className="grid grid-cols-[1fr_140px_120px_120px_100px] gap-3 text-[10px] uppercase tracking-wider text-muted-foreground font-semibold">
            <span>Action / Resource</span>
            <span>User</span>
            <span>Role</span>
            <span>IP</span>
            <span>When</span>
          </div>
        </CardHeader>
        <CardContent className="p-0">
          {loading ? (
            <div className="space-y-0 divide-y divide-border/50">
              {[...Array(8)].map((_, i) => (
                <div key={i} className="px-4 py-3">
                  <Skeleton className="h-4 w-full" />
                </div>
              ))}
            </div>
          ) : filtered.length === 0 ? (
            <div className="flex items-center justify-center py-16 text-muted-foreground">
              <div className="text-center">
                <ScrollText size={32} className="mx-auto mb-3 opacity-20" />
                <p className="text-sm">No audit entries found</p>
              </div>
            </div>
          ) : (
            <ScrollArea className="h-[500px]">
              <div className="divide-y divide-border/50">
                {filtered.map((log, logIdx) => (
                  <div
                    key={log.id}
                    onClick={() => { if (!playbackMode) { setPlaybackMode(true); setPlaybackCursor(logIdx); setPlaying(false); } else { setPlaybackCursor(logIdx); } }}
                    className={cn(
                      "grid grid-cols-[1fr_140px_120px_120px_100px] gap-3 items-center px-4 py-2.5 text-xs transition-colors cursor-pointer",
                      playbackMode && playbackLogs[playbackCursor]?.id === log.id
                        ? "bg-indigo-500/15 ring-1 ring-indigo-500/30"
                        : "hover:bg-muted/20"
                    )}
                  >
                    {/* Action + resource */}
                    <div className="flex items-center gap-2 min-w-0">
                      <span className={cn(
                        "shrink-0 rounded-full border px-2 py-0.5 text-[10px] font-medium",
                        actionColor(log.action)
                      )}>
                        {log.action}
                      </span>
                      {log.resource && (
                        <span className="text-muted-foreground truncate font-mono text-[11px]">{log.resource}</span>
                      )}
                    </div>
                    {/* User */}
                    <div className="flex items-center gap-1.5 min-w-0">
                      <User size={11} className="text-muted-foreground shrink-0" />
                      <span className="truncate font-medium">{log.user_name || "—"}</span>
                    </div>
                    {/* Role */}
                    <div className="flex items-center gap-1">
                      {ROLE_ICON(log.role)}
                      <span className="text-muted-foreground truncate">{log.role || "—"}</span>
                    </div>
                    {/* IP */}
                    <div className="flex items-center gap-1 text-muted-foreground">
                      <Globe size={10} className="shrink-0" />
                      <span className="font-mono text-[10px] truncate">{log.ip_addr || "—"}</span>
                    </div>
                    {/* Time */}
                    <span className="text-muted-foreground shrink-0">{timeAgo(log.ts)}</span>
                  </div>
                ))}
              </div>
            </ScrollArea>
          )}
        </CardContent>
      </Card>

      {/* Pagination */}
      <div className="flex items-center justify-between text-xs text-muted-foreground">
        <span>Page {page} of {totalPages} · {total} total events</span>
        <div className="flex gap-1">
          <Button
            variant="ghost" size="icon" className="h-7 w-7"
            onClick={() => setPage(Math.max(1, page - 1))}
            disabled={page <= 1}
          >
            <ChevronLeft size={13} />
          </Button>
          <Button
            variant="ghost" size="icon" className="h-7 w-7"
            onClick={() => setPage(Math.min(totalPages, page + 1))}
            disabled={page >= totalPages}
          >
            <ChevronRight size={13} />
          </Button>
        </div>
      </div>
    </div>
  );
}
