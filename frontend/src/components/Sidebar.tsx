'use client';

import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { cn } from '@/lib/utils';
import {
  LayoutDashboard,
  ListTodo,
  ShieldAlert,
  Activity,
  Terminal,
  Key,
  Camera,
  GitBranch,
  ScrollText,
  ChevronLeft,
  ChevronRight,
  ChevronDown,
  Cpu,
  Wifi,
  WifiOff,
  BarChart3,
  Layers,
  MonitorDot,
  FlaskConical,
  Shield,
  Play,
  Sun,
  Moon,
} from 'lucide-react';
import { useState, useEffect } from 'react';
import { healthLive } from '@/lib/api';
import { useSidebar } from '@/components/SidebarContext';
import { useTheme } from '@/lib/theme-context';
import { Button } from '@/components/ui/button';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';

const NAV = [
  { href: '/', label: 'Dashboard', icon: LayoutDashboard },
  { href: '/tasks', label: 'Tasks', icon: ListTodo },
  { href: '/security', label: 'Security', icon: ShieldAlert },
  { href: '/monitor', label: 'Monitor', icon: Activity },
  { href: '/terminal', label: 'Terminal', icon: Terminal },
  { href: '/tokens', label: 'Tokens', icon: Key },
  { href: '/snapshots', label: 'Snapshots', icon: Camera },
  { href: '/traces', label: 'Traces', icon: GitBranch },
  { href: '/audit', label: 'Audit Log', icon: ScrollText },
];

const NAV_ADVANCED = [
  { href: '/analytics', label: 'Analytics', icon: BarChart3 },
  { href: '/metrics', label: 'Metrics', icon: Activity },
  { href: '/batch', label: 'Batch Exec', icon: Layers },
  { href: '/command-center', label: 'Command Center', icon: MonitorDot },
  { href: '/tests', label: 'Test Suite', icon: FlaskConical },
  { href: '/rbac', label: 'RBAC & Audit', icon: Shield },
  { href: '/demo', label: 'Demo', icon: Play },
];

