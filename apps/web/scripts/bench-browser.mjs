#!/usr/bin/env node
/**
 * Browser / WASM scale benchmark harness for Schematic IDE.
 *
 *   pnpm --filter @schematic-ide/web bench:browser -- --scales 1m,5m,10m
 *
 * Steps:
 *   1. cargo build -p schematic-wasm --target wasm32-unknown-unknown --release
 *   2. Generate dense .litematic fixtures with
 *      `cargo run --release -p schematic-wasm --example gen_dense_fixture`
 *   3. vite build of the app + the benchmark page (apps/web/bench/) into
 *      target/bench-web, with the fresh release wasm copied over the committed
 *      public/schematic_wasm.wasm, then serve it with `vite preview`.
 *   4. For every scale, in a fresh headless Chromium:
 *      - "engine" mode: apps/web/bench/index.html drives SchematicEngine and
 *        VoxelRenderer on the main thread and reports per-phase timings.
 *      - "app" mode: the real app UI; the file is opened through the file
 *        input and the worker round-trips / first WebGL draw are observed via
 *        an init script (no app code changes).
 *   5. Print a summary and write JSON to target/bench-results/.
 *
 * Options:
 *   --scales <list>     comma list of 32,1m,5m,10m (default: 1m)
 *   --modes <list>      comma list of engine,app (default: engine,app)
 *   --no-render         engine mode: skip the WebGL render step
 *   --skip-build        reuse the existing wasm / vite build / fixtures
 *   --timeout <sec>     per scale and mode (default: 600)
 *   --wasm <file>       use this prebuilt .wasm instead of building one
 *   --out <file>        JSON output path
 *
 * Not part of CI. Requires the wasm32-unknown-unknown target and a Playwright
 * Chromium (already needed for `pnpm test:e2e`).
 */
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from '@playwright/test';
import { build, preview } from 'vite';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const appDir = path.resolve(scriptDir, '..');
const repoRoot = path.resolve(appDir, '../..');
const targetDir = path.join(repoRoot, 'target');
const fixtureDir = path.join(targetDir, 'bench-fixtures');
const webOutDir = path.join(targetDir, 'bench-web');
const defaultWasmPath = path.join(
  targetDir,
  'wasm32-unknown-unknown',
  'release',
  'schematic_wasm.wasm',
);
const MIB = 1024 * 1024;

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

function parseArgs(argv) {
  const opts = {
    scales: ['1m'],
    modes: ['engine', 'app'],
    render: true,
    skipBuild: false,
    wasm: null,
    timeoutSec: 600,
    out: null,
  };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--') continue;
    if (a === '--scales') opts.scales = argv[++i].split(',').map((s) => s.trim().toLowerCase());
    else if (a === '--modes') opts.modes = argv[++i].split(',').map((s) => s.trim());
    else if (a === '--no-render') opts.render = false;
    else if (a === '--skip-build') opts.skipBuild = true;
    else if (a === '--wasm') opts.wasm = path.resolve(argv[++i]);
    else if (a === '--timeout') opts.timeoutSec = Number(argv[++i]);
    else if (a === '--out') opts.out = path.resolve(argv[++i]);
    else if (a === '--help' || a === '-h') {
      console.log(fs.readFileSync(fileURLToPath(import.meta.url), 'utf8').split('*/')[0]);
      process.exit(0);
    } else {
      throw new Error(`unknown argument: ${a}`);
    }
  }
  return opts;
}

function run(cmd, args, { capture = false } = {}) {
  const res = spawnSync(cmd, args, {
    cwd: repoRoot,
    stdio: capture ? ['ignore', 'pipe', 'inherit'] : 'inherit',
    encoding: 'utf8',
  });
  if (res.status !== 0) {
    throw new Error(`${cmd} ${args.join(' ')} failed with status ${res.status}`);
  }
  return res.stdout;
}

// ---------------------------------------------------------------------------
// Process memory (Linux only, best effort)
// ---------------------------------------------------------------------------

