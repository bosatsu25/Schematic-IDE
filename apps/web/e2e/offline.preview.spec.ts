import { test, expect } from '@playwright/test';
import * as path from 'path';
import * as fs from 'fs';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Runs against the production build (`vite preview`, see playwright.config.ts),
// where the generated sw.js is registered.
test.describe('Offline PWA shell (production build)', () => {
  test('first visit installs the service worker; the app and sample load offline', async ({
    page,
    context,
  }) => {
    // 1. First (online) visit installs and activates the service worker,
    //    which precaches the whole build and then claims this page.
    // Relative to the project's baseURL, which is a sub-path of the server.
    await page.goto('./');
    const base = new URL(page.url()).pathname;
    expect(base).toBe('/schematic-ide/');
    await expect(page.locator('.app-title')).toHaveText('Schematic IDE');
    await page.evaluate(() => navigator.serviceWorker.ready.then(() => undefined));
    await page.waitForFunction(() => navigator.serviceWorker.controller !== null);

    // Exactly one build-versioned cache holds the JS bundles and the WASM.
    const cacheState = await page.evaluate(async () => {
      const names = (await caches.keys()).filter((n) => n.startsWith('schematic-ide-'));
      const cache = await caches.open(names[0]);
      const urls = (await cache.keys()).map((r) => new URL(r.url).pathname);
      return { names, urls };
    });
    expect(cacheState.names).toHaveLength(1);
    expect(cacheState.names[0]).toMatch(/^schematic-ide-[0-9a-f]{16}$/);
    const cached = cacheState.urls.map((u) => (u.startsWith(base) ? u.slice(base.length) : u));
    expect(cached).toContain('index.html');
    expect(cached).toContain('schematic_wasm.wasm');
    expect(cached).toContain('sample.litematic');
    expect(cached.some((u) => /^assets\/index-.*\.js$/.test(u))).toBe(true);
    expect(cached.some((u) => /^assets\/schematic\.worker-.*\.js$/.test(u))).toBe(true);

    // 2. Go offline and reload: the app shell must come from the cache.
    await context.setOffline(true);
    await page.reload();
    await expect(page.locator('.app-title')).toHaveText('Schematic IDE');
    await expect(page.getByTestId('empty-state')).toBeVisible();
    await expect(page.getByTestId('sw-update-banner')).toHaveCount(0);

    // Sanity check: the network really is down for anything not precached.
    const uncachedFetch = await page.evaluate(() =>
      fetch('not-precached.txt').then(
        () => 'reachable',
        () => 'offline',
      ),
    );
    expect(uncachedFetch).toBe('offline');

    // The bundled sample is served from the precache while offline.
    const sampleBytes = await page.evaluate(async () => {
      const res = await fetch('sample.litematic');
      return res.ok ? (await res.arrayBuffer()).byteLength : -1;
    });
    const fixturePath = path.resolve(__dirname, '../public/sample.litematic');
    expect(sampleBytes).toBe(fs.statSync(fixturePath).size);

    // 3. Opening the sample offline exercises the worker bundle + WASM engine,
    //    both served from the same versioned cache.
    await page.getByTestId('open-file-input').setInputFiles(fixturePath);
    await expect(page.getByTestId('doc-name')).toHaveText('WasmTestSchematic');
    await expect(page.getByTestId('region-block-count')).toHaveText('12');
    await expect(page.getByTestId('viewport-canvas')).toBeVisible();

    await context.setOffline(false);
  });
});
