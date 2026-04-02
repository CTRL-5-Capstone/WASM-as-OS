'use client';

import { usePathname } from 'next/navigation';
import Sidebar from '@/components/Sidebar';
import { ErrorBoundary } from '@/components/ErrorBoundary';
import { useSidebar } from '@/components/SidebarContext';

export default function AppShell({ children }: { children: React.ReactNode }) {
  const pathname  = usePathname();
  const { collapsed } = useSidebar();

  // Full-screen routes (no sidebar chrome)
  const isFullScreen =
    pathname === '/demo' ||
    pathname.startsWith('/demo/') ||
    pathname === '/terminal' ||
    pathname.startsWith('/terminal/');

  if (isFullScreen) {
    return (
      <main id="main-content" className="min-h-screen bg-background">
        {children}
      </main>
    );
  }

  const marginLeft = collapsed
    ? 'var(--sidebar-width-collapsed)'
    : 'var(--sidebar-width)';

  return (
    <div className="flex min-h-screen bg-background">
      <Sidebar />
      <main
        id="main-content"
        role="main"
        aria-label="Page content"
        className="flex-1 min-h-screen overflow-y-auto"
        style={{ marginLeft, transition: 'margin-left 300ms' }}
      >
        <div className="mx-auto max-w-[1440px] px-6 py-8 md:px-10">
          <ErrorBoundary>{children}</ErrorBoundary>
        </div>
      </main>
    </div>
  );
}
