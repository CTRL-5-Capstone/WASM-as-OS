/** @type {import('next').NextConfig} */
const BACKEND = process.env.NEXT_PUBLIC_BACKEND_URL || "http://127.0.0.1:8080";

// In dev mode (`next dev`) keep output in .next so the HMR server doesn't
// try to read the static-export artifacts from ../web/ as its cache.
// In production (`next build`) we export to ../web/ for Rust actix-files.
const isDev = process.env.NODE_ENV === "development";

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

  experimental: {
    proxyTimeout: 120_000,
  },
};

export default nextConfig;
