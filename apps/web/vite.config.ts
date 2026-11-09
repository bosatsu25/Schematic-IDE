import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { serviceWorkerPlugin } from './service-worker/plugin';

export default defineConfig({
  plugins: [react(), serviceWorkerPlugin()],
  test: {
    environment: 'jsdom',
    globals: true,
    exclude: ['**/node_modules/**', '**/dist/**', 'e2e/**'],
  },
});
