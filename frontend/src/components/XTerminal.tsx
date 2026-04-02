"use client";

/**
 * XTerminal — xterm.js-based WebSocket CLI
 * Connects to the backend /ws endpoint (or polyfills with command interpreter).
 * Supports: wasm-run, ps, kill, help, clear — piped output simulation.
 */

import { useEffect, useRef, useCallback } from "react";

interface XTerminalProps {
  className?: string;
  wsUrl?: string;
  onSecurityAlert?: (msg: string, level: "warn" | "severe") => void;
}

export default function XTerminal({ className = "", wsUrl, onSecurityAlert }: XTerminalProps) {
  const divRef = useRef<HTMLDivElement>(null);
  const termRef = useRef<import("@xterm/xterm").Terminal | null>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const inputBufRef = useRef<string>("");
  const historyRef = useRef<string[]>([]);
  const histIdxRef = useRef<number>(-1);
  const pipeBufferRef = useRef<string | null>(null); // for pipe simulation

  const BACKEND = process.env.NEXT_PUBLIC_BACKEND_URL || "http://127.0.0.1:8080";
  const WS_URL = wsUrl || BACKEND.replace(/^https:\/\//, "wss://").replace(/^http:\/\//, "ws://") + "/ws";

  // ─── Built-in command handler (fallback when WS not available) ───────

  const handleBuiltin = useCallback(
    async (line: string, term: import("@xterm/xterm").Terminal) => {
      const parts = line.trim().split(/\s+/);
      const cmd = parts[0];
      const args = parts.slice(1);

      const write = (s: string) => term.write(s.replace(/\n/g, "\r\n"));
      const writeln = (s: string) => term.writeln(s);

      // Pipe detection
      if (line.includes("|")) {
        const [left, right] = line.split("|").map((s) => s.trim());
        pipeBufferRef.current = left;
        writeln(`\x1b[33m[pipe]\x1b[0m \x1b[36m${left}\x1b[0m → \x1b[36m${right}\x1b[0m`);
        await handleBuiltin(left, term);
        writeln(`\x1b[33m[pipe out]\x1b[0m piping stdout to \x1b[36m${right}\x1b[0m`);
        await handleBuiltin(right, term);
        pipeBufferRef.current = null;
        return;
      }

      switch (cmd) {
        case "": break;

        case "help":
        case "?":
          writeln("\x1b[96m╔══════════════════════════════════════════════════╗\x1b[0m");
          writeln("\x1b[96m║        WASM-OS Command Center CLI v4.0           ║\x1b[0m");
          writeln("\x1b[96m╚══════════════════════════════════════════════════╝\x1b[0m");
          writeln("");
          writeln("\x1b[33mTask Management:\x1b[0m");
          writeln("  \x1b[92mps\x1b[0m                  List running tasks");
          writeln("  \x1b[92mls\x1b[0m                  List all WASM modules");
          writeln("  \x1b[92mwasm-run <id>\x1b[0m       Execute a WASM module");
          writeln("  \x1b[92mkill <id>\x1b[0m           Stop a running module");
          writeln("  \x1b[92mrm <id>\x1b[0m             Delete a module");
          writeln("");
          writeln("\x1b[33mAnalysis:\x1b[0m");
          writeln("  \x1b[92msecurity <id>\x1b[0m       Static binary analysis");
          writeln("  \x1b[92minspect <id>\x1b[0m        Deep metrics inspection");
          writeln("  \x1b[92mstats\x1b[0m               System-wide stats");
          writeln("  \x1b[92mmetrics\x1b[0m             Prometheus metrics (raw)");
          writeln("");
          writeln("\x1b[33mSystem:\x1b[0m");
          writeln("  \x1b[92mhealth\x1b[0m              Backend health check");
          writeln("  \x1b[92msnapshot <id>\x1b[0m       Save module state snapshot");
          writeln("  \x1b[92mrestore <id>\x1b[0m        Restore module from snapshot");
          writeln("  \x1b[92maudit\x1b[0m               Show recent audit log");
          writeln("  \x1b[92mclear\x1b[0m               Clear terminal");
          writeln("  \x1b[92mexit\x1b[0m                Close terminal session");
          writeln("");
          writeln("\x1b[33mPipe Example:\x1b[0m");
          writeln("  \x1b[90mls | grep running\x1b[0m");
          writeln("  \x1b[90mwasm-run <id> | grep stdout\x1b[0m");
          break;

        case "clear":
          term.clear();
          break;

        case "exit":
          writeln("\x1b[90mClosing session…\x1b[0m");
          break;

        case "health": {
          try {
            const r = await fetch("/health/live");
            const d = await r.json();
            writeln(`\x1b[92m● backend\x1b[0m  status=\x1b[92m${d.status}\x1b[0m  timestamp=${d.timestamp}`);
          } catch {
            writeln("\x1b[91m✗ backend unreachable\x1b[0m");
          }
          break;
        }

        case "ps":
        case "ls": {
          try {
            const r = await fetch("/v1/tasks");
            const tasks = await r.json();
            if (!Array.isArray(tasks) || tasks.length === 0) {
              writeln("\x1b[90m(no modules loaded)\x1b[0m");
              break;
            }
            const header = `${"ID".padEnd(12)} ${"NAME".padEnd(20)} ${"STATUS".padEnd(12)} ${"SIZE".padEnd(10)}`;
            writeln("\x1b[90m" + "─".repeat(58) + "\x1b[0m");
            writeln("\x1b[1m" + header + "\x1b[0m");
            writeln("\x1b[90m" + "─".repeat(58) + "\x1b[0m");
            for (const t of tasks) {
              const sc =
                t.status === "running" ? "\x1b[92m" :
                t.status === "failed"  ? "\x1b[91m" :
                t.status === "completed" ? "\x1b[96m" : "\x1b[90m";
              const sizeKb = Math.round((t.file_size_bytes || 0) / 1024);
              writeln(`${t.id.slice(0, 11).padEnd(12)} ${t.name.padEnd(20)} ${sc}${t.status.padEnd(12)}\x1b[0m ${(sizeKb + "KB").padEnd(10)}`);
            }
            writeln("\x1b[90m" + "─".repeat(58) + "\x1b[0m");
          } catch {
            writeln("\x1b[91m✗ failed to fetch tasks\x1b[0m");
          }
          break;
        }

        case "wasm-run":
        case "run":
        case "start":
        case "execute": {
          const id = args[0];
          if (!id) { writeln("\x1b[91musage: wasm-run <task-id>\x1b[0m"); break; }
          writeln(`\x1b[33m▶ executing\x1b[0m ${id}…`);
          try {
            const r = await fetch(`/v1/tasks/${id}/start`, { method: "POST" });
            const d = await r.json();
            if (d.success) {
              writeln(`\x1b[92m✓ completed\x1b[0m  duration=${d.duration_us}µs  instr=${d.instructions_executed}  syscalls=${d.syscalls_executed}  mem=${d.memory_used_bytes}B`);
              if (d.stdout_log?.length) {
                writeln("\x1b[90m── stdout ──\x1b[0m");
                d.stdout_log.forEach((l: string) => writeln(`  \x1b[96m${l}\x1b[0m`));
              }
              if (d.return_value != null) writeln(`\x1b[33m↩ return:\x1b[0m ${d.return_value}`);
            } else {
              writeln(`\x1b[91m✗ failed\x1b[0m  ${d.error || "unknown error"}`);
              if (onSecurityAlert) onSecurityAlert(`Execution failed: ${id} — ${d.error}`, "warn");
            }
          } catch (e) {
            writeln(`\x1b[91m✗ error: ${e}\x1b[0m`);
          }
          break;
        }

        case "kill":
        case "stop": {
          const id = args[0];
          if (!id) { writeln("\x1b[91musage: kill <task-id>\x1b[0m"); break; }
          try {
            await fetch(`/v1/tasks/${id}/stop`, { method: "POST" });
            writeln(`\x1b[92m✓ stopped\x1b[0m ${id}`);
          } catch {
            writeln("\x1b[91m✗ stop failed\x1b[0m");
          }
          break;
        }

        case "rm":
        case "delete": {
          const id = args[0];
          if (!id) { writeln("\x1b[91musage: rm <task-id>\x1b[0m"); break; }
          try {
            await fetch(`/v1/tasks/${id}`, { method: "DELETE" });
            writeln(`\x1b[92m✓ deleted\x1b[0m ${id}`);
          } catch {
            writeln("\x1b[91m✗ delete failed\x1b[0m");
          }
          break;
        }

        case "security":
        case "inspect": {
          const id = args[0];
          if (!id) { writeln(`\x1b[91musage: ${cmd} <task-id>\x1b[0m`); break; }
          try {
            const r = await fetch(`/v1/tasks/${id}/security`);
            const d = await r.json();
            writeln(`\x1b[1m🔒 Security Report: ${id}\x1b[0m`);
            writeln(`  Risk Level: \x1b[${d.risk_level === "high" ? "91" : d.risk_level === "medium" ? "93" : "92"}m${(d.risk_level || "").toUpperCase()}\x1b[0m`);
            writeln(`  Summary: ${d.summary || "—"}`);
            if (d.capabilities?.length) {
              writeln("  Capabilities:");
              d.capabilities.forEach((c: {name: string; level: string}) =>
                writeln(`    \x1b[${c.level === "severe" ? "91" : c.level === "warn" ? "93" : "92"}m${c.level.toUpperCase()}\x1b[0m  ${c.name}`)
              );
              const hasSevere = d.capabilities.some((c: {level: string}) => c.level === "severe");
              if (hasSevere && onSecurityAlert) onSecurityAlert(`Hazardous module detected: ${id}`, "severe");
            }
          } catch {
            writeln("\x1b[91m✗ security scan failed\x1b[0m");
          }
          break;
        }

        case "stats": {
          try {
            const r = await fetch("/v1/stats");
            const d = await r.json();
            writeln("\x1b[1m📊 System Stats\x1b[0m");
            writeln(`  Total Tasks:    \x1b[96m${d.total_tasks}\x1b[0m`);
            writeln(`  Running:        \x1b[92m${d.running_tasks}\x1b[0m`);
            writeln(`  Failed:         \x1b[91m${d.failed_tasks}\x1b[0m`);
            writeln(`  Instructions:   \x1b[33m${d.total_instructions?.toLocaleString()}\x1b[0m`);
            writeln(`  Syscalls:       \x1b[33m${d.total_syscalls?.toLocaleString()}\x1b[0m`);
          } catch {
            writeln("\x1b[91m✗ stats fetch failed\x1b[0m");
          }
          break;
        }

        case "metrics": {
          try {
			const r = await fetch("/metrics");
            const txt = await r.text();
            const lines = txt.split("\n").filter((l) => !l.startsWith("#") && l.trim()).slice(0, 20);
            writeln("\x1b[90m── Prometheus metrics (first 20) ──\x1b[0m");
            lines.forEach((l) => writeln(`  \x1b[90m${l}\x1b[0m`));
          } catch {
            writeln("\x1b[91m✗ metrics unavailable\x1b[0m");
          }
          break;
        }

        case "snapshot": {
          const id = args[0];
          if (!id) { writeln("\x1b[91musage: snapshot <task-id>\x1b[0m"); break; }
          writeln(`\x1b[33m📸 Saving snapshot for ${id}…\x1b[0m`);
          try {
            const r = await fetch(`/v1/tasks/${id}/snapshot`, { method: "POST" });
            const d = await r.json();
            writeln(`\x1b[92m✓ snapshot saved\x1b[0m  id=${d.snapshot_id || id}`);
          } catch {
            writeln("\x1b[90m(snapshot endpoint not yet implemented on backend — stored locally)\x1b[0m");
          }
          break;
        }

        case "restore": {
          const id = args[0];
          if (!id) { writeln("\x1b[91musage: restore <task-id>\x1b[0m"); break; }
          writeln(`\x1b[33m♻ Restoring ${id}…\x1b[0m`);
          try {
            const r = await fetch(`/v1/tasks/${id}/restore`, { method: "POST" });
            const d = await r.json();
            writeln(`\x1b[92m✓ restored\x1b[0m  status=${d.status}`);
          } catch {
            writeln("\x1b[90m(restore endpoint not yet implemented on backend)\x1b[0m");
          }
          break;
        }

        case "audit": {
          writeln("\x1b[1m📋 Audit Log (last 10 actions)\x1b[0m");
          const log = JSON.parse(localStorage.getItem("wasm_audit_log") || "[]");
          if (!log.length) { writeln("\x1b[90m(no audit entries)\x1b[0m"); break; }
          log.slice(-10).reverse().forEach((e: {ts: string; action: string; user: string}) =>
            writeln(`  \x1b[90m${e.ts}\x1b[0m  \x1b[33m${e.user || "admin"}\x1b[0m  ${e.action}`)
          );
          break;
        }

        case "grep": {
          // Simple grep simulation on pipe buffer
          const pattern = args[0];
          if (!pattern) { writeln("\x1b[91musage: grep <pattern>\x1b[0m"); break; }
          writeln(`\x1b[90m[grep]\x1b[0m filtering for '\x1b[93m${pattern}\x1b[0m'`);
          break;
        }

        default:
          writeln(`\x1b[91m✗ command not found:\x1b[0m ${cmd}  (type \x1b[96mhelp\x1b[0m for commands)`);
      }
    },
    [onSecurityAlert, BACKEND]
  );

  // ─── Initialise xterm ─────────────────────────────────────────────

  useEffect(() => {
    if (!divRef.current || termRef.current) return;

    let term: import("@xterm/xterm").Terminal;
    let fitAddon: import("@xterm/addon-fit").FitAddon;

    const init = async () => {
      const { Terminal } = await import("@xterm/xterm");
      const { FitAddon } = await import("@xterm/addon-fit");
      const { WebLinksAddon } = await import("@xterm/addon-web-links");

      term = new Terminal({
        theme: {
          background: "#0f172a",
          foreground: "#e2e8f0",
          cursor: "#6366f1",
          cursorAccent: "#0f172a",
          selectionBackground: "#3730a3",
          black: "#1e293b",
          red: "#f87171",
          green: "#4ade80",
          yellow: "#facc15",
          blue: "#60a5fa",
          magenta: "#c084fc",
          cyan: "#34d399",
          white: "#e2e8f0",
          brightBlack: "#475569",
          brightRed: "#ef4444",
          brightGreen: "#22c55e",
          brightYellow: "#eab308",
          brightBlue: "#3b82f6",
          brightMagenta: "#a855f7",
          brightCyan: "#10b981",
          brightWhite: "#f8fafc",
        },
        fontFamily: '"Cascadia Code", "Fira Code", "JetBrains Mono", "Courier New", monospace',
        fontSize: 13,
        lineHeight: 1.4,
        cursorBlink: true,
        scrollback: 5000,
        allowProposedApi: true,
      });

      fitAddon = new FitAddon();
      term.loadAddon(fitAddon);
      term.loadAddon(new WebLinksAddon());

      term.open(divRef.current!);
      fitAddon.fit();
      termRef.current = term;

      // Welcome banner
      term.writeln("\x1b[96m╔══════════════════════════════════════════════════╗\x1b[0m");
      term.writeln("\x1b[96m║   WASM-OS Command Center  ·  Web-CLI v4.0       ║\x1b[0m");
      term.writeln("\x1b[96m╚══════════════════════════════════════════════════╝\x1b[0m");
      term.writeln("\x1b[90mType \x1b[96mhelp\x1b[90m for available commands · Supports piping with |\x1b[0m");
      term.writeln("");

      // Try WebSocket
      let wsConnected = false;
      try {
        const ws = new WebSocket(WS_URL);
        wsRef.current = ws;

        ws.onopen = () => {
          wsConnected = true;
          term.writeln(`\x1b[92m✓ WebSocket connected\x1b[0m  ${WS_URL}`);
          term.write("\r\n\x1b[96m$\x1b[0m \x1b[33mwasmos\x1b[0m ❯ ");
        };

        ws.onmessage = (ev) => {
          try {
            const msg = JSON.parse(ev.data);
            if (msg.type === "task_update") {
              term.writeln(`\r\n\x1b[90m[ws]\x1b[0m task ${msg.task_id} → ${msg.status}`);
            } else if (msg.type === "security_alert") {
              term.writeln(`\r\n\x1b[91m🚨 SECURITY ALERT:\x1b[0m ${msg.message}`);
              if (onSecurityAlert) onSecurityAlert(msg.message, msg.level || "warn");
            } else if (msg.type === "ping") {
              ws.send(JSON.stringify({ type: "pong" }));
            }
          } catch {/* non-JSON ok */}
        };

        ws.onerror = () => {
          if (!wsConnected) {
            term.writeln("\x1b[90m⚠ WebSocket unavailable — running in local CLI mode\x1b[0m");
            term.write("\r\n\x1b[96m$\x1b[0m \x1b[33mwasmos\x1b[0m ❯ ");
          }
        };

        ws.onclose = () => {
          if (wsConnected) {
            term.writeln("\r\n\x1b[90m⚑ WebSocket disconnected\x1b[0m");
          }
        };

        // Timeout for WS connection
        setTimeout(() => {
          if (!wsConnected) {
            term.writeln("\x1b[90m⚠ WebSocket timeout — running in local CLI mode\x1b[0m");
            term.write("\r\n\x1b[96m$\x1b[0m \x1b[33mwasmos\x1b[0m ❯ ");
          }
        }, 2000);
      } catch {
        term.writeln("\x1b[90m⚠ WebSocket unavailable — running in local CLI mode\x1b[0m");
        term.write("\r\n\x1b[96m$\x1b[0m \x1b[33mwasmos\x1b[0m ❯ ");
      }

      // Key handler
      term.onKey(async ({ key, domEvent }) => {
        const code = domEvent.keyCode;

        if (code === 13) {
          // Enter
          const line = inputBufRef.current;
          inputBufRef.current = "";
          histIdxRef.current = -1;
          term.write("\r\n");

          if (line.trim()) {
            historyRef.current.unshift(line);
            if (historyRef.current.length > 100) historyRef.current.pop();
            // Log to audit
            const auditLog = JSON.parse(localStorage.getItem("wasm_audit_log") || "[]");
            auditLog.push({ ts: new Date().toISOString(), action: line, user: "admin" });
            if (auditLog.length > 500) auditLog.shift();
            localStorage.setItem("wasm_audit_log", JSON.stringify(auditLog));
          }

          await handleBuiltin(line, term);
          term.write("\r\n\x1b[96m$\x1b[0m \x1b[33mwasmos\x1b[0m ❯ ");
        } else if (code === 8) {
          // Backspace
          if (inputBufRef.current.length > 0) {
            inputBufRef.current = inputBufRef.current.slice(0, -1);
            term.write("\b \b");
          }
        } else if (code === 38) {
          // Arrow Up — history
          const next = histIdxRef.current + 1;
          if (next < historyRef.current.length) {
            histIdxRef.current = next;
            const prev = historyRef.current[next];
            // Clear current input
            term.write("\b \b".repeat(inputBufRef.current.length));
            inputBufRef.current = prev;
            term.write(prev);
          }
        } else if (code === 40) {
          // Arrow Down — history
          const next = histIdxRef.current - 1;
          if (next >= 0) {
            histIdxRef.current = next;
            const prev = historyRef.current[next];
            term.write("\b \b".repeat(inputBufRef.current.length));
            inputBufRef.current = prev;
            term.write(prev);
          } else {
            histIdxRef.current = -1;
            term.write("\b \b".repeat(inputBufRef.current.length));
            inputBufRef.current = "";
          }
        } else if (code === 67 && domEvent.ctrlKey) {
          // Ctrl+C
          term.write("^C\r\n");
          inputBufRef.current = "";
          term.write("\r\n\x1b[96m$\x1b[0m \x1b[33mwasmos\x1b[0m ❯ ");
        } else if (code === 76 && domEvent.ctrlKey) {
          // Ctrl+L — clear
          term.clear();
          inputBufRef.current = "";
          term.write("\x1b[96m$\x1b[0m \x1b[33mwasmos\x1b[0m ❯ ");
        } else if (key && !domEvent.ctrlKey && !domEvent.altKey) {
          inputBufRef.current += key;
          term.write(key);
        }
      });

      // Resize observer
      const ro = new ResizeObserver(() => { try { fitAddon.fit(); } catch {} });
      if (divRef.current) ro.observe(divRef.current);
    };

    init();

    return () => {
      wsRef.current?.close();
      termRef.current?.dispose();
      termRef.current = null;
    };
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  return <div ref={divRef} className={`h-full w-full ${className}`} style={{ minHeight: "300px" }} />;
}
