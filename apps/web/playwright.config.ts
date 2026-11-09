import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './e2e',
  timeout: 30000,
  use: {
    baseURL: 'http://localhost:5173',
    headless: true,
    channel: process.platform === 'win32' ? 'chrome' : undefined,
  },
  webServer: {
    // `pnpm dev` rebuilds the WASM module first (no-op when already fresh);
    // the generous timeout covers a cold cargo build.
    command: 'pnpm dev --port 5173 --strictPort',
    port: 5173,
    reuseExistingServer: true,
    timeout: 180000,
  },
});
