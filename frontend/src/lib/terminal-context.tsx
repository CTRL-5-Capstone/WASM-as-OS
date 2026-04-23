"use client";

import { createContext, useContext, useState, useCallback, useRef, type ReactNode } from "react";

// ─── Types ──────────────────────────────────────────────────────────

export interface TermLine {
  id: number;
  type: "input" | "output" | "error" | "system" | "table" | "progress";
  text: string;
  ts: number;
}

export interface TerminalApi {
  lines: TermLine[];
  busy: boolean;
  push: (type: TermLine["type"], text: string) => void;
  clear: () => void;
  setBusy: (b: boolean) => void;
  subscribe: (cb: TerminalSubscriber) => () => void;
}

export type TerminalSubscriber = (line: TermLine) => void;

// ─── Context ────────────────────────────────────────────────────────

const TerminalContext = createContext<TerminalApi | null>(null);

export function useTerminal() {
  const ctx = useContext(TerminalContext);
  if (!ctx) throw new Error("useTerminal must be inside TerminalProvider");
  return ctx;
}

// ─── Provider ───────────────────────────────────────────────────────

export function TerminalProvider({ children }: { children: ReactNode }) {
  const [lines, setLines] = useState<TermLine[]>([
    { id: 0, type: "system", text: "WASM-OS Terminal v3.0  —  type 'help' for commands", ts: Date.now() },
  ]);
  const [busy, setBusy] = useState(false);
  const nextId = useRef(1);
  const subscribers = useRef<Set<TerminalSubscriber>>(new Set());

  const push = useCallback((type: TermLine["type"], text: string) => {
    const line: TermLine = { id: nextId.current++, type, text, ts: Date.now() };
    setLines((prev) => [...prev, line]);
    // broadcast to all subscribers (Monitor page, Dashboard, etc.)
    subscribers.current.forEach((cb) => {
      try { cb(line); } catch { /* ignore */ }
    });
  }, []);

  const clear = useCallback(() => setLines([]), []);

  const subscribe = useCallback((cb: TerminalSubscriber) => {
    subscribers.current.add(cb);
    return () => { subscribers.current.delete(cb); };
  }, []);

  return (
    <TerminalContext.Provider value={{ lines, busy, push, clear, setBusy, subscribe }}>
      {children}
    </TerminalContext.Provider>
  );
}
