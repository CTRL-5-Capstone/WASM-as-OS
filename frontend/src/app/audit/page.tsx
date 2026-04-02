"use client";

import { useEffect, useState, useCallback } from "react";
import {
  ScrollText, RefreshCw, Search, X, ChevronLeft, ChevronRight,
  User, Shield, Activity, Clock, Globe, FileText,
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

export default function AuditPage() {
  const [logs,    setLogs]    = useState<AuditLog[]>([]);
  const [total,   setTotal]   = useState(0);
  const [page,    setPage]    = useState(1);
  const [loading, setLoading] = useState(true);
  const [search,  setSearch]  = useState("");
  const [action,  setAction]  = useState("all");
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

  const metaCards = [
    { icon: ScrollText, label: "Total Events", value: total },
    { icon: User,       label: "Unique Users", value: new Set(logs.map((l) => l.user_name)).size },
    { icon: Activity,   label: "Actions",      value: new Set(logs.map((l) => l.action)).size },
  ];

  return (
    <div className="animate-fade-in space-y-5">
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
        <Button onClick={load} variant="ghost" size="icon" className="h-9 w-9">
          <RefreshCw size={14} className={loading ? "animate-spin" : ""} />
        </Button>
      </div>

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
                {filtered.map((log) => (
                  <div
                    key={log.id}
                    className="grid grid-cols-[1fr_140px_120px_120px_100px] gap-3 items-center px-4 py-2.5 text-xs hover:bg-muted/20 transition-colors"
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
