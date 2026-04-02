"use client";

import { useState, useEffect, useCallback } from "react";
import {
  listTokens,
  issueToken,
  revokeToken,
  type TokenSummary,
  type CapabilityVariant,
  type IssueTokenRequest,
} from "@/lib/api";
import { Shield, Plus, Trash2, RefreshCw, Copy, Check } from "lucide-react";

// Keys are snake_case to match the Rust backend's #[serde(rename_all = "snake_case")]
const ALL_CAPABILITIES: CapabilityVariant[] = [
  "task_read", "task_write", "task_execute", "task_delete",
  "metrics_read", "metrics_system", "tenant_admin",
  "snapshot_read", "snapshot_write",
  "terminal_access", "audit_read", "admin",
];

const CAP_COLORS: Record<CapabilityVariant, string> = {
  admin:          "bg-red-900/60 text-red-300 border-red-700",
  tenant_admin:   "bg-orange-900/60 text-orange-300 border-orange-700",
  task_execute:   "bg-yellow-900/60 text-yellow-300 border-yellow-700",
  terminal_access: "bg-purple-900/60 text-purple-300 border-purple-700",
  metrics_system: "bg-blue-900/60 text-blue-300 border-blue-700",
  task_read:      "bg-gray-800 text-gray-300 border-gray-600",
  task_write:     "bg-gray-800 text-gray-300 border-gray-600",
  task_delete:    "bg-gray-800 text-gray-300 border-gray-600",
  metrics_read:   "bg-gray-800 text-gray-300 border-gray-600",
  snapshot_read:  "bg-gray-800 text-gray-300 border-gray-600",
  snapshot_write: "bg-gray-800 text-gray-300 border-gray-600",
  audit_read:     "bg-gray-800 text-gray-300 border-gray-600",
};

/** Human-readable label for display, derived from the snake_case key. */
function capLabel(cap: string): string {
  return cap.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase());
}

function capColor(cap: string): string {
  return CAP_COLORS[cap as CapabilityVariant] ?? "bg-gray-800 text-gray-300 border-gray-600";
}

