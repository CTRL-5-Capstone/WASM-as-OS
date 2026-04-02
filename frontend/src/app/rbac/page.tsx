"use client";

/**
 * RBAC & Audit Log page — R1.5 Multi-Tenant RBAC
 * Real API: GET/POST/DELETE /v1/tenants, GET /v1/audit
 */

import { useState, useEffect, useCallback } from "react";
import {
  Shield, UserCheck, Key, Clock, Activity, Trash2, RefreshCw, Lock, Plus,
} from "lucide-react";
import {
  getTenants, createTenant, deleteTenant, getAuditLogs,
  type Tenant, type AuditLog,
} from "@/lib/api";
import { timeAgo, cn } from "@/lib/utils";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { toast } from "sonner";

type Role = "admin" | "operator" | "viewer" | "auditor";

const ROLE_COLORS: Record<Role, string> = {
  admin:    "bg-red-50 text-red-700 border-red-200",
  operator: "bg-indigo-50 text-indigo-700 border-indigo-200",
  viewer:   "bg-emerald-50 text-emerald-700 border-emerald-200",
  auditor:  "bg-amber-50 text-amber-700 border-amber-200",
};

const ROLE_PERMS: Record<Role, string[]> = {
  admin:    ["Upload modules", "Execute modules", "Delete modules", "Manage tenants", "View audit log", "Snapshot/Restore", "Security scan"],
  operator: ["Upload modules", "Execute modules", "View audit log", "Security scan"],
  viewer:   ["View modules", "View metrics", "View audit log"],
  auditor:  ["View audit log", "View security reports", "View metrics"],
};

