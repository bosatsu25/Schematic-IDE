// Schematic IDE service worker.
//
// This file is a template: `service-worker/plugin.ts` fills in the build hash
// and the precache manifest (every file Vite emitted, including hashed
// `assets/*` and the WASM module) and writes the result to `dist/sw.js`.
//
// Strategy:
// - All build output is precached into ONE cache named after the build hash,
//   and served cache-first from that cache only. JS, the worker bundle and the
//   WASM module therefore always come from the same build; nothing is
//   revalidated individually.
// - A new deploy changes the hash, so the browser installs a new worker that
//   precaches the new build into a new cache. It then WAITS until the page
//   posts SKIP_WAITING (the "New version available - Reload" banner), so a
//   running session never mixes builds.
// - On activate, caches from older builds are deleted.
const BUILD_HASH = '__SW_BUILD_HASH__';
const PRECACHE_URLS = /* __SW_PRECACHE_URLS__ */ [];

const CACHE_PREFIX = 'schematic-ide-';
const CACHE_NAME = `${CACHE_PREFIX}${BUILD_HASH}`;
// PRECACHE_URLS are relative to this script, so the build can be hosted at a
// domain root or under any sub-path.
const BASE_URL = new URL('./', self.location.href);
const toAbsolute = (relative) => new URL(relative, BASE_URL).href;
const APP_SHELL_URL = toAbsolute('index.html');
const PRECACHED = new Set(PRECACHE_URLS.map(toAbsolute));

self.addEventListener('install', (event) => {
  event.waitUntil(
    caches
      .open(CACHE_NAME)
      .then((cache) =>
        // `cache: 'reload'` bypasses the HTTP cache so we never store a stale
        // file from a previous deploy under this build's cache.
        cache.addAll(
          PRECACHE_URLS.map((url) => new Request(toAbsolute(url), { cache: 'reload' })),
        ),
      ),
  );
});

self.addEventListener('message', (event) => {
  if (event.data && event.data.type === 'SKIP_WAITING') {
    self.skipWaiting();
  }
});

self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) =>
        Promise.all(
          keys
            .filter((key) => key.startsWith(CACHE_PREFIX) && key !== CACHE_NAME)
            .map((key) => caches.delete(key)),
        ),
      )
      .then(() => self.clients.claim()),
  );
});

self.addEventListener('fetch', (event) => {
  const { request } = event;
  if (request.method !== 'GET') return;

  const url = new URL(request.url);
  if (url.origin !== self.location.origin) return;

  // Ignore query and fragment when matching build files.
  const href = url.origin + url.pathname;
  let cacheKey = null;
  if (PRECACHED.has(href)) {
    cacheKey = href;
  } else if (request.mode === 'navigate' && href.startsWith(BASE_URL.href)) {
    // Single-page app: every navigation gets this build's app shell.
    cacheKey = APP_SHELL_URL;
  }
  if (cacheKey === null) return; // Not part of the build: plain network.

  event.respondWith(
    caches
      .open(CACHE_NAME)
      .then((cache) => cache.match(cacheKey))
      .then((cached) => cached || fetch(request)),
  );
});
