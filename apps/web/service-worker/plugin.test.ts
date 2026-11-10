import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { renderServiceWorker } from './plugin';

const template = readFileSync(join(__dirname, 'sw.template.js'), 'utf8');
const enc = (s: string) => new TextEncoder().encode(s);

describe('renderServiceWorker', () => {
  const entries = [
    { url: '/schematic_wasm.wasm', content: enc('wasm-v1') },
    { url: '/assets/index-abc.js', content: enc('js-v1') },
    { url: '/index.html', content: enc('<html>') },
  ];

  it('embeds every precached URL (sorted) and a build-hashed cache name', () => {
    const { source, hash, urls } = renderServiceWorker(template, entries);
    expect(urls).toEqual(['/assets/index-abc.js', '/index.html', '/schematic_wasm.wasm']);
    expect(hash).toMatch(/^[0-9a-f]{16}$/);
    expect(source).toContain(`const BUILD_HASH = '${hash}';`);
    expect(source).toContain('"/assets/index-abc.js"');
    expect(source).toContain('"/schematic_wasm.wasm"');
    expect(source).not.toContain('__SW_');
  });

  it('is deterministic regardless of input order', () => {
    const a = renderServiceWorker(template, entries);
    const b = renderServiceWorker(template, [...entries].reverse());
    expect(a.hash).toBe(b.hash);
    expect(a.source).toBe(b.source);
  });

  it('changes the cache name when any file content changes', () => {
    const a = renderServiceWorker(template, entries);
    const b = renderServiceWorker(
      template,
      entries.map((e) => (e.url.endsWith('.wasm') ? { ...e, content: enc('wasm-v2') } : e)),
    );
    expect(a.hash).not.toBe(b.hash);
  });

  it('rejects a template without placeholders', () => {
    expect(() => renderServiceWorker('self.addEventListener("fetch", () => {});', entries)).toThrow(
      /placeholders/,
    );
  });
});
