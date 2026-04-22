/** @type {import('next').NextConfig} */
const BACKEND = process.env.NEXT_PUBLIC_BACKEND_URL || "http://127.0.0.1:8080";

// In dev mode (`next dev`) keep output in .next so the HMR server doesn't
// try to read the static-export artifacts from ../web/ as its cache.
// In production (`next build`) we export to ../web/ for Rust actix-files.
const isDev = process.env.NODE_ENV === "development";

/** @type {import('next').NextConfig['headers']} */
const securityHeaders = [
  // Prevent clickjacking
  { key: "X-Frame-Options", value: "DENY" },
  // Prevent MIME-type sniffing
  { key: "X-Content-Type-Options", value: "nosniff" },
  // Force HTTPS in production (1 year, include subdomains)
  { key: "Strict-Transport-Security", value: "max-age=31536000; includeSubDomains" },
  // Referrer policy — don't leak path info to third parties
  { key: "Referrer-Policy", value: "strict-origin-when-cross-origin" },
  // Permissions policy — disable browser APIs not used by this app
  { key: "Permissions-Policy", value: "camera=(), microphone=(), geolocation=(), payment=()" },
  // Content Security Policy
  // 'unsafe-inline' is needed by Next.js for style tags; 'unsafe-eval' for dev HMR only.
  {
    key: "Content-Security-Policy",
    value: [
      "default-src 'self'",
      // Scripts: self + inline (Next.js requires this for hydration chunks)
      "script-src 'self' 'unsafe-inline'" + (isDev ? " 'unsafe-eval'" : ""),
      // Styles: self + inline (Tailwind CSS injects inline styles)
      "style-src 'self' 'unsafe-inline'",
      // Images: self + data URIs (for SVG icons)
      "img-src 'self' data: blob:",
      // Fonts served from same origin
      "font-src 'self'",
      // WebSocket connections to backend
      "connect-src 'self' " + BACKEND.replace("http:", "ws:").replace("https:", "wss:") + " " + BACKEND,
      // No plugins, no object embeds
      "object-src 'none'",
      // Base URI locked to self (prevents base-tag injection)
      "base-uri 'self'",
      // Forms only submit to same origin
      "form-action 'self'",
      // Block framing from other origins
      "frame-ancestors 'none'",
    ].join("; "),
  },
];

const nextConfig = {
  // ── Static export → outputs to ../web/ for Rust actix-files to serve ──
  output: isDev ? undefined : "export",
  distDir: isDev ? ".next" : "../web",
  trailingSlash: true,
  images: { unoptimized: true },

  // ── Dev-only rewrites (ignored in static export build) ─────────────────
  // These proxy API calls in `next dev` so the browser hits :3001
  // and Next.js forwards to the Rust backend on :8080.
  // In production the static files are served by Rust directly (same origin),
  // so fetch('/v1/...') resolves to the Rust server with no CORS.
  async rewrites() {
    if (!isDev) return [];
    return [
      { source: "/v1/:path*",    destination: `${BACKEND}/v1/:path*` },
      { source: "/v2/:path*",    destination: `${BACKEND}/v2/:path*` },
      { source: "/health/:path*",destination: `${BACKEND}/health/:path*` },
      { source: "/metrics",      destination: `${BACKEND}/metrics` },
      { source: "/ws",           destination: `${BACKEND}/ws` },
    ];
  },

  // ── Security headers (applied in dev and prod, skipped in static export) ──
  // Note: headers() is only active in Next.js server mode (next dev / next start).
  // For the static-export production build served by Rust, these headers are
  // injected by the Rust actix-web layer instead (see server.rs middleware).
  async headers() {
    return [
      {
        source: "/(.*)",
        headers: securityHeaders,
      },
    ];
  },

  experimental: {
    proxyTimeout: 120_000,
  },
};

export default nextConfig;
