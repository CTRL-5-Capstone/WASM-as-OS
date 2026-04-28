/**
 * Auth utilities — admin_key → JWT token flow.
 * Token is stored in localStorage under "wasmos_token".
 * All api.ts requests read it via getToken().
 */

const TOKEN_KEY = "wasmos_token";
const EXPIRY_KEY = "wasmos_token_exp";

export function getToken(): string | null {
  if (typeof window === "undefined") return null;
  const exp = localStorage.getItem(EXPIRY_KEY);
  if (exp && Date.now() > Number(exp)) {
    clearToken();
    return null;
  }
  return localStorage.getItem(TOKEN_KEY);
}

export function setToken(token: string, expiresInSeconds = 3600) {
  localStorage.setItem(TOKEN_KEY, token);
  localStorage.setItem(EXPIRY_KEY, String(Date.now() + expiresInSeconds * 1000));
}

export function clearToken() {
  localStorage.removeItem(TOKEN_KEY);
  localStorage.removeItem(EXPIRY_KEY);
}

export function isAuthenticated(): boolean {
  return getToken() !== null;
}

export interface LoginResponse {
  token: string;
  expires_in: number;
  token_type: string;
  role: string;
  user_id: string;
}

/**
 * Exchange admin_key for a signed JWT.
 * POST /v1/auth/token — this endpoint is exempt from JWT middleware (bootstrap).
 * Optional userId defaults to "admin" on the backend if omitted.
 */
export async function login(adminKey: string, userId?: string): Promise<string> {
  const res = await fetch("/v1/auth/token", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      admin_key: adminKey,
      ...(userId ? { user_id: userId } : {}),
    }),
  });
  if (!res.ok) {
    const body = await res.text().catch(() => "");
    // Parse structured error from backend if available
    try {
      const parsed = JSON.parse(body);
      throw new Error(parsed.error ?? `Auth failed: ${res.status}`);
    } catch {
      throw new Error(body || `Auth failed: ${res.status}`);
    }
  }
  const data: LoginResponse = await res.json();
  const token: string = data.token ?? "";
  if (!token) throw new Error("No token in auth response");
  setToken(token, data.expires_in ?? 3600);
  return token;
}

export async function logout() {
  clearToken();
}

// --- In-source tests (vitest) --------------------------------------------------
// Stripped from production via `define: { 'import.meta.vitest': 'undefined' }`.
// Run with: npm test