export default function Sidebar() {
  const pathname = usePathname();
  const { collapsed, setCollapsed } = useSidebar();
  const { theme, toggleTheme } = useTheme();
  const [online, setOnline] = useState<boolean | null>(null);
  const [advancedOpen, setAdvancedOpen] = useState(() => {
    // Auto-open if current page is in advanced nav
    if (typeof window === 'undefined') return false;
    return NAV_ADVANCED.some(({ href }) => window.location.pathname.startsWith(href));
  });

  useEffect(() => {
    const check = () =>
      healthLive()
        .then(() => setOnline(true))
        .catch(() => setOnline(false));
    check();
    const id = setInterval(check, 8_000);
    return () => clearInterval(id);
  }, []);

  return (
    <TooltipProvider delayDuration={0}>
      <aside
        aria-label="Application sidebar"
        style={{
          width: collapsed
            ? 'var(--sidebar-width-collapsed)'
            : 'var(--sidebar-width)',
        }}
        className="fixed inset-y-0 left-0 z-40 flex flex-col border-r border-border bg-card transition-all duration-300 no-print"
      >
        {/* ── Logo ── */}
        <div className="flex h-16 items-center gap-3 border-b border-border px-4 shrink-0">
          <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-primary text-primary-foreground shadow-lg shadow-primary/25">
            <Cpu size={18} strokeWidth={2.5} />
          </div>
          {!collapsed && (
            <div className="min-w-0">
              <span className="gradient-text block text-[15px] font-bold tracking-tight select-none truncate">
                WasmOS
              </span>
              <span className="block text-[10px] text-muted-foreground font-medium tracking-widest uppercase select-none">
                Runtime
              </span>
            </div>
          )}
        </div>

        {/* ── Nav ── */}
        <nav className="flex-1 overflow-y-auto overflow-x-hidden px-2 py-3 space-y-0.5" aria-label="Main navigation">
          {NAV.map(({ href, label, icon: Icon }) => {
            const active =
              href === '/' ? pathname === '/' : pathname.startsWith(href);

            const link = (
              <Link
                key={href}
                href={href}
                aria-label={label}
                aria-current={active ? 'page' : undefined}
                className={cn(
                  'group flex items-center gap-3 rounded-md px-2.5 py-2 text-[13px] font-medium transition-all duration-150',
                  active
                    ? 'bg-primary/10 text-primary shadow-sm shadow-primary/5'
                    : 'text-muted-foreground hover:bg-accent hover:text-accent-foreground',
                  collapsed && 'justify-center px-0'
                )}
              >
                <Icon
                  size={17}
                  strokeWidth={active ? 2.25 : 1.75}
                  aria-hidden="true"
                  className={cn(
                    'shrink-0 transition-colors',
                    active
                      ? 'text-primary'
                      : 'text-muted-foreground group-hover:text-accent-foreground'
                  )}
                />
                {!collapsed && (
                  <span className="truncate">{label}</span>
                )}
              </Link>
            );

            if (!collapsed) return <div key={href}>{link}</div>;

            return (
              <Tooltip key={href}>
                <TooltipTrigger asChild>{link}</TooltipTrigger>
                <TooltipContent side="right" className="text-xs font-medium">
                  {label}
                </TooltipContent>
              </Tooltip>
            );
          })}

          {/* ── Advanced section ── */}
          <div className="pt-2">
            {!collapsed ? (
              <button
                onClick={() => setAdvancedOpen(o => !o)}
                aria-expanded={advancedOpen}
                aria-controls="advanced-nav"
                className="w-full flex items-center gap-1.5 px-2.5 py-1.5 text-[11px] font-semibold uppercase tracking-widest text-muted-foreground/70 hover:text-muted-foreground transition-colors"
              >
                <span className="flex-1 text-left">Advanced</span>
                <ChevronDown
                  size={12}
                  strokeWidth={2.5}
                  className={cn('transition-transform duration-200', advancedOpen && 'rotate-180')}
                />
              </button>
            ) : (
              <div className="border-t border-border/50 my-1.5" />
            )}

            <div
              id="advanced-nav"
              className={cn(
                'space-y-0.5 overflow-hidden transition-all duration-200',
                collapsed || advancedOpen ? 'max-h-96 opacity-100' : 'max-h-0 opacity-0'
              )}
            >
              {NAV_ADVANCED.map(({ href, label, icon: Icon }) => {
                const active = pathname.startsWith(href);

                const link = (
                  <Link
                    key={href}
                    href={href}
                    aria-label={label}
                    aria-current={active ? 'page' : undefined}
                    className={cn(
                      'group flex items-center gap-3 rounded-md px-2.5 py-2 text-[13px] font-medium transition-all duration-150',
                      active
                        ? 'bg-primary/10 text-primary shadow-sm shadow-primary/5'
                        : 'text-muted-foreground hover:bg-accent hover:text-accent-foreground',
                      collapsed && 'justify-center px-0'
                    )}
                  >
                    <Icon
                      size={17}
                      strokeWidth={active ? 2.25 : 1.75}
                      aria-hidden="true"
                      className={cn(
                        'shrink-0 transition-colors',
                        active
                          ? 'text-primary'
                          : 'text-muted-foreground group-hover:text-accent-foreground'
                      )}
                    />
                    {!collapsed && (
                      <span className="truncate">{label}</span>
                    )}
                  </Link>
                );

                if (!collapsed) return <div key={href}>{link}</div>;

                return (
                  <Tooltip key={href}>
                    <TooltipTrigger asChild>{link}</TooltipTrigger>
                    <TooltipContent side="right" className="text-xs font-medium">
                      {label}
                    </TooltipContent>
                  </Tooltip>
                );
              })}
            </div>
          </div>
        </nav>

        {/* ── Status ── */}
        <div
          role="status"
          aria-live="polite"
          aria-label={`Backend connection: ${online === null ? 'checking' : online ? 'connected' : 'offline'}`}
          className="border-t border-border px-3 py-3 shrink-0"
        >
          <div className={cn('flex items-center gap-2 text-xs', collapsed && 'justify-center')}>
            {online === null ? (
              <div aria-hidden="true" className="h-2 w-2 rounded-full bg-muted-foreground/50 animate-pulse" />
            ) : online ? (
              <Wifi size={14} strokeWidth={2.5} aria-hidden="true" className="text-emerald-400 shrink-0" />
            ) : (
              <WifiOff size={14} strokeWidth={2.5} aria-hidden="true" className="text-destructive shrink-0" />
            )}
            {!collapsed && (
              <span
                className={cn(
                  'font-medium truncate',
                  online === null
                    ? 'text-muted-foreground'
                    : online
                      ? 'text-emerald-400'
                      : 'text-destructive'
                )}
              >
                {online === null ? 'Checking…' : online ? 'Connected' : 'Offline'}
              </span>
            )}
          </div>
        </div>

        {/* ── Theme toggle ── */}
        <Button
          onClick={toggleTheme}
          aria-label={theme === 'dark' ? 'Switch to light mode' : 'Switch to dark mode'}
          variant="ghost"
          size="icon"
          title={theme === 'dark' ? 'Light mode' : 'Dark mode'}
          className="h-10 w-full rounded-none border-t border-border text-muted-foreground hover:text-foreground shrink-0"
        >
          {theme === 'dark' ? (
            <Sun size={15} strokeWidth={2} aria-hidden="true" />
          ) : (
            <Moon size={15} strokeWidth={2} aria-hidden="true" />
          )}
          {!collapsed && (
            <span className="ml-2 text-xs font-medium">
              {theme === 'dark' ? 'Light mode' : 'Dark mode'}
            </span>
          )}
        </Button>

        {/* ── Collapse toggle ── */}
        <Button
          onClick={() => setCollapsed(!collapsed)}
          aria-label={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
          aria-expanded={!collapsed}
          variant="ghost"
          size="icon"
          className="h-10 w-full rounded-none border-t border-border text-muted-foreground hover:text-foreground shrink-0"
        >
          {collapsed ? (
            <ChevronRight size={15} strokeWidth={2.5} aria-hidden="true" />
          ) : (
            <ChevronLeft size={15} strokeWidth={2.5} aria-hidden="true" />
          )}
        </Button>
      </aside>
    </TooltipProvider>
  );
}
