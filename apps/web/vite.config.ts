import { defineConfig } from 'vitest/config';
import { readFileSync } from 'node:fs';
import react from '@vitejs/plugin-react';
import { serviceWorkerPlugin } from './service-worker/plugin.ts';

const { version } = JSON.parse(
  readFileSync(new URL('./package.json', import.meta.url), 'utf8'),
) as { version: string };

export default defineConfig({
  // Shown in the status bar so bug reports can name the build.
  define: { __APP_VERSION__: JSON.stringify(version) },
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
