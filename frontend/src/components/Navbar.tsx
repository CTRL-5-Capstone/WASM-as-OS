'use client';

import { useState, useEffect } from 'react';
import Link from 'next/link';
import { usePathname, useRouter } from 'next/navigation';
import { Menu, X, LogOut, Wifi, WifiOff, Loader2 } from 'lucide-react';
import { cn } from '@/lib/utils';
import { isAuthenticated, logout } from '@/lib/auth';
import { isBackendAlive } from '@/lib/api';
import { useWebSocket } from '@/lib/use-websocket';
import type { WsStatus } from '@/lib/use-websocket';

const NAV = [
  { href: '/', label: 'Dashboard' },
  { href: '/command-center', label: '⌘ Command Center' },
  { href: '/tasks', label: 'Tasks' },
  { href: '/tests', label: 'Tests' },
  { href: '/monitor', label: 'Monitor' },
  { href: '/terminal', label: 'Terminal' },
  { href: '/batch', label: 'Batch' },
  { href: '/snapshots', label: 'Snapshots' },
  { href: '/rbac', label: 'RBAC' },
  { href: '/demo', label: 'Demo' },
];

const WS_STATUS_UI: Record<WsStatus, { color: string; icon: React.ReactNode; title: string }> = {
  connected: { color: "text-emerald-400", icon: <Wifi size={13} strokeWidth={2.5} />, title: "Live — WebSocket connected" },
  connecting: { color: "text-amber-400", icon: <Loader2 size={13} strokeWidth={2.5} className="animate-spin" />, title: "Connecting…" },
  disconnected: { color: "text-muted-foreground", icon: <WifiOff size={13} strokeWidth={2.5} />, title: "Disconnected — reconnecting" },
  error: { color: "text-destructive", icon: <WifiOff size={13} strokeWidth={2.5} />, title: "WebSocket error" },
};

export default function Navbar() {
  const pathname = usePathname();
  const router = useRouter();
  const [open, setOpen] = useState(false);
  const [authed, setAuthed] = useState(false);
  const [backendOk, setBackendOk] = useState<boolean | null>(null);

  const { status: wsStatus } = useWebSocket({ silent: false });

  useEffect(() => {
    setAuthed(isAuthenticated());
    isBackendAlive().then(setBackendOk).catch(() => setBackendOk(false));

    const handle = () => { setAuthed(false); router.replace('/login'); };
    window.addEventListener('wasmos:unauthorized', handle);
    return () => window.removeEventListener('wasmos:unauthorized', handle);
  }, [router]);

  const handleLogout = async () => {
    await logout();
    setAuthed(false);
    router.replace('/login');
  };

  const wsUi = WS_STATUS_UI[wsStatus];

  return (
    <header className="sticky top-0 z-50 border-b border-border bg-card/95 backdrop-blur-xl">
      <div className="mx-auto flex h-14 max-w-[1400px] items-center justify-between px-6 md:px-12">
        <Link href="/" className="flex items-center gap-3 shrink-0">
          <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-primary text-primary-foreground text-sm font-extrabold shadow-lg shadow-primary/25">
            W
          </div>
          <span className="gradient-text text-lg font-bold tracking-tight">WasmOS</span>
        </Link>

        {/* Desktop nav */}
        <nav className="hidden items-center gap-0.5 md:flex overflow-x-auto scrollbar-none max-w-[700px]">
          {NAV.map(({ href, label }) => {
            const active = href === '/' ? pathname === '/' : pathname.startsWith(href);
            return (
              <Link
                key={href}
                href={href}
                className={cn(
                  'whitespace-nowrap rounded-md px-3 py-1.5 text-xs font-medium transition-all',
                  active
                    ? 'bg-primary/15 text-primary'
                    : 'text-muted-foreground hover:bg-accent hover:text-accent-foreground'
                )}
              >
                {label}
              </Link>
            );
          })}
        </nav>

        {/* Right cluster */}
        <div className="hidden md:flex items-center gap-3 shrink-0">
          <span
            title={wsUi.title}
            className={cn("flex items-center gap-1 text-xs font-medium", wsUi.color)}
          >
            {wsUi.icon}
            <span className="hidden lg:inline">
              {wsStatus === 'connected' ? 'Live' : wsStatus}
            </span>
          </span>

          {backendOk !== null && (
            <span
              title={backendOk ? "Backend online" : "Backend offline"}
              className={cn(
                "w-2 h-2 rounded-full",
                backendOk ? "bg-emerald-400 status-dot-running" : "bg-destructive"
              )}
            />
          )}

          {authed && (
            <button
              onClick={handleLogout}
              title="Sign out"
              className="flex items-center gap-1.5 rounded-md px-2.5 py-1.5 text-xs font-medium text-muted-foreground hover:bg-destructive/10 hover:text-destructive transition-colors"
            >
              <LogOut size={13} strokeWidth={2.5} /> <span className="hidden lg:inline">Sign out</span>
            </button>
          )}
          {!authed && (
            <Link
              href="/login"
              className="rounded-md bg-primary px-3 py-1.5 text-xs font-semibold text-primary-foreground hover:bg-primary/90 transition-colors"
            >
              Sign in
            </Link>
          )}
        </div>

        {/* Mobile hamburger */}
        <button
          onClick={() => setOpen((p) => !p)}
          className="md:hidden rounded-md border border-border p-2 text-muted-foreground hover:bg-accent transition-colors"
          aria-label="Toggle menu"
        >
          {open ? <X size={18} strokeWidth={2.5} /> : <Menu size={18} strokeWidth={2.5} />}
        </button>
      </div>

      {/* Mobile dropdown */}
      {open && (
        <div className="md:hidden border-t border-border bg-card/98 backdrop-blur-xl">
          <nav className="mx-auto max-w-[1400px] px-4 py-3 grid grid-cols-2 gap-1">
            {NAV.map(({ href, label }) => {
              const active = href === '/' ? pathname === '/' : pathname.startsWith(href);
              return (
                <Link
                  key={href}
                  href={href}
                  onClick={() => setOpen(false)}
                  className={cn(
                    'rounded-md px-3 py-2.5 text-sm font-medium transition-all',
                    active
                      ? 'bg-primary/15 text-primary'
                      : 'text-muted-foreground hover:bg-accent hover:text-accent-foreground'
                  )}
                >
                  {label}
                </Link>
              );
            })}
            {authed && (
              <button
                onClick={() => { setOpen(false); handleLogout(); }}
                className="rounded-md px-3 py-2.5 text-sm font-medium text-destructive hover:bg-destructive/10 text-left"
              >
                Sign out
              </button>
            )}
          </nav>
        </div>
      )}
    </header>
  );
}