function readProc(pid, file) {
  try {
    return fs.readFileSync(`/proc/${pid}/${file}`, 'utf8');
  } catch {
    return null;
  }
}

function descendants(rootPid) {
  if (!fs.existsSync('/proc')) return [];
  const parent = new Map();
  for (const entry of fs.readdirSync('/proc')) {
    if (!/^\d+$/.test(entry)) continue;
    const stat = readProc(entry, 'stat');
    if (!stat) continue;
    const fields = stat.slice(stat.lastIndexOf(')') + 2).split(' ');
    parent.set(Number(entry), Number(fields[1]));
  }
  const out = [];
  const stack = [rootPid];
  while (stack.length) {
    const p = stack.pop();
    for (const [child, pp] of parent) {
      if (pp === p) {
        out.push(child);
        stack.push(child);
      }
    }
  }
  return out;
}

function processKind(pid) {
  const cmd = readProc(pid, 'cmdline');
  if (!cmd) return null;
  const m = /--type=([a-z-]+)/.exec(cmd);
  if (!m) return null;
  if (m[1] === 'utility' && cmd.includes('NetworkService')) return 'network';
  return m[1];
}

/** Polls VmHWM (peak RSS) of Chromium child processes, keyed by process type. */
class ChromiumMemoryProbe {
  constructor(rootPid) {
    this.rootPid = rootPid;
    this.peakMb = {};
    this.timer = fs.existsSync('/proc') ? setInterval(() => this.sample(), 250) : null;
  }

  sample() {
    for (const pid of descendants(this.rootPid)) {
      const kind = processKind(pid);
      if (!kind) continue;
      const status = readProc(pid, 'status');
      const m = status && /VmHWM:\s+(\d+)/.exec(status);
      if (!m) continue;
      const mb = Number(m[1]) / 1024;
      this.peakMb[kind] = Math.max(this.peakMb[kind] ?? 0, mb);
    }
  }

  stop() {
    if (this.timer) {
      this.sample();
      clearInterval(this.timer);
    }
    return Object.fromEntries(
      Object.entries(this.peakMb).map(([k, v]) => [k, Math.round(v)]),
    );
  }
}

// ---------------------------------------------------------------------------
// Browser runs
// ---------------------------------------------------------------------------

async function withBrowser(timeoutSec, fn) {
  const browser = await chromium.launch({
    // precise-memory-info makes performance.memory report real numbers.
    args: ['--enable-precise-memory-info'],
  });
  // Chromium is spawned as a descendant of this Node process; only one
  // browser is alive at a time, so every typed child process belongs to it.
  const probe = new ChromiumMemoryProbe(process.pid);
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();
  const stages = {};
  const consoleErrors = [];
  page.on('console', (msg) => {
    const text = msg.text();
    if (text.startsWith('BENCH_STAGE ')) {
      Object.assign(stages, JSON.parse(text.slice('BENCH_STAGE '.length)));
    } else if (msg.type() === 'error') {
      consoleErrors.push(text.slice(0, 300));
    }
  });
  page.on('pageerror', (err) => consoleErrors.push(String(err).slice(0, 300)));

  let crashed = false;
  const crash = new Promise((_, reject) =>
    page.on('crash', () => {
      crashed = true;
      reject(new Error('renderer process crashed (likely out of memory)'));
    }),
  );
  let timer;
  const timeout = new Promise((_, reject) => {
    timer = setTimeout(
      () => reject(new Error(`timed out after ${timeoutSec}s`)),
      timeoutSec * 1000,
    );
  });

  const result = { stages, consoleErrors };
  try {
    Object.assign(result, await Promise.race([fn(page), crash, timeout]));
    result.ok = true;
  } catch (err) {
    result.ok = false;
    result.error = err instanceof Error ? err.message : String(err);
  } finally {
    clearTimeout(timer);
    if (!crashed) {
      try {
        const cdp = await context.newCDPSession(page);
        await cdp.send('Performance.enable');
        const { metrics } = await cdp.send('Performance.getMetrics');
        const get = (name) => metrics.find((m) => m.name === name)?.value;
        result.mainThreadJsHeapUsedMb = (get('JSHeapUsedSize') ?? 0) / MIB;
        result.mainThreadJsHeapTotalMb = (get('JSHeapTotalSize') ?? 0) / MIB;
      } catch {
        // page may be unresponsive; ignore
      }
    }
    result.processPeakRssMb = probe.stop();
    result.chromiumVersion = browser.version();
    await browser.close().catch(() => {});
  }
  return result;
}