export default function RBACPage() {
  const [tenants, setTenants] = useState<Tenant[]>([]);
  const [auditLogs, setAuditLogs] = useState<AuditLog[]>([]);
  const [loadingTenants, setLoadingTenants] = useState(true);
  const [loadingAudit, setLoadingAudit] = useState(true);
  const [filter, setFilter] = useState("");
  const [newName, setNewName] = useState("");
  const [maxTasks, setMaxTasks] = useState("100");
  const [deletingId, setDeletingId] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const [auditPage, setAuditPage] = useState(1);
  const [auditTotal, setAuditTotal] = useState(0);

  // ── Load tenants ───────────────────────────────────────────────

  const loadTenants = useCallback(async () => {
    setLoadingTenants(true);
    try {
      const data = await getTenants();
      setTenants(data ?? []);
    } catch {
      toast.error("Failed to load tenants");
    } finally {
      setLoadingTenants(false);
    }
  }, []);

  // ── Load audit log ─────────────────────────────────────────────

  const loadAudit = useCallback(async (page = 1) => {
    setLoadingAudit(true);
    try {
      const res = await getAuditLogs({ page, per_page: 50 });
      setAuditLogs(res.logs ?? []);
      setAuditTotal(res.total ?? 0);
      setAuditPage(page);
    } catch {
      // Audit endpoint may not exist yet — degrade gracefully
      setAuditLogs([]);
    } finally {
      setLoadingAudit(false);
    }
  }, []);

  useEffect(() => {
    loadTenants();
    loadAudit(1);
  }, [loadTenants, loadAudit]);

  // ── Create tenant ──────────────────────────────────────────────

  const addTenant = async () => {
    if (!newName.trim()) return;
    setCreating(true);
    try {
      const t = await createTenant({
        name: newName.trim(),
        max_tasks: Number(maxTasks) || 100,
      });
      setTenants((prev) => [t, ...prev]);
      toast.success(`Tenant "${t.name}" created`);
      setNewName("");
    } catch (e: unknown) {
      toast.error(`Create failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setCreating(false);
    }
  };

  // ── Delete tenant ──────────────────────────────────────────────

  const removeTenant = async (id: string, name: string) => {
    setDeletingId(id);
    try {
      await deleteTenant(id);
      setTenants((prev) => prev.filter((t) => t.id !== id));
      toast.success(`Tenant "${name}" deleted`);
    } catch (e: unknown) {
      toast.error(`Delete failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setDeletingId(null);
    }
  };

  const filteredAudit = auditLogs.filter(
    (e) => !filter || e.action.toLowerCase().includes(filter.toLowerCase())
  );

  return (
    <div className="animate-fade-in space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold gradient-text flex items-center gap-2">
            <Shield size={26} /> RBAC & Audit
          </h1>
          <p className="text-sm text-slate-500 mt-1">
            Tenant management, role-based access control, and full audit trail
          </p>
        </div>
        <Button variant="outline" size="sm" onClick={() => { loadTenants(); loadAudit(1); }} className="text-xs">
          <RefreshCw size={13} /> Refresh
        </Button>
      </div>

      <div className="grid gap-6 lg:grid-cols-3">
        {/* Roles overview */}
        <div className="space-y-3">
          <h2 className="text-sm font-semibold text-slate-700 flex items-center gap-1.5">
            <Key size={13} className="text-indigo-500" /> Role Definitions
          </h2>
          {(Object.keys(ROLE_PERMS) as Role[]).map((role) => (
            <Card key={role}>
              <CardContent className="p-4">
                <div className="flex items-center gap-2 mb-2">
                  <span className={cn("text-xs font-bold rounded-full border px-2.5 py-0.5", ROLE_COLORS[role])}>
                    {role.toUpperCase()}
                  </span>
                </div>
                <ul className="space-y-1">
                  {ROLE_PERMS[role].map((p) => (
                    <li key={p} className="flex items-center gap-1.5 text-xs text-slate-600">
                      <div className="w-1 h-1 rounded-full bg-slate-400 shrink-0" />
                      {p}
                    </li>
                  ))}
                </ul>
              </CardContent>
            </Card>
          ))}
        </div>

        {/* Tenants */}
        <div className="space-y-3">
          <h2 className="text-sm font-semibold text-slate-700 flex items-center gap-1.5">
            <UserCheck size={13} className="text-indigo-500" /> Tenants
          </h2>

          {loadingTenants ? (
            <div className="space-y-2">
              {[1, 2, 3].map((i) => <div key={i} className="h-16 animate-pulse rounded-xl bg-slate-100" />)}
            </div>
          ) : (
            <ScrollArea className="max-h-80">
              <div className="space-y-2 pr-1">
                {tenants.map((t) => (
                  <Card key={t.id} className={cn("transition-all", t.active && "border-emerald-200")}>
                    <CardContent className="p-3 flex items-center gap-3">
                      <div className={cn(
                        "w-8 h-8 rounded-full flex items-center justify-center text-xs font-bold shrink-0",
                        t.active ? "bg-emerald-100 text-emerald-700" : "bg-slate-100 text-slate-500"
                      )}>
                        {t.name[0].toUpperCase()}
                      </div>
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2">
                          <p className="text-sm font-medium text-slate-800 truncate">{t.name}</p>
                          {t.active && (
                            <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse shrink-0" />
                          )}
                        </div>
                        <div className="flex items-center gap-2">
                          <span className="text-[10px] text-slate-400 font-mono">
                            id: {t.id.slice(0, 8)}…
                          </span>
                          <span className="text-[10px] text-slate-400">max {t.max_tasks} tasks</span>
                        </div>
                      </div>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-7 w-7 text-slate-400 hover:text-red-500"
                        disabled={deletingId === t.id}
                        onClick={() => removeTenant(t.id, t.name)}
                      >
                        {deletingId === t.id
                          ? <RefreshCw size={12} className="animate-spin" />
                          : <Trash2 size={12} />}
                      </Button>
                    </CardContent>
                  </Card>
                ))}

                {tenants.length === 0 && (
                  <div className="rounded-lg border border-dashed border-slate-200 px-4 py-6 text-center text-xs text-slate-400">
                    No tenants found
                  </div>
                )}
              </div>
            </ScrollArea>
          )}

          {/* Add tenant form */}
          <Card className="border-dashed border-slate-300">
            <CardContent className="p-3 space-y-2">
              <p className="text-xs font-medium text-slate-500 flex items-center gap-1">
                <Plus size={11} /> Add Tenant
              </p>
              <Input
                placeholder="Tenant name"
                value={newName}
                onChange={(e) => setNewName(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && addTenant()}
                className="h-8 text-xs"
              />
              <div className="flex gap-2">
                <Input
                  type="number"
                  placeholder="Max tasks"
                  value={maxTasks}
                  onChange={(e) => setMaxTasks(e.target.value)}
                  className="h-8 text-xs"
                  min="1"
                />
                <Button onClick={addTenant} size="sm" className="h-8 text-xs" disabled={creating}>
                  {creating ? <RefreshCw size={11} className="animate-spin" /> : "Add"}
                </Button>
              </div>
            </CardContent>
          </Card>
        </div>

        {/* Security Alerts */}
        <div className="space-y-3">
          <h2 className="text-sm font-semibold text-slate-700 flex items-center gap-1.5">
            <Lock size={13} className="text-red-500" /> Recent Audit Activity
          </h2>
          <Card>
            <CardContent className="p-0">
              <div className="max-h-80 overflow-y-auto divide-y divide-slate-100">
                {loadingAudit ? (
                  <div className="p-6 text-center text-xs text-slate-400">Loading…</div>
                ) : auditLogs.length === 0 ? (
                  <div className="p-6 text-center text-xs text-slate-400">No audit entries yet</div>
                ) : auditLogs.slice(0, 20).map((e) => (
                  <div key={e.id} className="px-3 py-2 text-xs flex items-start gap-2">
                    <span className="w-1.5 h-1.5 rounded-full shrink-0 mt-1 bg-sky-500" />
                    <div className="min-w-0">
                      <p className="text-slate-700 truncate">{e.action}</p>
                      <div className="flex gap-2 text-slate-400">
                        {e.resource && <span>{e.resource}</span>}
                        <span>{timeAgo(e.ts)}</span>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </CardContent>
          </Card>
        </div>
      </div>

      {/* Audit log table */}
      <div>
        <div className="flex items-center justify-between mb-3">
          <h2 className="text-sm font-semibold text-slate-700 flex items-center gap-1.5">
            <Activity size={13} className="text-indigo-500" /> Audit Log
            {auditTotal > 0 && (
              <span className="ml-1 rounded-full bg-slate-100 text-slate-600 text-[10px] px-2 py-0.5">
                {auditTotal} total
              </span>
            )}
          </h2>
          <div className="flex gap-2 items-center">
            <Input
              placeholder="Filter actions…"
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
              className="h-8 w-56 text-xs"
            />
            <Button variant="ghost" size="sm" onClick={() => loadAudit(1)} className="text-xs">
              <RefreshCw size={12} /> Reload
            </Button>
          </div>
        </div>
        <Card>
          <CardContent className="p-0">
            <div className="overflow-x-auto">
              <table className="w-full text-xs">
                <thead className="border-b border-slate-200">
                  <tr className="text-left text-[10px] uppercase text-slate-500">
                    <th className="px-4 py-2.5">Timestamp</th>
                    <th className="px-4 py-2.5">User</th>
                    <th className="px-4 py-2.5">Action</th>
                    <th className="px-4 py-2.5">Resource</th>
                    <th className="px-4 py-2.5">IP</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-slate-100">
                  {loadingAudit ? (
                    <tr>
                      <td colSpan={5} className="px-4 py-8 text-center text-slate-400">Loading audit log…</td>
                    </tr>
                  ) : filteredAudit.length === 0 ? (
                    <tr>
                      <td colSpan={5} className="px-4 py-8 text-center text-slate-400">
                        No audit entries found
                      </td>
                    </tr>
                  ) : filteredAudit.map((e) => (
                    <tr key={e.id} className="hover:bg-slate-50 transition-colors">
                      <td className="px-4 py-2 font-mono text-slate-500 whitespace-nowrap">
                        {new Date(e.ts).toLocaleString()}
                      </td>
                      <td className="px-4 py-2 font-medium text-slate-700">
                        {e.user_name ?? e.tenant_id ?? "—"}
                      </td>
                      <td className="px-4 py-2 text-slate-600 max-w-xs truncate">{e.action}</td>
                      <td className="px-4 py-2 text-slate-500">
                        {e.resource ?? "—"}
                      </td>
                      <td className="px-4 py-2 font-mono text-slate-400">{e.ip_addr ?? "—"}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            {/* Pagination */}
            {auditTotal > 50 && (
              <div className="flex items-center justify-between px-4 py-3 border-t border-slate-100">
                <span className="text-xs text-slate-500">
                  Page {auditPage} of {Math.ceil(auditTotal / 50)}
                </span>
                <div className="flex gap-2">
                  <Button
                    variant="outline"
                    size="sm"
                    className="h-7 text-xs"
                    disabled={auditPage <= 1}
                    onClick={() => loadAudit(auditPage - 1)}
                  >
                    Prev
                  </Button>
                  <Button
                    variant="outline"
                    size="sm"
                    className="h-7 text-xs"
                    disabled={auditPage >= Math.ceil(auditTotal / 50)}
                    onClick={() => loadAudit(auditPage + 1)}
                  >
                    Next
                  </Button>
                </div>
              </div>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
