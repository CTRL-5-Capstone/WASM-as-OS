"use client";

import { useState } from "react";
import dynamic from "next/dynamic";
import Link from "next/link";
import {
  Terminal as TerminalIcon, Maximize2, Minimize2,
  Wifi, WifiOff, ArrowLeft, HelpCircle,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";

// XTerminal uses browser-only APIs — dynamic import with ssr:false
const XTerminal = dynamic(() => import("@/components/XTerminal"), {
  ssr: false,
  loading: () => (
    <div className="flex h-full items-center justify-center bg-[#0f172a]">
      <div className="text-center text-slate-400">
        <TerminalIcon size={32} className="mx-auto mb-3 opacity-40 animate-pulse" />
        <p className="text-sm">Loading terminal…</p>
      </div>
    </div>
  ),
});

export default function TerminalPage() {
  const [compact, setCompact] = useState(false);

  return (
    <div className={cn(
      "flex flex-col bg-background",
      compact ? "h-screen" : "min-h-screen"
    )}>
      {/* ── Toolbar ── */}
      <div className="flex items-center gap-3 border-b border-border bg-card px-4 py-2.5 shrink-0">
        <Link href="/tasks">
          <Button variant="ghost" size="icon" className="h-7 w-7 text-muted-foreground">
            <ArrowLeft size={14} />
          </Button>
        </Link>

        <div className="flex items-center gap-2">
          <div className="flex h-6 w-6 items-center justify-center rounded-md bg-primary/15">
            <TerminalIcon size={13} className="text-primary" />
          </div>
          <span className="text-sm font-semibold">WasmOS Terminal</span>
          <Badge variant="secondary" className="text-[10px] h-4 px-1.5">v4.0</Badge>
        </div>

        {/* Connection indicator */}
        <div className="flex items-center gap-1.5 ml-2">
          <span className="inline-block h-1.5 w-1.5 rounded-full bg-green-400 animate-pulse" />
          <span className="text-[11px] text-muted-foreground">WebSocket</span>
        </div>

        <div className="ml-auto flex items-center gap-1.5">
          {/* Quick help */}
          <Button
            variant="ghost"
            size="sm"
            className="h-7 text-xs text-muted-foreground gap-1"
            onClick={() => {
              // Programmatically type 'help' — XTerminal listens on its own
              // We can't easily communicate, but this is visible to user
            }}
            title="Type 'help' in the terminal for commands"
          >
            <HelpCircle size={13} />
            <span className="hidden sm:inline">Commands: help</span>
          </Button>

          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7 text-muted-foreground"
            onClick={() => setCompact(!compact)}
            title={compact ? "Restore" : "Full screen"}
          >
            {compact ? <Minimize2 size={13} /> : <Maximize2 size={13} />}
          </Button>
        </div>
      </div>

      {/* ── Command reference strip ── */}
      <div className="flex items-center gap-3 border-b border-border/50 bg-card/50 px-4 py-1.5 overflow-x-auto shrink-0">
        <span className="text-[10px] text-muted-foreground uppercase tracking-wider shrink-0 font-semibold">Quick:</span>
        {[
          { cmd: "ls",               desc: "list modules"    },
          { cmd: "wasm-run <id>",    desc: "execute module"  },
          { cmd: "security <id>",    desc: "scan module"     },
          { cmd: "stats",            desc: "system stats"    },
          { cmd: "snapshot <id>",    desc: "save snapshot"   },
          { cmd: "audit",            desc: "audit log"       },
          { cmd: "health",           desc: "backend health"  },
          { cmd: "kill <id>",        desc: "stop module"     },
        ].map(({ cmd, desc }) => (
          <div key={cmd} className="flex items-center gap-1 shrink-0">
            <code className="text-[10px] font-mono text-primary/80 bg-primary/8 rounded px-1.5 py-0.5">{cmd}</code>
            <span className="text-[10px] text-muted-foreground/60">{desc}</span>
          </div>
        ))}
      </div>

      {/* ── Terminal area ── */}
      <div className="flex-1 min-h-0 overflow-hidden">
        <XTerminal className="h-full w-full" />
      </div>
    </div>
  );
}