async function runEngineMode(baseUrl, fixture, opts) {
  return withBrowser(opts.timeoutSec, async (page) => {
    await page.goto(`${baseUrl}/bench/index.html`);
    await page.waitForFunction(() => window.__benchReady === true);
    const results = await page.evaluate(
      (o) => window.__runBench(o),
      {
        wasmUrl: '/schematic_wasm.wasm',
        fixtureUrl: `/bench-fixtures/${path.basename(fixture.path)}`,
        regionName: fixture.region,
        render: opts.render,
      },
    );
    return { results };
  });
}

/** Installed before any app script: observes worker traffic and WebGL draws. */
function appInstrumentation() {
  const B = {
    events: [],
    changeAt: null,
    firstDrawAt: null,
    firstFrameAt: null,
    firstFrameGpuDoneAt: null,
    drawCalls: 0,
    instancesDrawn: 0,
    longTasks: [],
  };
  window.__bench = B;
  try {
    new PerformanceObserver((list) => {
      for (const e of list.getEntries()) {
        if (B.changeAt !== null) B.longTasks.push({ start: e.startTime, ms: e.duration });
      }
    }).observe({ type: 'longtask', buffered: true });
  } catch {
    // longtask timing unsupported
  }
  // After the first frame's draw calls, insert a GPU fence and poll it without
  // blocking, to learn when the GPU (SwiftShader in headless) actually finished.
  const watchGpu = (gl) => {
    setTimeout(() => {
      const sync = gl.fenceSync(gl.SYNC_GPU_COMMANDS_COMPLETE, 0);
      gl.flush();
      const poll = () => {
        if (gl.getSyncParameter(sync, gl.SYNC_STATUS) === gl.SIGNALED) {
          B.firstFrameGpuDoneAt = performance.now();
          gl.deleteSync(sync);
        } else {
          setTimeout(poll, 5);
        }
      };
      poll();
    }, 0);
  };
  document.addEventListener(
    'change',
    (e) => {
      const t = e.target;
      if (t && t.type === 'file' && B.changeAt === null) B.changeAt = performance.now();
    },
    true,
  );
  const NativeWorker = window.Worker;
  window.Worker = class extends NativeWorker {
    constructor(...args) {
      super(...args);
      this.addEventListener('message', (e) => {
        const d = e.data || {};
        B.events.push({ dir: 'recv', id: d.id, ok: d.ok, error: d.error, t: performance.now() });
      });
    }
    postMessage(msg, ...rest) {
      B.events.push({ dir: 'send', id: msg && msg.id, type: msg && msg.type, t: performance.now() });
      return super.postMessage(msg, ...rest);
    }
  };
  const proto = window.WebGL2RenderingContext && window.WebGL2RenderingContext.prototype;
  if (proto) {
    // Chunk meshes are indexed (drawElements); the legacy renderer used
    // instanced draws. Overlay lines use drawArrays and are not counted.
    for (const name of ['drawElements', 'drawElementsInstanced', 'drawArraysInstanced']) {
      const orig = proto[name];
      proto[name] = function (...a) {
        if (B.changeAt !== null) {
          B.drawCalls++;
          B.instancesDrawn += name === 'drawElements' ? 1 : a[a.length - 1];
          if (B.firstDrawAt === null) {
            B.firstDrawAt = performance.now();
            requestAnimationFrame(() => {
              B.firstFrameAt = performance.now();
            });
            watchGpu(this);
          }
        }
        return orig.apply(this, a);
      };
    }
  }
}

