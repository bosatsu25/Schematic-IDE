/**
 * Engine-level browser benchmark page (main thread, no React, no worker).
 *
 * Driven by `apps/web/scripts/bench-browser.mjs`, which calls
 * `window.__runBench(options)` and collects the `BENCH_STAGE` console lines so
 * partial results survive a renderer crash / OOM. See docs/benchmarks.md.
 */
import { SchematicEngine, type RegionMeshData } from '../src/engine/schematicEngine';
import { VoxelRenderer } from '@schematic-ide/renderer';

/** The subset of the wasm C-ABI used to split getRegionMesh into phases. */
interface RawExports {
  memory: WebAssembly.Memory;
  wasm_alloc(size: number): number;
  wasm_dealloc(ptr: number, size: number): void;
  wasm_get_response_ptr(): number;
  wasm_get_response_len(): number;
  schematic_get_region_mesh(ptr: number, len: number): number;
  /** Binary face-culled mesh; only present in newer wasm builds. */
  schematic_region_faces?(ptr: number, len: number): number;
}

interface BenchOptions {
  wasmUrl: string;
  fixtureUrl: string;
  regionName: string;
  render: boolean;
}

type BenchResults = Record<string, unknown>;

interface BenchWindow {
  __runBench?: (opts: BenchOptions) => Promise<BenchResults>;
  __benchReady?: boolean;
}

const MIB = 1024 * 1024;

function record(results: BenchResults, key: string, value: unknown): void {
  results[key] = value;
  // One line per stage so the harness keeps partial results if the tab dies.
  console.log('BENCH_STAGE ' + JSON.stringify({ [key]: value }));
}

function jsHeapMb(): number | null {
  const mem = (performance as unknown as { memory?: { usedJSHeapSize: number } }).memory;
  return mem ? mem.usedJSHeapSize / MIB : null;
}

function nextFrame(): Promise<void> {
  return new Promise((resolve) => requestAnimationFrame(() => resolve()));
}

