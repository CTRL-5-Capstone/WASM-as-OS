"use client";

/**
 * Login — exchange admin_key for a JWT token.
 * Redirects to dashboard on success.
 * Also shows "skip" for development/no-auth backends.
 */

import { useState, useEffect } from "react";
import { useRouter } from "next/navigation";
import { Shield, Eye, EyeOff, LogIn, Zap, AlertCircle, Info } from "lucide-react";
import { login, isAuthenticated } from "@/lib/auth";
import { isBackendAlive } from "@/lib/api";
import { cn } from "@/lib/utils";

export default function LoginPage() {
  const router = useRouter();
  const [adminKey, setAdminKey] = useState("");
  const [showKey, setShowKey] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [backendOk, setBackendOk] = useState<boolean | null>(null);

  // If already authenticated, skip to dashboard
  useEffect(() => {
    if (isAuthenticated()) {
      router.replace("/");
    }
    isBackendAlive().then(setBackendOk).catch(() => setBackendOk(false));
  }, [router]);

  const handleLogin = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!adminKey.trim()) return;
    setLoading(true);
    setError(null);
    try {
      await login(adminKey.trim());
      router.replace("/");
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : "Login failed");
    } finally {
      setLoading(false);
    }
  };

  // Dev shortcut: skip auth (for backends without auth enabled)
  const skipAuth = () => {
    // Store a placeholder so isAuthenticated() returns true
    localStorage.setItem("wasmos_token", "dev-no-auth");
    localStorage.setItem("wasmos_token_exp", String(Date.now() + 86400_000));
    router.replace("/");
  };

  return (
    <div className="min-h-screen flex items-center justify-center bg-gradient-to-br from-slate-900 via-indigo-950 to-slate-900 px-4">
      <div className="w-full max-w-md">
        {/* Logo */}
        <div className="text-center mb-8">
          <div className="inline-flex items-center justify-center w-16 h-16 rounded-2xl bg-indigo-600/20 border border-indigo-500/30 mb-4">
            <Zap size={32} className="text-indigo-400" />
          </div>
          <h1 className="text-2xl font-bold text-white">WasmOS</h1>
          <p className="text-sm text-muted-foreground mt-1">WASM Runtime Management Platform</p>
        </div>

        {/* Backend status */}
        <div className={cn(
          "flex items-center gap-2 rounded-lg px-3 py-2 text-xs mb-4",
          backendOk === true ? "bg-emerald-500/10 border border-emerald-500/20 text-emerald-400"
          : backendOk === false ? "bg-red-500/10 border border-red-500/20 text-red-400"
          : "bg-muted border border-border text-muted-foreground"
        )}>
          <span className={cn(
            "w-1.5 h-1.5 rounded-full shrink-0",
            backendOk === true ? "bg-emerald-400 animate-pulse"
            : backendOk === false ? "bg-red-400"
            : "bg-muted-foreground"
          )} />
          {backendOk === true ? "Backend connected"
           : backendOk === false ? "Backend unreachable — check that Rust server is running on :8080"
           : "Checking backend…"}
        </div>

        {/* Login card */}
        <div className="rounded-2xl border border-border/50 bg-card/60 backdrop-blur-xl p-8 shadow-2xl">
          <h2 className="text-lg font-semibold text-foreground mb-1">Sign In</h2>
          <p className="text-xs text-muted-foreground mb-6">
            Enter your admin key to access the dashboard
          </p>

          <form onSubmit={handleLogin} className="space-y-4">
            <div>
              <label className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
                Admin Key
              </label>
              <div className="relative mt-1.5">
                <Shield
                  size={14}
                  className="absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground"
                />
                <input
                  type={showKey ? "text" : "password"}
                  value={adminKey}
                  onChange={(e) => setAdminKey(e.target.value)}
                  placeholder="your-admin-key"
                  autoComplete="current-password"
                  className="w-full rounded-lg border border-border bg-background/60 pl-9 pr-10 py-2.5 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:border-transparent transition-all"
                />
                <button
                  type="button"
                  tabIndex={-1}
                  onClick={() => setShowKey((v) => !v)}
                  className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
                >
                  {showKey ? <EyeOff size={14} /> : <Eye size={14} />}
                </button>
              </div>
            </div>

            {error && (
              <div className="flex items-start gap-2 rounded-lg bg-red-500/10 border border-red-500/20 px-3 py-2.5 text-xs text-red-400">
                <AlertCircle size={14} className="shrink-0 mt-0.5" />
                {error}
              </div>
            )}

            <button
              type="submit"
              disabled={loading || !adminKey.trim()}
              className={cn(
                "w-full flex items-center justify-center gap-2 rounded-lg px-4 py-2.5 text-sm font-semibold transition-all",
                loading || !adminKey.trim()
                  ? "bg-indigo-600/40 text-indigo-300 cursor-not-allowed"
                  : "bg-indigo-600 hover:bg-indigo-500 text-white shadow-lg shadow-indigo-900/40"
              )}
            >
              {loading ? (
                <><span className="w-4 h-4 border-2 border-indigo-300/30 border-t-indigo-300 rounded-full animate-spin" /> Signing in…</>
              ) : (
                <><LogIn size={15} /> Sign In</>
              )}
            </button>
          </form>

          {/* Dev bypass */}
          <div className="mt-6 pt-5 border-t border-border">
            <div className="flex items-start gap-2 rounded-lg bg-amber-500/10 border border-amber-500/20 px-3 py-2.5 text-xs text-amber-400 mb-3">
              <Info size={13} className="shrink-0 mt-0.5" />
              <span>If your backend has auth disabled, use the dev bypass below.</span>
            </div>
            <button
              onClick={skipAuth}
              className="w-full rounded-lg border border-border bg-muted/50 hover:bg-muted px-4 py-2.5 text-xs font-medium text-foreground/80 transition-all"
            >
              Continue without auth (dev mode)
            </button>
          </div>
        </div>

        <p className="text-center text-xs text-muted-foreground/60 mt-6">
          WasmOS Runtime Platform — production-grade WASM execution
        </p>
      </div>
    </div>
  );
}
