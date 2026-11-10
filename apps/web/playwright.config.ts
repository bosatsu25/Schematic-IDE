import { defineConfig } from '@playwright/test';

const DEV_PORT = 5173;
const PREVIEW_PORT = 4173;

export default defineConfig({
  testDir: './e2e',
  timeout: 30000,
  use: {
    headless: true,
    channel: process.platform === 'win32' ? 'chrome' : undefined,
  },
  projects: [
    {
      // Editor workflows against the Vite dev server.
      name: 'dev',
      testIgnore: /\.preview\.spec\.ts$/,
      use: { baseURL: `http://localhost:${DEV_PORT}` },
    },
    {
      // Production-only behaviour (service worker, offline) against `vite preview`.
      name: 'preview',
      testMatch: /\.preview\.spec\.ts$/,
      use: { baseURL: `http://localhost:${PREVIEW_PORT}` },
    },
  ],
  webServer: [
    {
      // `pnpm dev` rebuilds the WASM module first (no-op when already fresh);
      // the generous timeout covers a cold cargo build.
      command: `pnpm dev --port ${DEV_PORT} --strictPort`,
      port: DEV_PORT,
      reuseExistingServer: true,
      timeout: 180000,
    },
    {
      // Always serve a fresh production build in CI so sw.js matches the code.
      command: `pnpm build && pnpm preview --port ${PREVIEW_PORT} --strictPort`,
      port: PREVIEW_PORT,
      reuseExistingServer: !process.env.CI,
      timeout: 180000,
    },
  ],
});
