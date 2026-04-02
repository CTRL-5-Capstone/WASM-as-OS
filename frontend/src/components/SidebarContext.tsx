'use client';

import React, { createContext, useContext, useState, useEffect } from 'react';

interface SidebarContextValue {
  collapsed: boolean;
  setCollapsed: (v: boolean) => void;
}

const SidebarContext = createContext<SidebarContextValue>({
  collapsed: false,
  setCollapsed: () => {},
});

export function SidebarProvider({ children }: { children: React.ReactNode }) {
  const [collapsed, setCollapsed] = useState(false);

  // Sync CSS variable so any element can react to sidebar width
  useEffect(() => {
    const root = document.documentElement;
    root.style.setProperty(
      '--sidebar-current-width',
      collapsed
        ? 'var(--sidebar-width-collapsed)'
        : 'var(--sidebar-width)'
    );
  }, [collapsed]);

  return (
    <SidebarContext.Provider value={{ collapsed, setCollapsed }}>
      {children}
    </SidebarContext.Provider>
  );
}

export function useSidebar() {
  return useContext(SidebarContext);
}
