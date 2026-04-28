/**
 * Client-side stale-while-revalidate (SWR) cache for WasmOS API responses.
 *
 * ## Behaviour
 * - GET requests are cached by URL for a configurable TTL.
 * - While fresh (within TTL): served instantly from memory.
 * - Stale (between TTL and 2×TTL): returned immediately AND a background
 *   revalidation is fired so the next caller sees fresh data.
 * - Expired (>2×TTL): cache miss, blocks on fresh fetch.
 *
 * ## Usage (via api.ts integration)
 * The cache is applied automatically inside `request()` for GET calls.
 * You do not call it directly.
 *
 * ## Cache invalidation
 * `invalidate(pattern)` removes all entries whose key starts with `pattern`.
 * Call it after any write operation so reads following a mutation are fresh.
 */

interface CacheEntry<T> {
  data: T;
  fetchedAt: number; // ms since epoch
  ttl: number;       // ms
}

// Default TTLs matching the backend cache headers
const DEFAULT_TTL_MS: Record<string, number> = {
  "/v1/stats":         10_000,
  "/v1/tasks":         15_000,
  "/v1/tokens":        60_000,
  "/v1/traces":         5_000,
  "/v1/scheduler":      5_000,
};
const FALLBACK_TTL_MS = 15_000;
const STALE_MULTIPLIER = 2; // serve stale up to 2× TTL before evicting

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const store = new Map<string, CacheEntry<any>>();

/** Return the configured TTL for a given URL path. */
function ttlFor(path: string): number {
  for (const [prefix, ms] of Object.entries(DEFAULT_TTL_MS)) {
    if (path.startsWith(prefix)) return ms;
  }
  return FALLBACK_TTL_MS;
}

/** Read from cache. Returns `{ data, stale }` or `null` on miss/expired. */
export function cacheGet<T>(key: string): { data: T; stale: boolean } | null {
  const entry = store.get(key) as CacheEntry<T> | undefined;
  if (!entry) return null;

  const age = Date.now() - entry.fetchedAt;
  if (age > entry.ttl * STALE_MULTIPLIER) {
    // Fully expired — evict
    store.delete(key);
    return null;
  }
  return { data: entry.data, stale: age > entry.ttl };
}

/** Write a value into the cache. */
export function cacheSet<T>(key: string, data: T, ttlMs?: number): void {
  store.set(key, {
    data,
    fetchedAt: Date.now(),
    ttl: ttlMs ?? ttlFor(key),
  });
}

/** Remove all entries whose key starts with `prefix`. */
export function invalidate(prefix: string): void {
  const toDelete: string[] = [];
  store.forEach((_, key) => {
    if (key.startsWith(prefix)) toDelete.push(key);
  });
  toDelete.forEach((key) => store.delete(key));
}

/** Clear the entire cache (e.g. on logout). */
export function clearAll(): void {
  store.clear();
}

/**
 * Wrap a fetch function with SWR semantics.
 *
 * - Returns cached data immediately when fresh or stale (stale triggers background refetch).
 * - Falls through to `fetchFn` on full cache miss.
 * - Prevents concurrent duplicate revalidation requests via an in-flight set.
 */
const _revalidating = new Set<string>();

export async function withSWR<T>(
  key: string,
  fetchFn: () => Promise<T>,
  ttlMs?: number,
): Promise<T> {
  const cached = cacheGet<T>(key);

  if (cached && !cached.stale) {
    // Cache hit — serve fresh data
    return cached.data;
  }

  if (cached && cached.stale) {
    // Stale — return immediately, fire background revalidation
    if (!_revalidating.has(key)) {
      _revalidating.add(key);
      fetchFn()
        .then((fresh) => cacheSet(key, fresh, ttlMs))
        .catch(() => { /* background errors are silent */ })
        .finally(() => _revalidating.delete(key));
    }
    return cached.data;
  }

  // Cache miss — fetch, store, return
  const data = await fetchFn();
  cacheSet(key, data, ttlMs);
  return data;
}