function summarizeAppEvents(bench) {
  const sends = new Map();
  const calls = [];
  for (const ev of bench.events) {
    if (ev.dir === 'send') sends.set(ev.id, ev);
  }
  for (const ev of bench.events) {
    if (ev.dir !== 'recv') continue;
    const s = sends.get(ev.id);
    if (!s || bench.changeAt === null || s.t < bench.changeAt) continue;
    calls.push({
      type: s.type,
      ms: ev.t - s.t,
      ok: ev.ok,
      error: ev.error,
      startMs: s.t - bench.changeAt,
      endMs: ev.t - bench.changeAt,
    });
  }
  return calls;
}

async function runAppMode(baseUrl, fixture, opts) {
  return withBrowser(opts.timeoutSec, async (page) => {
    await page.addInitScript(appInstrumentation);
    await page.goto(`${baseUrl}/`);
    await page.locator('.app-title').waitFor();
    await page.getByTestId('empty-state').waitFor();
    await page.getByTestId('open-file-input').setInputFiles(fixture.path);

    // Done when the post-load GET_STATUS round trip (last step of the store's
    // loadBuffer) has returned, or a worker call failed, and either a frame was
    // drawn or the load failed.
    let bench;
    for (;;) {
      bench = await page.evaluate(() => window.__bench);
      const calls = summarizeAppEvents(bench);
      const failed = calls.some((c) => c.ok === false);
      const loadEnd = calls.find((c) => c.type === 'LOAD_LITEMATIC')?.endMs;
      const statusDone = calls.some(
        (c) => c.type === 'GET_STATUS' && loadEnd !== undefined && c.startMs >= loadEnd,
      );
      if (failed || (statusDone && bench.firstFrameGpuDoneAt !== null)) break;
      await page.waitForTimeout(250);
    }

    const calls = summarizeAppEvents(bench);
    const shownCount = await page
      .getByTestId('region-block-count')
      .textContent({ timeout: 1000 })
      .catch(() => null);
    const errorText = await page
      .locator('.error-banner, [data-testid="error-message"]')
      .first()
      .textContent({ timeout: 500 })
      .catch(() => null);
    const rel = (t) => (t === null || bench.changeAt === null ? null : t - bench.changeAt);
    return {
      results: {
        calls,
        first_draw_call_ms: rel(bench.firstDrawAt),
        first_raf_after_draw_ms: rel(bench.firstFrameAt),
        first_frame_gpu_done_ms: rel(bench.firstFrameGpuDoneAt),
        app_idle_ms: Math.max(0, ...calls.map((c) => c.endMs)),
        longest_main_thread_task_ms: Math.max(0, ...bench.longTasks.map((t) => t.ms)),
        total_long_task_ms: bench.longTasks.reduce((sum, t) => sum + t.ms, 0),
        draw_calls: bench.drawCalls,
        instances_drawn: bench.instancesDrawn,
        ui_block_count: shownCount,
        ui_error: errorText,
      },
    };
  });
}

// ---------------------------------------------------------------------------
// Reporting
// ---------------------------------------------------------------------------

const f1 = (v) => (typeof v === 'number' ? v.toFixed(1) : (v ?? 'n/a'));
const f0 = (v) => (typeof v === 'number' ? v.toFixed(0) : (v ?? 'n/a'));
const mib = (v) => (typeof v === 'number' ? (v / MIB).toFixed(1) : 'n/a');

