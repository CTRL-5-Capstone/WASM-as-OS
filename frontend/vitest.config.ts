import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    environment: 'jsdom',
    globals: true,
    // Tells vitest to look INSIDE source files for `if (import.meta.vitest)` blocks
    includeSource: ['src/**/*.{ts,tsx}'],
  },
  define: {
    // Strips test code from production bundles via dead-code elimination
    'import.meta.vitest': 'undefined',
  },
});