/**
 * useWebSocket — live task events and analytics from WasmOS backend WebSocket (/ws).
 * Handles: task events, live metrics (P50/P95/P99), trace updates, ping/pong.
 * Reconnects automatically with exponential backoff.
 * Dispatches Sonner toasts on task completion/failure.
 */
"use client";

import { useEffect, useRef, useCallback, useState } from "react";
import { toast } from "sonner";
import type { LiveMetrics } from "./api";

// All type tags MUST match the backend's WsMessage #[serde(rename = "...")] values.
// The Rust enum uses explicit snake_case renames for every variant — PascalCase
// tags like "LiveMetrics" were silently dropped because JSON.parse returned an
// object with type === "live_metrics" which never matched the old constants.
export type WsEventType =
  | "task_started"
  | "task_completed"
  | "task_failed"
  | "task_stopped"
  | "task_event"
  | "task_update"
  | "ping"
  | "pong"
  | "connected"
  | "live_metrics"    // was "LiveMetrics"  — backend: #[serde(rename = "live_metrics")]
  | "trace_update"   // was "TraceUpdate"  — backend: #[serde(rename = "trace_update")]
  | "system_stats"   // was "SystemStats"  — backend: #[serde(rename = "system_stats")]
  | "output"         // was "Output"       — backend: #[serde(rename = "output")]
  | "terminal_ready" // was "TerminalReady"— backend: #[serde(rename = "terminal_ready")]
  | "resize"
  | "error";         // was "Error"        — backend: #[serde(rename = "error")]

export interface WsTaskEvent {
  type: WsEventType;
  task_id?: string;
  task_name?: string;
  status?: string;
  error?: string;
  timestamp?: string;
  data?: LiveMetrics | Record<string, unknown>;
  [key: string]: unknown;
}

export type WsStatus = "connecting" | "connected" | "disconnected" | "error";

interface UseWebSocketOptions {
  /** Called for every parsed message */
  onEvent?: (evt: WsTaskEvent) => void;
  /** Called specifically when LiveMetrics arrives */
  onLiveMetrics?: (metrics: LiveMetrics) => void;
  /** Disable toasts (default: toasts enabled) */
  silent?: boolean;
  /** Disable auto-reconnect */
  noReconnect?: boolean;
}

// Build the WebSocket URL from env var or derive from backend URL.
// Preserves the security protocol: http → ws, https → wss.
function getWsUrl(): string {
  if (typeof window === "undefined") return "";
  const envWs = process.env.NEXT_PUBLIC_WS_URL;
  if (envWs) return envWs;
  const backendUrl = process.env.NEXT_PUBLIC_BACKEND_URL || "http://127.0.0.1:8080";
  // Replace http:// → ws:// and https:// → wss:// to match the transport security
  return backendUrl.replace(/^https:\/\//, "wss://").replace(/^http:\/\//, "ws://") + "/ws";
}

export function useWebSocket(opts: UseWebSocketOptions = {}) {
  const [status, setStatus] = useState<WsStatus>("disconnected");
  const wsRef = useRef<WebSocket | null>(null);
  const retryRef = useRef(0);
  const unmountedRef = useRef(false);
  const pingIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const connect = useCallback(() => {
    if (unmountedRef.current) return;
    if (typeof window === "undefined") return;

    const wsUrl = getWsUrl();
    setStatus("connecting");

    try {
      const ws = new WebSocket(wsUrl);
      wsRef.current = ws;

      ws.onopen = () => {
        if (unmountedRef.current) { ws.close(); return; }
        setStatus("connected");
        retryRef.current = 0;
        // Send periodic pings to keep connection alive
        pingIntervalRef.current = setInterval(() => {
          if (ws.readyState === WebSocket.OPEN) {
            ws.send(JSON.stringify({ type: "ping" }));  // backend WsClientMessage: #[serde(rename = "ping")]
          }
        }, 20_000);
      };

      ws.onmessage = (e) => {
        if (unmountedRef.current) return;
        try {
          const evt = JSON.parse(e.data) as WsTaskEvent;

          // live_metrics arrives as a flat object (no nested .data field) —
          // the backend serialises the struct fields directly into the message.
          if (evt.type === "live_metrics") {
            opts.onLiveMetrics?.(evt as unknown as LiveMetrics);
          }

          // Route to caller
          opts.onEvent?.(evt);

          // Toast notifications
          if (!opts.silent) {
            // Handle both snake_case (backend task events) and PascalCase (new WS protocol)
            const evtType = evt.type?.toLowerCase().replace("task_", "");
            switch (evtType) {
              case "completed":
                toast.success(`✅ Task completed`, {
                  description: evt.task_name ?? evt.task_id ?? "",
                });
                break;
              case "failed":
                toast.error(`❌ Task failed`, {
                  description: `${evt.task_name ?? evt.task_id ?? ""}${evt.error ? ` — ${evt.error}` : ""}`,
                });
                break;
              case "started":
                toast.info(`▶ Task started`, {
                  description: evt.task_name ?? evt.task_id ?? "",
                  duration: 2500,
                });
                break;
              case "stopped":
                toast.warning(`⏹ Task stopped`, {
                  description: evt.task_name ?? evt.task_id ?? "",
                  duration: 2500,
                });
                break;
            }
          }
        } catch {
          // Ignore non-JSON pings
        }
      };

      ws.onerror = () => {
        if (unmountedRef.current) return;
        setStatus("error");
      };

      ws.onclose = () => {
        if (unmountedRef.current) return;
        setStatus("disconnected");
        wsRef.current = null;
        if (pingIntervalRef.current) {
          clearInterval(pingIntervalRef.current);
          pingIntervalRef.current = null;
        }

        if (!opts.noReconnect) {
          // Exponential backoff: 1s, 2s, 4s, 8s … capped at 30s
          const delay = Math.min(1000 * 2 ** retryRef.current, 30_000);
          retryRef.current++;
          setTimeout(connect, delay);
        }
      };
    } catch {
      setStatus("error");
    }
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    unmountedRef.current = false;
    connect();
    return () => {
      unmountedRef.current = true;
      if (pingIntervalRef.current) clearInterval(pingIntervalRef.current);
      wsRef.current?.close();
    };
  }, [connect]);

  const send = useCallback((data: unknown) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(typeof data === "string" ? data : JSON.stringify(data));
    }
  }, []);

  return { status, send };
}
