"use client";

import { TerminalProvider } from "@/lib/terminal-context";
import { SecurityAlertProvider } from "@/lib/security-alerts";
import { SidebarProvider } from "@/components/SidebarContext";
import { Toaster } from "sonner";

export default function Providers({ children }: { children: React.ReactNode }) {
  return (
    <SecurityAlertProvider>
      <SidebarProvider>
      <TerminalProvider>
        {children}
        <Toaster
          position="top-right"
          richColors
          closeButton
          toastOptions={{
            style: { fontFamily: "inherit" },
          }}
        />
      </TerminalProvider>
      </SidebarProvider>
    </SecurityAlertProvider>
  );
}
