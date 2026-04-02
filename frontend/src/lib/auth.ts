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