async function runBench(opts: BenchOptions): Promise<BenchResults> {
  const results: BenchResults = {};
  const now = () => performance.now();
  let t0: number;

  const wasmBytes = await (await fetch(opts.wasmUrl)).arrayBuffer();
  record(results, 'wasm_bytes', wasmBytes.byteLength);

  const engine = new SchematicEngine();
  t0 = now();
  await engine.init(wasmBytes);
  record(results, 'wasm_init_ms', now() - t0);
  const ex = (engine as unknown as { exports: RawExports }).exports;
  const wasmMb = () => ex.memory.buffer.byteLength / MIB;

  const fixture = new Uint8Array(await (await fetch(opts.fixtureUrl)).arrayBuffer());
  record(results, 'fixture_bytes', fixture.byteLength);

  // 1. Load -> summary (wasm parse + summary JSON + JSON.parse).
  t0 = now();
  const summary = engine.loadLitematic(fixture);
  record(results, 'load_to_summary_ms', now() - t0);
  record(results, 'summary_non_air_blocks', summary.regions[0]?.non_air_blocks ?? null);
  record(results, 'wasm_memory_after_load_mb', wasmMb());
  record(results, 'js_heap_after_load_mb', jsHeapMb());

  // 2. getRegionMesh, split into the same phases SchematicEngine performs.
  const nameBytes = new TextEncoder().encode(opts.regionName);
  t0 = now();
  const ptr = ex.wasm_alloc(nameBytes.length);
  new Uint8Array(ex.memory.buffer, ptr, nameBytes.length).set(nameBytes);
  const code = ex.schematic_get_region_mesh(ptr, nameBytes.length);
  ex.wasm_dealloc(ptr, nameBytes.length);
  const tWasm = now();
  const rptr = ex.wasm_get_response_ptr();
  const rlen = ex.wasm_get_response_len();
  const json = new TextDecoder().decode(new Uint8Array(ex.memory.buffer, rptr, rlen));
  if (code !== 0) {
    throw new Error(`schematic_get_region_mesh failed: ${json.slice(0, 200)}`);
  }
  const tDecode = now();
  const mesh = JSON.parse(json) as RegionMeshData;
  const tParse = now();
  record(results, 'mesh_wasm_ms', tWasm - t0);
  record(results, 'mesh_json_bytes', rlen);
  record(results, 'mesh_decode_ms', tDecode - tWasm);
  record(results, 'mesh_json_parse_ms', tParse - tDecode);
  record(results, 'get_region_mesh_total_ms', tParse - t0);
  record(results, 'mesh_blocks', mesh.blocks.length / 4);
  record(results, 'wasm_memory_after_mesh_mb', wasmMb());
  record(results, 'js_heap_after_mesh_mb', jsHeapMb());

  // 2b. Binary face-culled mesh (`schematic_region_faces`), when the wasm
  //     build has it. Stream: i32 LE [chunk_count, per chunk: cx, cy, cz, n,
  //     then n x (x, y, z, (palette << 3) | dir)].
  if (typeof ex.schematic_region_faces === 'function') {
    const args = new TextEncoder().encode(JSON.stringify({ region: opts.regionName }));
    t0 = now();
    const aptr = ex.wasm_alloc(args.length);
    new Uint8Array(ex.memory.buffer, aptr, args.length).set(args);
    const fcode = ex.schematic_region_faces(aptr, args.length);
    ex.wasm_dealloc(aptr, args.length);
    const tFacesWasm = now();
    const fptr = ex.wasm_get_response_ptr();
    const flen = ex.wasm_get_response_len();
    if (fcode !== 0) {
      const err = new TextDecoder().decode(new Uint8Array(ex.memory.buffer, fptr, flen));
      throw new Error(`schematic_region_faces failed: ${err.slice(0, 200)}`);
    }
    // Copy out of wasm memory (what a worker would transfer to the main thread).
    const faces = new Int32Array(ex.memory.buffer.slice(fptr, fptr + flen));
    const tCopy = now();
    let faceCount = 0;
    const chunkCount = faces[0];
    let i = 1;
    for (let c = 0; c < chunkCount; c++) {
      const n = faces[i + 3];
      faceCount += n;
      i += 4 + n * 4;
    }
    record(results, 'faces_wasm_ms', tFacesWasm - t0);
    record(results, 'faces_copy_ms', tCopy - tFacesWasm);
    record(results, 'faces_bytes', flen);
    record(results, 'faces_chunks', chunkCount);
    record(results, 'faces_count', faceCount);
    record(results, 'wasm_memory_after_faces_mb', wasmMb());
  }

  // 3. Worker -> main thread hand-off cost in the real app (structured clone).
  t0 = now();
  const cloned = structuredClone(mesh);
  record(results, 'mesh_structured_clone_ms', now() - t0);
  record(results, 'js_heap_with_clone_mb', jsHeapMb());
  void cloned;

  // 4. Render the region with the production VoxelRenderer.
  if (opts.render) {
    const canvas = document.getElementById('bench-canvas') as HTMLCanvasElement;
    t0 = now();
    const renderer = new VoxelRenderer(canvas);
    renderer.resize(canvas.clientWidth, canvas.clientHeight);
    record(results, 'renderer_init_ms', now() - t0);
    const gl = canvas.getContext('webgl2');
    record(
      results,
      'gl_renderer',
      gl
        ? (() => {
            const dbg = gl.getExtension('WEBGL_debug_renderer_info');
            return dbg ? gl.getParameter(dbg.UNMASKED_RENDERER_WEBGL) : 'webgl2';
          })()
        : null,
    );

    const pixel = new Uint8Array(4);
    const cx = Math.floor(canvas.width / 2);
    const cy = Math.floor(canvas.height / 2);

    t0 = now();
    renderer.renderRegion({
      origin: mesh.origin,
      size: mesh.size,
      palette: mesh.palette,
      blocks: mesh.blocks,
      preview_diff: mesh.preview_diff,
    });
    const tSubmit = now();
    // readPixels blocks until the GPU (SwiftShader in headless) has finished.
    gl?.readPixels(cx, cy, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, pixel);
    const tGpu = now();
    await nextFrame();
    const tFrame = now();
    record(results, 'render_build_and_submit_ms', tSubmit - t0);
    record(results, 'first_frame_gpu_done_ms', tGpu - t0);
    record(results, 'first_frame_presented_ms', tFrame - t0);
    record(results, 'center_pixel_rgba', Array.from(pixel));
    record(results, 'js_heap_after_render_mb', jsHeapMb());

    t0 = now();
    renderer.render();
    gl?.readPixels(cx, cy, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, pixel);
    record(results, 'steady_frame_ms', now() - t0);
  }

  record(results, 'done', true);
  return results;
}

const w = window as unknown as BenchWindow;
w.__runBench = runBench;
w.__benchReady = true;