export default function TokensPage() {
  const [tokens, setTokens] = useState<TokenSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showForm, setShowForm] = useState(false);
  const [newToken, setNewToken] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  // Form state
  const [label, setLabel] = useState("");
  const [subject, setSubject] = useState("");
  const [tenantId, setTenantId] = useState("");
  const [ttlHours, setTtlHours] = useState<number | "">(24);
  const [selectedCaps, setSelectedCaps] = useState<Set<CapabilityVariant>>(new Set());
  const [issuing, setIssuing] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await listTokens();
      setTokens(Array.isArray(data) ? data : []);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { load(); }, [load]);

  const toggleCap = (cap: CapabilityVariant) => {
    setSelectedCaps(prev => {
      const next = new Set(prev);
      if (next.has(cap)) next.delete(cap); else next.add(cap);
      return next;
    });
  };

  const handleIssue = async () => {
    if (!label.trim() || !subject.trim() || selectedCaps.size === 0) return;
    setIssuing(true);
    try {
      const body: IssueTokenRequest = {
        label: label.trim(),
        subject: subject.trim(),
        tenant_id: tenantId.trim() || null,
        capabilities: Array.from(selectedCaps),
        ttl_hours: typeof ttlHours === "number" ? ttlHours : null,
      };
      const res = await issueToken(body);
      // Backend returns token_id; display it as the copyable credential
      setNewToken(res.token_id);
      setShowForm(false);
      setLabel(""); setSubject(""); setTenantId(""); setSelectedCaps(new Set());
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setIssuing(false);
    }
  };

  const handleRevoke = async (id: string) => {
    try {
      await revokeToken(id);
      await load();
    } catch (e) {
      setError(String(e));
    }
  };

  const copyToken = () => {
    if (newToken) {
      navigator.clipboard.writeText(newToken);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  return (
    <div className="space-y-6 max-w-5xl">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold gradient-text flex items-center gap-2">
            <Shield className="h-6 w-6" /> Capability Tokens
          </h1>
          <p className="text-sm text-muted-foreground mt-1">
            Zero-trust access model — fine-grained capability delegation
          </p>
        </div>
        <div className="flex gap-2">
          <button
            onClick={load}
            className="p-2 rounded-lg bg-secondary hover:bg-secondary/80 transition-colors"
            title="Refresh"
          >
            <RefreshCw className="h-4 w-4" />
          </button>
          <button
            onClick={() => setShowForm(true)}
            className="flex items-center gap-2 px-4 py-2 rounded-lg bg-primary hover:bg-primary/80 text-primary-foreground text-sm font-medium transition-colors"
          >
            <Plus className="h-4 w-4" /> Issue Token
          </button>
        </div>
      </div>

      {/* New token banner */}
      {newToken && (
        <div className="rounded-xl border border-green-700 bg-green-950/50 p-4">
          <p className="text-green-400 font-semibold mb-2">✓ Token issued — copy it now (shown once only)</p>
          <div className="flex items-center gap-2 font-mono text-xs bg-black/40 rounded-lg p-3 break-all">
            <span className="flex-1 text-green-300">{newToken}</span>
            <button onClick={copyToken} className="shrink-0 p-1 rounded hover:bg-white/10">
              {copied ? <Check className="h-4 w-4 text-green-400" /> : <Copy className="h-4 w-4 text-gray-400" />}
            </button>
          </div>
          <button onClick={() => setNewToken(null)} className="mt-2 text-xs text-gray-500 hover:text-gray-300">
            Dismiss
          </button>
        </div>
      )}

      {/* Issue form */}
      {showForm && (
        <div className="rounded-xl border border-border bg-card p-6 space-y-4">
          <h2 className="text-lg font-semibold">Issue New Token</h2>

          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium mb-1">Label *</label>
              <input
                value={label}
                onChange={e => setLabel(e.target.value)}
                placeholder="e.g. CI/CD pipeline"
                className="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-primary"
              />
            </div>
            <div>
              <label className="block text-sm font-medium mb-1">Subject *</label>
              <input
                value={subject}
                onChange={e => setSubject(e.target.value)}
                placeholder="e.g. user@example.com"
                className="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-primary"
              />
            </div>
            <div>
              <label className="block text-sm font-medium mb-1">Tenant ID (optional)</label>
              <input
                value={tenantId}
                onChange={e => setTenantId(e.target.value)}
                placeholder="tenant UUID"
                className="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-primary"
              />
            </div>
            <div>
              <label className="block text-sm font-medium mb-1">TTL (hours, blank = never)</label>
              <input
                type="number"
                value={ttlHours}
                onChange={e => setTtlHours(e.target.value === "" ? "" : Number(e.target.value))}
                min={1}
                placeholder="24"
                className="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-primary"
              />
            </div>
          </div>

          <div>
            <label className="block text-sm font-medium mb-2">Capabilities *</label>
            <div className="flex flex-wrap gap-2">
              {ALL_CAPABILITIES.map(cap => (
                <button
                  key={cap}
                  onClick={() => toggleCap(cap)}
                  className={`px-3 py-1 rounded-full text-xs font-medium border transition-all ${
                    selectedCaps.has(cap)
                      ? capColor(cap) + " ring-2 ring-primary"
                      : "bg-gray-800/40 text-gray-500 border-gray-700 hover:border-gray-500"
                  }`}
                >
                  {capLabel(cap)}
                </button>
              ))}
            </div>
            {selectedCaps.size === 0 && (
              <p className="text-xs text-yellow-500 mt-1">Select at least one capability</p>
            )}
          </div>

          {error && <p className="text-sm text-red-400">{error}</p>}

          <div className="flex gap-3 pt-2">
            <button
              onClick={handleIssue}
              disabled={issuing || !label.trim() || !subject.trim() || selectedCaps.size === 0}
              className="px-6 py-2 rounded-lg bg-primary hover:bg-primary/80 text-primary-foreground text-sm font-medium disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
            >
              {issuing ? "Issuing…" : "Issue Token"}
            </button>
            <button
              onClick={() => { setShowForm(false); setError(null); }}
              className="px-6 py-2 rounded-lg bg-secondary hover:bg-secondary/80 text-sm transition-colors"
            >
              Cancel
            </button>
          </div>
        </div>
      )}

      {/* Token list */}
      {loading ? (
        <div className="text-center py-12 text-muted-foreground">Loading tokens…</div>
      ) : tokens.length === 0 ? (
        <div className="text-center py-12 text-muted-foreground">
          <Shield className="mx-auto h-10 w-10 mb-3 opacity-30" />
          <p>No capability tokens issued yet.</p>
        </div>
      ) : (
        <div className="space-y-3">
          {tokens.map(t => (
            <div
              key={t.id}
              className={`rounded-xl border ${t.revoked ? "border-gray-700 opacity-50" : "border-border"} bg-card p-4`}
            >
              <div className="flex items-start justify-between gap-4">
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2 flex-wrap">
                    <span className="font-semibold text-sm">{t.label}</span>
                    {t.revoked && (
                      <span className="px-2 py-0.5 rounded-full bg-red-900/50 text-red-400 text-xs border border-red-700">
                        REVOKED
                      </span>
                    )}
                    {t.expires_at && (
                      <span className="text-xs text-muted-foreground">
                        expires {new Date(t.expires_at).toLocaleString()}
                      </span>
                    )}
                    {!t.expires_at && (
                      <span className="text-xs text-muted-foreground">no expiry</span>
                    )}
                  </div>
                  <p className="text-xs text-muted-foreground mt-1">
                    subject: <span className="text-foreground/70">{t.subject}</span>
                    {t.tenant_id && <> · tenant: <span className="text-foreground/70">{t.tenant_id}</span></>}
                  </p>
                  <p className="text-xs text-muted-foreground mt-1 font-mono opacity-60">
                    id: {t.id}
                  </p>
                  <div className="flex flex-wrap gap-1 mt-2">
                    {t.capabilities.map(cap => (
                      <span
                        key={cap}
                        className={`px-2 py-0.5 rounded-full text-xs border ${capColor(cap)}`}
                      >
                        {capLabel(cap)}
                      </span>
                    ))}
                  </div>
                </div>
                {!t.revoked && (
                  <button
                    onClick={() => handleRevoke(t.id)}
                    className="shrink-0 p-2 rounded-lg text-red-400 hover:bg-red-900/30 transition-colors"
                    title="Revoke token"
                  >
                    <Trash2 className="h-4 w-4" />
                  </button>
                )}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