// ─── In-source tests (vitest) ────────────────────────────────────────────────
// Stripped from production via `define: { 'import.meta.vitest': 'undefined' }`.
// Run with: npm test
if (import.meta.vitest) {
  const { describe, it, expect, beforeEach, afterEach, vi } = import.meta.vitest;
 
  beforeEach(() => {
    clearAll();
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-04-27T12:00:00Z'));
  });
  afterEach(() => vi.useRealTimers());
 
  describe('cacheSet / cacheGet basics', () => {
    it('returns null on a miss', () => {
      expect(cacheGet('/v1/tasks')).toBeNull();
    });
 
    it('returns the stored value as fresh immediately after writing', () => {
      cacheSet('/v1/tasks', [{ id: 'a' }]);
      const hit = cacheGet<{ id: string }[]>('/v1/tasks');
      expect(hit).not.toBeNull();
      expect(hit!.data).toEqual([{ id: 'a' }]);
      expect(hit!.stale).toBe(false);
    });
 
    it('uses the path-specific TTL (15s for /v1/tasks)', () => {
      cacheSet('/v1/tasks', 'x');
      vi.advanceTimersByTime(14_000);
      expect(cacheGet('/v1/tasks')!.stale).toBe(false);
      vi.advanceTimersByTime(2_000);
      expect(cacheGet('/v1/tasks')!.stale).toBe(true);
    });
 
    it('uses the fallback TTL (15s) for unknown paths', () => {
      cacheSet('/v1/unknown', 'x');
      vi.advanceTimersByTime(14_000);
      expect(cacheGet('/v1/unknown')!.stale).toBe(false);
      vi.advanceTimersByTime(2_000);
      expect(cacheGet('/v1/unknown')!.stale).toBe(true);
    });
 
    it('honours an explicit ttlMs override', () => {
      cacheSet('/v1/tasks', 'x', 1_000);
      vi.advanceTimersByTime(900);
      expect(cacheGet('/v1/tasks')!.stale).toBe(false);
      vi.advanceTimersByTime(200);
      expect(cacheGet('/v1/tasks')!.stale).toBe(true);
    });
 
    it('treats values older than 2× TTL as evicted (returns null)', () => {
      cacheSet('/v1/tasks', 'x');
      vi.advanceTimersByTime(31_000);
      expect(cacheGet('/v1/tasks')).toBeNull();
    });
 
    it('uses the shorter 5s TTL for /v1/traces', () => {
      cacheSet('/v1/traces', 'x');
      vi.advanceTimersByTime(4_000);
      expect(cacheGet('/v1/traces')!.stale).toBe(false);
      vi.advanceTimersByTime(2_000);
      expect(cacheGet('/v1/traces')!.stale).toBe(true);
    });
  });
 
  describe('invalidate()', () => {
    it('removes only entries whose keys match the prefix', () => {
      cacheSet('/v1/tasks', 'a');
      cacheSet('/v1/tasks/abc', 'b');
      cacheSet('/v1/stats', 'c');
 
      invalidate('/v1/tasks');
 
      expect(cacheGet('/v1/tasks')).toBeNull();
      expect(cacheGet('/v1/tasks/abc')).toBeNull();
      expect(cacheGet('/v1/stats')!.data).toBe('c');
    });
 
    it('is a no-op when no keys match', () => {
      cacheSet('/v1/stats', 'c');
      invalidate('/nothing-here');
      expect(cacheGet('/v1/stats')!.data).toBe('c');
    });
  });
 
  describe('clearAll()', () => {
    it('drops every entry', () => {
      cacheSet('/a', 1);
      cacheSet('/b', 2);
      clearAll();
      expect(cacheGet('/a')).toBeNull();
      expect(cacheGet('/b')).toBeNull();
    });
  });
 
  describe('withSWR()', () => {
    it('runs the fetcher and caches the result on a cold miss', async () => {
      const fetcher = vi.fn().mockResolvedValue({ ok: true });
      const result = await withSWR('/v1/tasks', fetcher);
 
      expect(result).toEqual({ ok: true });
      expect(fetcher).toHaveBeenCalledTimes(1);
      expect(cacheGet('/v1/tasks')!.data).toEqual({ ok: true });
    });
 
    it('serves cached data without re-fetching while fresh', async () => {
      cacheSet('/v1/tasks', { cached: true });
      const fetcher = vi.fn().mockResolvedValue({ cached: false });
 
      const result = await withSWR('/v1/tasks', fetcher);
 
      expect(result).toEqual({ cached: true });
      expect(fetcher).not.toHaveBeenCalled();
    });
 
    it('returns stale data immediately and revalidates in the background', async () => {
      cacheSet('/v1/tasks', { v: 1 });
      vi.advanceTimersByTime(20_000);
      expect(cacheGet('/v1/tasks')!.stale).toBe(true);
 
      let resolve!: (val: { v: number }) => void;
      const fetcher = vi.fn(
        () => new Promise<{ v: number }>((r) => { resolve = r; }),
      );
 
      const immediate = await withSWR('/v1/tasks', fetcher);
 
      expect(immediate).toEqual({ v: 1 });
      expect(fetcher).toHaveBeenCalledTimes(1);
 
      resolve({ v: 2 });
      await vi.waitFor(() => {
        expect(cacheGet('/v1/tasks')!.data).toEqual({ v: 2 });
      });
    });
 
    it('deduplicates concurrent background revalidations for the same key', async () => {
      cacheSet('/v1/tasks', { v: 1 });
      vi.advanceTimersByTime(20_000);
 
      const fetcher = vi.fn().mockResolvedValue({ v: 2 });
      await withSWR('/v1/tasks', fetcher);
      await withSWR('/v1/tasks', fetcher);
 
      expect(fetcher).toHaveBeenCalledTimes(1);
    });
 
    it('swallows background errors silently (caller still gets stale data)', async () => {
      cacheSet('/v1/tasks', { v: 1 });
      vi.advanceTimersByTime(20_000);
 
      const fetcher = vi.fn().mockRejectedValue(new Error('boom'));
      const result = await withSWR('/v1/tasks', fetcher);
      expect(result).toEqual({ v: 1 });
      await vi.waitFor(() => {
        expect(fetcher).toHaveBeenCalled();
      });
    });
 
    it('blocks on the fetcher when fully expired (past 2× TTL)', async () => {
      cacheSet('/v1/tasks', { v: 1 });
      vi.advanceTimersByTime(31_000);
 
      const fetcher = vi.fn().mockResolvedValue({ v: 99 });
      const result = await withSWR('/v1/tasks', fetcher);
 
      expect(result).toEqual({ v: 99 });
      expect(fetcher).toHaveBeenCalledTimes(1);
    });
  });
}