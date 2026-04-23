"use client";

import dynamic from "next/dynamic";
import Link from "next/link";
import {
  Terminal as TerminalIcon, ArrowLeft, Info,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";

// TerminalEmulator uses browser APIs — ssr:false prevents hydration errors
const TerminalEmulator = dynamic(() => import("@/components/TerminalEmulator"), {
  ssr: false,
  loading: () => (
    <div className="flex h-full items-center justify-center bg-background">
      <div className="text-center text-muted-foreground">
        <TerminalIcon size={32} className="mx-auto mb-3 opacity-40 animate-pulse" />
        <p className="text-sm">Loading terminal…</p>
        <p className="text-xs mt-1 opacity-50">Starting WASM-OS shell</p>
      </div>
    </div>
  ),
});

const QUICK_CMDS = [
  { cmd: "list",             desc: "all tasks"       },
  { cmd: "start <id>",       desc: "execute task"    },
  { cmd: "security <id>",    desc: "scan binary"     },
  { cmd: "stats",            desc: "system stats"    },
  { cmd: "top",              desc: "live monitor"    },
  { cmd: "audit",            desc: "audit log"       },
  { cmd: "health",           desc: "backend health"  },
  { cmd: "traces",           desc: "trace viewer"    },
  { cmd: "tokens",           desc: "API tokens"      },
  { cmd: "testall",          desc: "run test suite"  },
];

export default function TerminalPage() {
  return (
    <div className="flex flex-col h-screen bg-background">
      {/* ── Toolbar ── */}
      <div className="flex items-center gap-3 border-b border-border bg-card px-4 py-2 shrink-0">
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
          <Badge variant="secondary" className="text-[10px] h-4 px-1.5">v5.0</Badge>
        </div>
        <div className="flex items-center gap-1.5 ml-2">
          <span className="inline-block h-1.5 w-1.5 rounded-full bg-green-400" />
          <span className="text-[11px] text-muted-foreground">REST API</span>
        </div>
        <div className="ml-auto flex items-center gap-1.5">
          <Button
            variant="ghost"
            size="sm"
            className="h-7 text-xs text-muted-foreground gap-1"
            title="Type 'help' in the terminal for all commands"
          >
            <Info size={13} />
            <span className="hidden sm:inline">Type &apos;help&apos; for all commands</span>
          </Button>
        </div>
      </div>

      {/* ── Quick command strip ── */}
      <div className="flex items-center gap-3 border-b border-border/50 bg-card/50 px-4 py-1.5 overflow-x-auto shrink-0">
        <span className="text-[10px] text-muted-foreground uppercase tracking-wider shrink-0 font-semibold">Quick:</span>
        {QUICK_CMDS.map(({ cmd, desc }) => (
          <div key={cmd} className="flex items-center gap-1 shrink-0">
            <code className="text-[10px] font-mono text-primary/80 bg-primary/8 rounded px-1.5 py-0.5">{cmd}</code>
            <span className="text-[10px] text-muted-foreground/60">{desc}</span>
          </div>
        ))}
      </div>

      {/* ── Terminal ── */}
      <div className="flex-1 min-h-0 overflow-hidden p-2">
        <TerminalEmulator />
      </div>
    </div>
  );
}
