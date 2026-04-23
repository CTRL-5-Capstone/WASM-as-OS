"use client";

/**
 * Security Alert Context — fires Sonner toasts on hazardous WASM events.
 * Used by XTerminal, task execution, and the security scan components.
 */

import { createContext, useContext, useCallback, type ReactNode } from "react";
import { toast } from "sonner";
import { ShieldAlert, AlertTriangle, Info } from "lucide-react";

type AlertLevel = "info" | "warn" | "severe";

interface SecurityAlertContextType {
  alert: (msg: string, level?: AlertLevel) => void;
  hazardous: (moduleName: string, reason: string) => void;
  confusedDeputy: (moduleName: string) => void;
}

const SecurityAlertContext = createContext<SecurityAlertContextType>({
  alert: () => {},
  hazardous: () => {},
  confusedDeputy: () => {},
});

export function SecurityAlertProvider({ children }: { children: ReactNode }) {
  const alert = useCallback((msg: string, level: AlertLevel = "warn") => {
    if (level === "severe") {
      toast.error(msg, {
        icon: <ShieldAlert size={16} className="text-red-400" />,
        description: "Immediate review required",
        duration: 8000,
        style: { background: "#1e0a0a", border: "1px solid rgba(239,68,68,0.4)", color: "#fca5a5" },
      });
    } else if (level === "warn") {
      toast.warning(msg, {
        icon: <AlertTriangle size={16} className="text-yellow-400" />,
        description: "Elevated risk detected",
        duration: 5000,
        style: { background: "#1a130a", border: "1px solid rgba(234,179,8,0.4)", color: "#fde68a" },
      });
    } else {
      toast(msg, {
        icon: <Info size={16} className="text-sky-400" />,
        duration: 3000,
      });
    }
    // Persist to audit log
    try {
      const log = JSON.parse(localStorage.getItem("wasm_security_log") || "[]");
      log.push({ ts: new Date().toISOString(), msg, level });
      if (log.length > 200) log.shift();
      localStorage.setItem("wasm_security_log", JSON.stringify(log));
    } catch {}
  }, []);

  const hazardous = useCallback((moduleName: string, reason: string) => {
    toast.error(`🚨 Hazardous Module: ${moduleName}`, {
      description: reason,
      duration: 10000,
      style: { background: "#1e0a0a", border: "1px solid rgba(239,68,68,0.6)", color: "#fca5a5" },
      action: {
        label: "View Report",
        onClick: () => { window.location.href = "/tasks"; },
      },
    });
    try {
      const log = JSON.parse(localStorage.getItem("wasm_security_log") || "[]");
      log.push({ ts: new Date().toISOString(), msg: `HAZARDOUS: ${moduleName} — ${reason}`, level: "severe" });
      localStorage.setItem("wasm_security_log", JSON.stringify(log));
    } catch {}
  }, []);

  const confusedDeputy = useCallback((moduleName: string) => {
    toast.error(`⚠ Confused Deputy Attempt: ${moduleName}`, {
      description: "Module attempted privilege escalation or unauthorized resource access",
      duration: 10000,
      style: { background: "#1e0a0a", border: "1px solid rgba(239,68,68,0.6)", color: "#fca5a5" },
    });
  }, []);

  return (
    <SecurityAlertContext.Provider value={{ alert, hazardous, confusedDeputy }}>
      {children}
    </SecurityAlertContext.Provider>
  );
}

export function useSecurityAlert() {
  return useContext(SecurityAlertContext);
}
