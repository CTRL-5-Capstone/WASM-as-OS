import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

export function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
}

export function formatDuration(us: number): string {
  if (us < 1000) return `${us}\u00b5s`;
  if (us < 1_000_000) return `${(us / 1000).toFixed(1)}ms`;
  return `${(us / 1_000_000).toFixed(2)}s`;
}

export function formatNumber(n: number): string {
  if (n >= 1_000_000_000) return `${(n / 1_000_000_000).toFixed(1)}B`;
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return n.toString();
}

export function timeAgo(dateString: string): string {
  const date = new Date(dateString);
  const now = new Date();
  const seconds = Math.floor((now.getTime() - date.getTime()) / 1000);
  if (seconds < 60) return 'just now';
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
  return `${Math.floor(seconds / 86400)}d ago`;
}

/** Alias kept for backward compat */
export const relativeTime = timeAgo;

// Backend serialises TaskStatus with #[serde(rename_all = "lowercase")],
// so statuses arrive as "running", "completed", etc.  All lookups are
// normalised to lowercase so these helpers accept either case.

export function statusColor(status: string): string {
  const map: Record<string, string> = {
    running:   'text-green-400',
    completed: 'text-blue-400',
    failed:    'text-red-400',
    stopped:   'text-yellow-400',
    pending:   'text-muted-foreground',
  };
  return map[status?.toLowerCase()] || 'text-muted-foreground';
}

export function statusBg(status: string): string {
  const map: Record<string, string> = {
    running:   'bg-green-500/20 text-green-400 border-green-500/30',
    completed: 'bg-blue-500/20 text-blue-400 border-blue-500/30',
    failed:    'bg-red-500/20 text-red-400 border-red-500/30',
    stopped:   'bg-yellow-500/20 text-yellow-400 border-yellow-500/30',
    pending:   'bg-muted/40 text-muted-foreground border-border',
  };
  return map[status?.toLowerCase()] || 'bg-muted/40 text-muted-foreground border-border';
}