function printEngine(fixture, r) {
  const s = { ...r.stages, ...(r.results ?? {}) };
  console.log(`\n=== engine mode: ${fixture.scale} (${fixture.occupied_blocks} blocks) ===`);
  if (!r.ok) console.log(`  FAILED: ${r.error}`);
  console.log(`  wasm init:               ${f1(s.wasm_init_ms)} ms (${mib(s.wasm_bytes)} MiB wasm)`);
  console.log(`  load -> summary:         ${f1(s.load_to_summary_ms)} ms`);
  console.log(
    `  getRegionMesh total:     ${f1(s.get_region_mesh_total_ms)} ms ` +
      `(wasm ${f1(s.mesh_wasm_ms)}, decode ${f1(s.mesh_decode_ms)}, JSON.parse ${f1(s.mesh_json_parse_ms)})`,
  );
  console.log(`  mesh JSON size:          ${mib(s.mesh_json_bytes)} MiB`);
  if (s.faces_bytes !== undefined) {
    console.log(
      `  regionFaces (binary):    ${f1(s.faces_wasm_ms)} ms wasm + ${f1(s.faces_copy_ms)} ms copy, ` +
        `${mib(s.faces_bytes)} MiB, ${s.faces_count} faces in ${s.faces_chunks} chunks`,
    );
  } else {
    console.log('  regionFaces (binary):    not exported by this wasm build');
  }
  console.log(`  structuredClone(mesh):   ${f1(s.mesh_structured_clone_ms)} ms`);
  console.log(
    `  first frame:             build+submit ${f1(s.render_build_and_submit_ms)} ms, ` +
      `GPU done ${f1(s.first_frame_gpu_done_ms)} ms, presented ${f1(s.first_frame_presented_ms)} ms`,
  );
  console.log(`  steady-state frame:      ${f1(s.steady_frame_ms)} ms`);
  console.log(
    `  wasm memory:             ${f0(s.wasm_memory_after_load_mb)} MiB after load, ` +
      `${f0(s.wasm_memory_after_mesh_mb)} MiB after mesh`,
  );
  console.log(
    `  JS heap (main):          ${f0(s.js_heap_after_load_mb)} -> ${f0(s.js_heap_after_mesh_mb)} ` +
      `-> ${f0(s.js_heap_after_render_mb)} MiB (load -> mesh -> render)`,
  );
  console.log(`  process peak RSS (MiB):  ${JSON.stringify(r.processPeakRssMb)}`);
  if (r.consoleErrors?.length) console.log(`  console errors: ${r.consoleErrors.slice(0, 3).join(' | ')}`);
}

