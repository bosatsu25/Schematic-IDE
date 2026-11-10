import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';
import { serviceWorkerPlugin } from './service-worker/plugin.ts';

export default defineConfig({
  // Relative asset URLs: the same build works at a domain root, under a
  // sub-path (GitHub Pages project sites) or from an unzipped folder server.
  base: './',
  plugins: [react(), serviceWorkerPlugin()],
  test: {
    environment: 'jsdom',
    globals: true,
    exclude: ['**/node_modules/**', '**/dist/**', 'e2e/**'],
  },
});
