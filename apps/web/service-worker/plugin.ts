import { createHash } from 'node:crypto';
import { readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { dirname, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import type { Plugin } from 'vite';

const HASH_PLACEHOLDER = '__SW_BUILD_HASH__';
const MANIFEST_PLACEHOLDER = '/* __SW_PRECACHE_URLS__ */ []';
const DEFAULT_TEMPLATE = join(dirname(fileURLToPath(import.meta.url)), 'sw.template.js');

export interface PrecacheEntry {
  /** URL path the file is served at, e.g. `/assets/index-abc123.js`. */
  url: string;
  content: Uint8Array;
}

/**
 * Fills the service-worker template with a precache manifest and a build hash
 * derived from the URL and bytes of every precached file, so any change to the
 * build output yields a new cache name.
 */
export function renderServiceWorker(
  template: string,
  entries: PrecacheEntry[],
): { source: string; hash: string; urls: string[] } {
  if (!template.includes(HASH_PLACEHOLDER) || !template.includes(MANIFEST_PLACEHOLDER)) {
    throw new Error('service worker template is missing its build placeholders');
  }
  const sorted = [...entries].sort((a, b) => (a.url < b.url ? -1 : a.url > b.url ? 1 : 0));
  const digest = createHash('sha256');
  for (const entry of sorted) {
    digest.update(entry.url);
    digest.update('\0');
    digest.update(entry.content);
    digest.update('\0');
  }
  const hash = digest.digest('hex').slice(0, 16);
  const urls = sorted.map((entry) => entry.url);
  const source = template
    .replace(HASH_PLACEHOLDER, hash)
    .replace(MANIFEST_PLACEHOLDER, JSON.stringify(urls, null, 2));
  return { source, hash, urls };
}

function listFiles(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const full = join(dir, entry.name);
    return entry.isDirectory() ? listFiles(full) : [full];
  });
}

export interface ServiceWorkerPluginOptions {
  /** Output file name inside `build.outDir`. */
  fileName?: string;
  /** Path to the template. Defaults to `service-worker/sw.template.js`. */
  template?: string;
}

/**
 * Emits `sw.js` after a production build, precaching every emitted file
 * (Vite's hashed `assets/*`, `index.html`, and everything copied from
 * `public/`, including the WASM module) under a build-hashed cache name.
 */
export function serviceWorkerPlugin(options: ServiceWorkerPluginOptions = {}): Plugin {
  const fileName = options.fileName ?? 'sw.js';
  const templatePath = options.template ?? DEFAULT_TEMPLATE;
  let outDir = '';

  return {
    name: 'schematic-ide:service-worker',
    apply: 'build',
    configResolved(config) {
      outDir = resolve(config.root, config.build.outDir);
    },
    writeBundle() {
      // By now Vite has written the bundle and copied `public/` into outDir.
      const entries: PrecacheEntry[] = listFiles(outDir)
        .map((file) => relative(outDir, file).split(sep).join('/'))
        .filter((rel) => rel !== fileName && !rel.endsWith('.map'))
        .map((rel) => ({
          // Relative to sw.js, which sits at the root of the build output.
          url: rel,
          content: readFileSync(join(outDir, rel)),
        }));
      const { source, hash, urls } = renderServiceWorker(
        readFileSync(templatePath, 'utf8'),
        entries,
      );
      writeFileSync(join(outDir, fileName), source);
      this.info(`${fileName}: precaching ${urls.length} files, cache schematic-ide-${hash}`);
    },
  };
}