function printApp(fixture, r) {
  console.log(`\n=== app mode: ${fixture.scale} (${fixture.occupied_blocks} blocks) ===`);
  if (!r.ok) console.log(`  FAILED: ${r.error}`);
  const res = r.results ?? {};
  for (const c of res.calls ?? []) {
    console.log(
      `  ${c.type.padEnd(20)} ${f1(c.ms).padStart(10)} ms  (t+${f1(c.startMs)} -> t+${f1(c.endMs)})` +
        (c.ok === false ? `  ERROR: ${String(c.error).slice(0, 120)}` : ''),
    );
  }
  console.log(`  first instanced draw:    t+${f1(res.first_draw_call_ms)} ms`);
  console.log(`  first rAF after draw:    t+${f1(res.first_raf_after_draw_ms)} ms`);
  console.log(`  first frame GPU done:    t+${f1(res.first_frame_gpu_done_ms)} ms (fence)`);
  console.log(`  app idle (last call):    t+${f1(res.app_idle_ms)} ms`);
  console.log(
    `  main-thread long tasks:  longest ${f1(res.longest_main_thread_task_ms)} ms, ` +
      `total ${f1(res.total_long_task_ms)} ms`,
  );
  console.log(`  instances drawn:         ${res.instances_drawn} in ${res.draw_calls} draw calls`);
  console.log(`  UI block count:          ${res.ui_block_count}  UI error: ${res.ui_error ?? 'none'}`);
  console.log(`  main JS heap (CDP):      ${f0(r.mainThreadJsHeapUsedMb)} MiB used`);
  console.log(`  process peak RSS (MiB):  ${JSON.stringify(r.processPeakRssMb)}`);
  if (r.consoleErrors?.length) console.log(`  console errors: ${r.consoleErrors.slice(0, 3).join(' | ')}`);
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

async function main() {
  const opts = parseArgs(process.argv.slice(2));
  fs.mkdirSync(fixtureDir, { recursive: true });

  const wasmPath = opts.wasm ?? defaultWasmPath;
  if (!opts.wasm && (!opts.skipBuild || !fs.existsSync(wasmPath))) {
    console.log('> building release wasm');
    run('cargo', ['build', '-p', 'schematic-wasm', '--target', 'wasm32-unknown-unknown', '--release']);
  }
  if (!opts.skipBuild) {
    console.log('> building fixture generator');
    run('cargo', ['build', '--release', '-p', 'schematic-wasm', '--example', 'gen_dense_fixture']);
  }
  const genBin = path.join(
    targetDir,
    'release',
    'examples',
    process.platform === 'win32' ? 'gen_dense_fixture.exe' : 'gen_dense_fixture',
  );

  const fixtures = [];
  for (const scale of opts.scales) {
    const out = path.join(fixtureDir, `dense-${scale}.litematic`);
    const metaPath = `${out}.json`;
    if (opts.skipBuild && fs.existsSync(out) && fs.existsSync(metaPath)) {
      fixtures.push(JSON.parse(fs.readFileSync(metaPath, 'utf8')));
      continue;
    }
    console.log(`> generating ${scale} fixture`);
    const line = run(genBin, [scale, out], { capture: true }).trim();
    fs.writeFileSync(metaPath, line);
    fixtures.push(JSON.parse(line));
  }

  const configFile = path.join(appDir, 'vite.config.ts');
  if (!opts.skipBuild || !fs.existsSync(path.join(webOutDir, 'bench', 'index.html'))) {
    console.log('> vite build (app + bench page)');
    await build({
      root: appDir,
      configFile,
      logLevel: 'warn',
      build: {
        outDir: webOutDir,
        emptyOutDir: true,
        rollupOptions: {
          input: {
            main: path.join(appDir, 'index.html'),
            bench: path.join(appDir, 'bench', 'index.html'),
          },
        },
      },
    });
  }
  // Serve the freshly built release wasm and the fixtures from the build dir.
  fs.copyFileSync(wasmPath, path.join(webOutDir, 'schematic_wasm.wasm'));
  fs.mkdirSync(path.join(webOutDir, 'bench-fixtures'), { recursive: true });
  for (const fx of fixtures) {
    fs.copyFileSync(fx.path, path.join(webOutDir, 'bench-fixtures', path.basename(fx.path)));
  }

  const server = await preview({
    root: appDir,
    configFile,
    logLevel: 'warn',
    build: { outDir: webOutDir },
    preview: { port: 4179, strictPort: false, open: false },
  });
  const baseUrl = server.resolvedUrls.local[0].replace(/\/$/, '');
  console.log(`> serving ${webOutDir} at ${baseUrl}`);

  const report = {
    date: new Date().toISOString(),
    env: {
      platform: `${os.type()} ${os.release()} ${os.arch()}`,
      cpu: os.cpus()[0]?.model,
      cpus: os.cpus().length,
      totalMemGiB: +(os.totalmem() / 1024 ** 3).toFixed(1),
      node: process.version,
      wasmBytes: fs.statSync(wasmPath).size,
    },
    runs: [],
  };

  try {
    for (const fixture of fixtures) {
      if (opts.modes.includes('engine')) {
        console.log(`> engine mode ${fixture.scale}`);
        const r = await runEngineMode(baseUrl, fixture, opts);
        report.runs.push({ mode: 'engine', fixture, ...r });
        printEngine(fixture, r);
      }
      if (opts.modes.includes('app')) {
        console.log(`> app mode ${fixture.scale}`);
        const r = await runAppMode(baseUrl, fixture, opts);
        report.runs.push({ mode: 'app', fixture, ...r });
        printApp(fixture, r);
      }
    }
  } finally {
    await new Promise((resolve) => server.httpServer.close(resolve));
  }

  const outPath =
    opts.out ??
    path.join(targetDir, 'bench-results', `browser-${report.date.replace(/[:.]/g, '-')}.json`);
  fs.mkdirSync(path.dirname(outPath), { recursive: true });
  fs.writeFileSync(outPath, JSON.stringify(report, null, 2));
  console.log(`\n> wrote ${outPath}`);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
