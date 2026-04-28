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
// ─── In-source tests (vitest) ────────────────────────────────────────────────
// Stripped from production bundles via `define: { 'import.meta.vitest': 'undefined' }`
// in vitest.config.ts. Run with: npm test
if (import.meta.vitest) {
  const { describe, it, expect, beforeEach, afterEach, vi } = import.meta.vitest;
 
  describe('cn() — class merging', () => {
    it('joins simple class strings', () => {
      expect(cn('a', 'b')).toBe('a b');
    });
 
    it('drops falsy values from clsx', () => {
      expect(cn('a', false && 'b', null, undefined, 'c')).toBe('a c');
    });
 
    it('lets twMerge resolve conflicting Tailwind utilities (last wins)', () => {
      expect(cn('px-2', 'px-4')).toBe('px-4');
    });
  });
 
  describe('formatBytes()', () => {
    it('returns "0 B" for zero', () => {
      expect(formatBytes(0)).toBe('0 B');
    });
 
    it('formats bytes under 1 KB without scaling', () => {
      expect(formatBytes(512)).toBe('512 B');
    });
 
    it('formats kilobytes', () => {
      expect(formatBytes(1024)).toBe('1 KB');
      expect(formatBytes(1536)).toBe('1.5 KB');
    });
 
    it('formats megabytes', () => {
      expect(formatBytes(1024 * 1024)).toBe('1 MB');
      expect(formatBytes(1024 * 1024 * 2.5)).toBe('2.5 MB');
    });
 
    it('formats gigabytes', () => {
      expect(formatBytes(1024 ** 3)).toBe('1 GB');
    });
 
    it('strips trailing zeros via parseFloat', () => {
      expect(formatBytes(1024)).toBe('1 KB');
    });
  });
 
  describe('formatDuration()', () => {
    it('reports microseconds for values under 1000', () => {
      expect(formatDuration(0)).toBe('0\u00b5s');
      expect(formatDuration(750)).toBe('750\u00b5s');
    });
 
    it('reports milliseconds with one decimal between 1k and 1M', () => {
      expect(formatDuration(1000)).toBe('1.0ms');
      expect(formatDuration(12_345)).toBe('12.3ms');
      expect(formatDuration(999_500)).toBe('999.5ms');
    });
 
    it('reports seconds with two decimals at and above 1M', () => {
      expect(formatDuration(1_000_000)).toBe('1.00s');
      expect(formatDuration(2_500_000)).toBe('2.50s');
    });
 
    it('uses the Greek mu (U+00B5), not the micro-sign U+03BC', () => {
      expect(formatDuration(42).codePointAt(2)).toBe(0x00b5);
    });
  });
 
  describe('formatNumber()', () => {
    it('returns the raw string for values under 1k', () => {
      expect(formatNumber(0)).toBe('0');
      expect(formatNumber(999)).toBe('999');
    });
 
    it('uses K suffix for thousands', () => {
      expect(formatNumber(1_000)).toBe('1.0K');
      expect(formatNumber(12_345)).toBe('12.3K');
    });
 
    it('uses M suffix for millions', () => {
      expect(formatNumber(1_000_000)).toBe('1.0M');
      expect(formatNumber(7_500_000)).toBe('7.5M');
    });
 
    it('uses B suffix for billions', () => {
      expect(formatNumber(2_000_000_000)).toBe('2.0B');
    });
  });
 
  describe('timeAgo()', () => {
    beforeEach(() => {
      vi.useFakeTimers();
      vi.setSystemTime(new Date('2026-04-27T12:00:00Z'));
    });
    afterEach(() => vi.useRealTimers());
 
    it('returns "just now" for the most recent minute', () => {
      expect(timeAgo('2026-04-27T11:59:30Z')).toBe('just now');
    });
 
    it('reports minutes for sub-hour durations', () => {
      expect(timeAgo('2026-04-27T11:55:00Z')).toBe('5m ago');
    });
 
    it('reports hours for sub-day durations', () => {
      expect(timeAgo('2026-04-27T09:00:00Z')).toBe('3h ago');
    });
 
    it('reports days for older timestamps', () => {
      expect(timeAgo('2026-04-25T12:00:00Z')).toBe('2d ago');
    });
 
    it('exposes relativeTime as an alias of timeAgo', () => {
      expect(relativeTime).toBe(timeAgo);
    });
  });
 
  describe('statusColor()', () => {
    it.each([
      ['running',   'text-green-400'],
      ['completed', 'text-blue-400'],
      ['failed',    'text-red-400'],
      ['stopped',   'text-yellow-400'],
      ['pending',   'text-muted-foreground'],
    ])('maps %s to %s', (status, expected) => {
      expect(statusColor(status)).toBe(expected);
    });
 
    it('is case-insensitive (backend may send any case)', () => {
      expect(statusColor('RUNNING')).toBe('text-green-400');
      expect(statusColor('Failed')).toBe('text-red-400');
    });
 
    it('falls back to muted-foreground for unknown statuses', () => {
      expect(statusColor('weird-status')).toBe('text-muted-foreground');
    });
 
    it('handles undefined / null without throwing', () => {
      expect(statusColor(undefined as unknown as string)).toBe('text-muted-foreground');
      expect(statusColor(null as unknown as string)).toBe('text-muted-foreground');
    });
  });
 
  describe('statusBg()', () => {
    it('returns the running variant', () => {
      expect(statusBg('running')).toContain('bg-green-500/20');
      expect(statusBg('running')).toContain('border-green-500/30');
    });
 
    it('falls back for unknown statuses', () => {
      expect(statusBg('foo')).toBe('bg-muted/40 text-muted-foreground border-border');
    });
 
    it('is case-insensitive', () => {
      expect(statusBg('FAILED')).toContain('bg-red-500/20');
    });
  });
}