if (import.meta.vitest) {
  const { describe, it, expect, beforeEach, afterEach, vi } = import.meta.vitest;
 
  beforeEach(() => {
    localStorage.clear();
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-04-27T12:00:00Z'));
  });
  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });
 
  describe('setToken() / getToken()', () => {
    it('stores and retrieves the token', () => {
      setToken('jwt-abc');
      expect(getToken()).toBe('jwt-abc');
    });
 
    it('persists the token under the documented key', () => {
      setToken('jwt-abc');
      expect(localStorage.getItem('wasmos_token')).toBe('jwt-abc');
    });
 
    it('records the expiry timestamp using the supplied seconds', () => {
      setToken('jwt-abc', 1800);
      const exp = Number(localStorage.getItem('wasmos_token_exp'));
      expect(exp).toBe(Date.parse('2026-04-27T12:00:00Z') + 1800 * 1000);
    });
 
    it('defaults to a 1-hour expiry when no seconds are passed', () => {
      setToken('jwt-abc');
      const exp = Number(localStorage.getItem('wasmos_token_exp'));
      expect(exp - Date.now()).toBe(3600 * 1000);
    });
  });
 
  describe('getToken() — expiry behaviour', () => {
    it('returns the token when not yet expired', () => {
      setToken('jwt-abc', 60);
      vi.advanceTimersByTime(30 * 1000);
      expect(getToken()).toBe('jwt-abc');
    });
 
    it('clears and returns null once the expiry passes', () => {
      setToken('jwt-abc', 60);
      vi.advanceTimersByTime(61 * 1000);
      expect(getToken()).toBeNull();
      expect(localStorage.getItem('wasmos_token')).toBeNull();
      expect(localStorage.getItem('wasmos_token_exp')).toBeNull();
    });
 
    it('returns null when nothing has been stored', () => {
      expect(getToken()).toBeNull();
    });
  });
 
  describe('clearToken()', () => {
    it('removes both the token and its expiry record', () => {
      setToken('jwt-abc');
      clearToken();
      expect(localStorage.getItem('wasmos_token')).toBeNull();
      expect(localStorage.getItem('wasmos_token_exp')).toBeNull();
    });
 
    it('is a no-op when nothing is stored', () => {
      expect(() => clearToken()).not.toThrow();
    });
  });
 
  describe('isAuthenticated()', () => {
    it('returns true when a valid token is present', () => {
      setToken('jwt-abc');
      expect(isAuthenticated()).toBe(true);
    });
 
    it('returns false when expired (getToken side-effect clears it)', () => {
      setToken('jwt-abc', 1);
      vi.advanceTimersByTime(2000);
      expect(isAuthenticated()).toBe(false);
    });
 
    it('returns false on a fresh storage', () => {
      expect(isAuthenticated()).toBe(false);
    });
  });
 
  describe('login()', () => {
    it('POSTs to /v1/auth/token with admin_key in the body', async () => {
      const fetchSpy = vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response(
        JSON.stringify({
          token: 'jwt-zzz', expires_in: 3600, token_type: 'Bearer',
          role: 'admin', user_id: 'admin',
        }),
        { status: 200, headers: { 'Content-Type': 'application/json' } },
      ));
 
      await login('s3cret');
 
      expect(fetchSpy).toHaveBeenCalledTimes(1);
      const [url, init] = fetchSpy.mock.calls[0];
      expect(url).toBe('/v1/auth/token');
      expect(init?.method).toBe('POST');
      expect(JSON.parse(init?.body as string)).toEqual({ admin_key: 's3cret' });
    });
 
    it('includes user_id in the body when provided', async () => {
      const fetchSpy = vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response(
        JSON.stringify({ token: 't', expires_in: 60, token_type: 'Bearer', role: 'admin', user_id: 'biraj' }),
        { status: 200 },
      ));
 
      await login('s3cret', 'biraj');
 
      const [, init] = fetchSpy.mock.calls[0];
      expect(JSON.parse(init?.body as string)).toEqual({
        admin_key: 's3cret', user_id: 'biraj',
      });
    });
 
    it('persists the returned token via setToken()', async () => {
      vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response(
        JSON.stringify({ token: 'jwt-zzz', expires_in: 1800, token_type: 'Bearer', role: 'admin', user_id: 'admin' }),
        { status: 200 },
      ));
 
      const tok = await login('s3cret');
 
      expect(tok).toBe('jwt-zzz');
      expect(getToken()).toBe('jwt-zzz');
      const exp = Number(localStorage.getItem('wasmos_token_exp'));
      expect(exp - Date.now()).toBe(1800 * 1000);
    });
 
    it('falls back to a 1-hour expiry when expires_in is missing', async () => {
      vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response(
        JSON.stringify({ token: 'jwt-zzz', token_type: 'Bearer', role: 'admin', user_id: 'admin' }),
        { status: 200 },
      ));
 
      await login('s3cret');
      const exp = Number(localStorage.getItem('wasmos_token_exp'));
      expect(exp - Date.now()).toBe(3600 * 1000);
    });
 
    it('rejects when the backend returns a structured JSON error', async () => {
      vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response(
        JSON.stringify({ error: 'Invalid admin_key', status: 401 }),
        { status: 401 },
      ));
      await expect(login('wrong')).rejects.toThrow('Invalid admin_key');
    });
 
    it('rejects with the raw body when the error is not JSON', async () => {
      vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response(
        'rate limited', { status: 429 },
      ));
      await expect(login('x')).rejects.toThrow('rate limited');
    });
 
    it('rejects when the response is OK but contains no token', async () => {
      vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response(
        JSON.stringify({ expires_in: 60 }), { status: 200 },
      ));
      await expect(login('x')).rejects.toThrow(/no token/i);
    });
  });
 
  describe('logout()', () => {
    it('clears any stored token', async () => {
      setToken('jwt-abc');
      await logout();
      expect(getToken()).toBeNull();
    });
  });
}