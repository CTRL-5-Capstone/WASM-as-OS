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
