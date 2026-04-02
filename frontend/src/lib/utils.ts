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
    pending:   'text-slate-400',
  };
  return map[status?.toLowerCase()] || 'text-slate-400';
}

export function statusBg(status: string): string {
  const map: Record<string, string> = {
    running:   'bg-green-500/20 text-green-400 border-green-500/30',
    completed: 'bg-blue-500/20 text-blue-400 border-blue-500/30',
    failed:    'bg-red-500/20 text-red-400 border-red-500/30',
    stopped:   'bg-yellow-500/20 text-yellow-400 border-yellow-500/30',
    pending:   'bg-slate-500/20 text-slate-400 border-slate-500/30',
  };
  return map[status?.toLowerCase()] || 'bg-slate-500/20 text-slate-400 border-slate-500/30';
}
// ── Tests (only run by vitest, stripped from production builds) ──────────
if (import.meta.vitest) {
  const { describe, it, expect } = import.meta.vitest;
 
  describe('formatBytes', () => {
    it('returns "0 B" for zero', () => {
      expect(formatBytes(0)).toBe('0 B');
    });
 
    it('formats raw bytes', () => {
      expect(formatBytes(500)).toBe('500 B');
    });
 
    it('formats exactly 1 KB', () => {
      expect(formatBytes(1024)).toBe('1 KB');
    });
 
    it('formats fractional KB', () => {
      expect(formatBytes(1536)).toBe('1.5 KB');
    });
 
    it('formats exactly 1 MB', () => {
      expect(formatBytes(1024 * 1024)).toBe('1 MB');
    });
 
    it('formats 50 MB', () => {
      expect(formatBytes(52428800)).toBe('50 MB');
    });
 
    it('formats exactly 1 GB', () => {
      expect(formatBytes(1024 * 1024 * 1024)).toBe('1 GB');
    });
  });
 
  describe('formatDuration', () => {
    it('formats microseconds', () => {
      expect(formatDuration(500)).toBe('500µs');
    });
 
    it('formats boundary at 999µs', () => {
      expect(formatDuration(999)).toBe('999µs');
    });
 
    it('formats exactly 1ms', () => {
      expect(formatDuration(1000)).toBe('1.0ms');
    });
 
    it('formats milliseconds', () => {
      expect(formatDuration(5000)).toBe('5.0ms');
    });
 
    it('formats exactly 1s', () => {
      expect(formatDuration(1_000_000)).toBe('1.00s');
    });
 
    it('formats seconds', () => {
      expect(formatDuration(2_500_000)).toBe('2.50s');
    });
  });
 
  describe('formatNumber', () => {
    it('returns plain number for < 1000', () => {
      expect(formatNumber(42)).toBe('42');
      expect(formatNumber(0)).toBe('0');
      expect(formatNumber(999)).toBe('999');
    });
 
    it('formats thousands', () => {
      expect(formatNumber(1000)).toBe('1.0K');
      expect(formatNumber(5500)).toBe('5.5K');
    });
 
    it('formats millions', () => {
      expect(formatNumber(1_000_000)).toBe('1.0M');
      expect(formatNumber(2_500_000)).toBe('2.5M');
    });
 
    it('formats billions', () => {
      expect(formatNumber(1_000_000_000)).toBe('1.0B');
    });
  });
 
  describe('timeAgo', () => {
    it('returns "just now" for current time', () => {
      expect(timeAgo(new Date().toISOString())).toBe('just now');
    });
 
    it('returns minutes ago', () => {
      const d = new Date(Date.now() - 5 * 60 * 1000).toISOString();
      expect(timeAgo(d)).toBe('5m ago');
    });
 
    it('returns hours ago', () => {
      const d = new Date(Date.now() - 2 * 3600 * 1000).toISOString();
      expect(timeAgo(d)).toBe('2h ago');
    });
 
    it('returns days ago', () => {
      const d = new Date(Date.now() - 3 * 86400 * 1000).toISOString();
      expect(timeAgo(d)).toBe('3d ago');
    });
  });
 
  describe('statusColor', () => {
    it('returns green for running', () => {
      expect(statusColor('running')).toBe('text-green-400');
    });
 
    it('returns red for failed', () => {
      expect(statusColor('failed')).toBe('text-red-400');
    });
 
    it('returns default for unknown status', () => {
      expect(statusColor('unknown')).toBe('text-slate-400');
    });
 
    it('handles uppercase input', () => {
      expect(statusColor('RUNNING')).toBe('text-green-400');
    });
  });
 
  describe('statusBg', () => {
    it('returns green bg for running', () => {
      expect(statusBg('running')).toContain('bg-green');
    });
 
    it('returns default for unknown', () => {
      expect(statusBg('xyz')).toContain('bg-slate');
    });
  });
}
