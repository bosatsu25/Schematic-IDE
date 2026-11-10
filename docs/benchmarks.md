# Large-Structure Scale Benchmarks

This page records how Schematic IDE performs on dense 1M, 5M and 10M-voxel
structures. It covers the native Rust `Session` pipeline and the release WASM
build running in Chromium, both through the engine directly and through the
real app UI.

> **Earlier numbers retracted.** The previous version of this page reported
> results for "1M / 5M / 10M blocks" that came from mostly-air volumes:
> 39,304 / 80,000 / 80,000 occupied blocks (0.8% fill at 10M, 14 KB export).
> They were measured on a debug native build on Windows, and never in WASM or
> a browser. Those numbers, and the claim that "Release/WASM runs exhibit
> comparable or faster execution", have been removed. Everything below was
> re-measured on dense fixtures.

## Current app: face-culled chunk renderer (#27)

The sections after this one were measured **before** #27, with the old
renderer (one instanced cube per block, JSON mesh). The app now renders
face-culled per-chunk meshes from a packed binary stream (one `u32` per face).
Each mesh holds `Uint8` chunk-local positions; colors are released after GPU
upload. Re-measured on 2026-11-09 with the same dense 55%-fill fixtures, the
same container (4 vCPU, no GPU) and Chromium headless on SwiftShader:

| | 1M before | 1M now | 10M before | 10M now |
|---|---:|---:|---:|---:|
| Mesh transfer (worker → main) | 6.0 MiB JSON + clone | 7.5 MiB binary, transferred | 68.8 MiB JSON + clone | 74.8 MiB binary, transferred |
| Engine: first frame presented | 9.9 s | **0.47 s** | 95 s | **4.2 s** |
| Engine: main JS heap after render | 87 MiB | 106 MiB | 782 MiB | 923 MiB |
| Engine: steady-state single frame (SwiftShader) | — | 6.7 s | — | 61 s |
| App: idle after opening the file | 13.8 s | **7.7 s** | 145.5 s | **70 s** |
| App: renderer process peak RSS | 620 MiB | **307 MiB** | 4.3 GB | **1.4 GB** |
| App: analysis on load | 3.9 s | **0 (lazy)** | 49.5 s | **0 (lazy)** |
| Native `region_faces` | — | 150 ms | — | 1.7 s |

Notes:

- The binary stream is slightly larger than the JSON on this fixture, because
  random 55% noise leaves 3.56 visible faces per block. It needs no encode,
  parse or structured clone, and is transferred rather than copied. Builds with
  solid interiors expose far fewer faces.
- In app mode, the worker replies after load (`INSPECT_DOCUMENT`,
  `VALIDATE_DOCUMENT`) appear to take 3–35 s. That time is the main thread
  blocked by SwiftShader frames, not the worker: natively both calls take
  under 125 ms at 10M (table below).
- Rendering is now triangle-bound: 3.9M triangles at 1M and 39M at 10M. On a
  CPU rasterizer that means seconds per frame. Greedy meshing / LOD and a
  hardware-GPU measurement are tracked in #38. Analysis itself is still slow
  when the Analysis tab is opened (#36).
- An intermediate build that stored Float32 positions, normals and colors
  reached 352 MiB (1M) / 3.4 GiB (10M) of main heap. That led to the compact
  `Uint8` layout above.

Reproduce: `pnpm --filter @schematic-ide/web bench:browser -- --scales 1m,10m`
and `cargo test --release -p schematic-wasm --test scale_benchmarks -- --ignored --nocapture --test-threads=1`.

## TL;DR (measured before #27)

- **Parsing and editing scale fine.** Loading 10M voxels with 5.5M occupied
  blocks takes 0.6 s natively and 0.9 s in WASM. Bounded replace, undo and
  redo stay in the low milliseconds to ~100 ms.
- **Rendering is the bottleneck.** The renderer draws one instanced cube per
  occupied block with no hidden-face culling. At 10M that is 5.5M instances,
  about 66M triangles per frame. On the headless software GPU (SwiftShader)
  one frame took about 10 s at 1M, 47 s at 5M and 95 s at 10M, and the main
  thread stayed blocked for that long. A real GPU will be far faster. This
  container has none, so no hardware-GPU number is claimed here.
- **Load-time analysis is the second bottleneck.** The app runs
  `analyze_document` on every load. It takes 3.2 s, 21 s and 41 s natively
  (1M, 5M, 10M) and 3.9 s, 20 s and 50 s in WASM. Native peak RSS reaches
  6.6 GB at 10M. The worker can do nothing else while it runs.
- **The JSON mesh is heavy.** The mesh is 6 / 33 / 69 MiB of JSON. Parsing,
  structured-cloning and per-block JS objects push the main-thread JS heap to
  87 / 344 / 782 MiB after the first render.
- **The first binary face mesh (`region_faces`) culled about 41% of faces but
  was 4-5x larger than the JSON**, at 16 bytes per face (30 / 150 / 299 MiB).
  It now packs one `u32` per face (7.5 MiB at 1M, 74.8 MiB at 10M); see the
  section above.
- **10M does load in the browser**, but slowly. In app mode the tab's
  renderer process peaked at 4.3 GB of RSS, and the app became idle 145 s
  after the file was opened, on software GL.

## How to run

### Native (Rust, release)

```bash
cargo test --release -p schematic-wasm --test scale_benchmarks -- \
    --ignored --nocapture --test-threads=1
```

- The 1M / 5M / 10M tests (`scale_01m_dense`, `scale_05m_dense`,
  `scale_10m_dense`) are `#[ignore]`d. They never run in the default
  `cargo test` or in CI.
- They assert generous regression budgets (about 5-10x the numbers below),
  and only in release builds. A debug build prints numbers and skips the
  asserts.
- `--test-threads=1` matters: per-stage peak RSS uses the process-wide
  `VmHWM` counter, which is reset through `/proc/self/clear_refs` (Linux only).
- The default suite runs `smoke_dense_32_cubed` instead. It drives the same
  pipeline on a 32³ dense volume (load, mesh, JSON, analysis, replace,
  undo/redo, export, re-parse) with no timing asserts.

### Browser / WASM (Chromium via Playwright)

```bash
pnpm --filter @schematic-ide/web bench:browser -- --scales 1m,5m,10m
```

`apps/web/scripts/bench-browser.mjs` does the following:

1. Runs `cargo build -p schematic-wasm --target wasm32-unknown-unknown --release`.
2. Writes the dense fixtures to `target/bench-fixtures/` with
   `cargo run --release -p schematic-wasm --example gen_dense_fixture -- <scale> <out>`.
3. Builds the app plus the benchmark-only page `apps/web/bench/index.html`
   into `target/bench-web/` (production Vite build). It copies the fresh
   release wasm over the committed `public/schematic_wasm.wasm` copy, then
   serves the result with `vite preview`.
4. Runs two modes per scale, each in a fresh headless Chromium:
   - **engine**: on the main thread, `SchematicEngine` loads the file. The
     page calls `schematic_get_region_mesh` through the C-ABI and times wasm
     work, UTF-8 decode and `JSON.parse` separately. It measures
     `structuredClone` of the mesh (the worker-to-main hand-off cost), then
     renders with the production `VoxelRenderer`. GPU completion is forced
     with a 1-pixel `readPixels`. If the wasm exports
     `schematic_region_faces`, the binary face mesh is measured as well.
   - **app**: the real UI. The fixture is opened through the file input. An
     init script times every worker round trip, the first instanced WebGL
     draw, GPU completion of that frame (a `fenceSync` polled without
     blocking) and main-thread long tasks. App code is not modified.
5. Prints a summary and writes JSON to `target/bench-results/`.

Options: `--modes engine,app`, `--no-render`, `--skip-build`,
`--wasm <file>` (benchmark a prebuilt `.wasm` instead of building one), and
`--timeout <sec>` (per scale and mode, default 600). The harness is not part of
CI or the default `test:e2e` run. It needs the `wasm32-unknown-unknown` target
and the Playwright Chromium already used by `test:e2e`.

## Fixtures

The generator is `crates/schematic-wasm/tests/support/dense_fixture.rs`. The
scale tests and the `gen_dense_fixture` example both use it, so native and
browser runs load byte-identical files.

- **Occupancy:** about 55% of voxels are filled, decided by a per-voxel hash.
  Because the pattern is random, interior faces are only partly hidden.
- **Materials:** 34 block states plus air. 21 are plain blocks. The rest
  carry properties: oak logs on 3 axes, oak stairs facing 4 ways, top and
  bottom slabs, a lantern, and chests, furnaces and barrels. Materials are
  picked per 4×2×4 cell, with about 1/8 per-voxel variation, so runs of the
  same block are spatially coherent, as in real builds.
- **Block entities:** about 1 in 512 occupied voxels is a container with a
  matching `TileEntities` compound.

| Scale | Dimensions | Voxels | Occupied | Fill | Block entities | `.litematic` size |
|---|---|---:|---:|---:|---:|---:|
| 1M | 100 × 100 × 100 | 1,000,000 | 550,407 | 55.0% | 1,131 | 526 KiB |
| 5M | 200 × 125 × 200 | 5,000,000 | 2,749,749 | 55.0% | 5,392 | 2,618 KiB |
| 10M | 250 × 160 × 250 | 10,000,000 | 5,499,787 | 55.0% | 10,762 | 5,484 KiB |

## Environment

| | |
|---|---|
| Machine | Linux container (Ubuntu 24.04.5, kernel 6.18), 4 vCPU Intel Xeon @ 2.30 GHz, 15 GiB RAM, no GPU |
| Rust | rustc / cargo 1.97.0, default `release` profile (opt-level 3, no LTO, 16 codegen units) |
| WASM | `wasm32-unknown-unknown` release build, 0.9 MiB, not run through `wasm-opt` |
| Browser | Playwright Chromium 153.0.8010.12, headless, default V8 heap limits. WebGL2 on ANGLE/Vulkan over **SwiftShader**, a CPU rasterizer |
| Node | 22.23.4, Vite 5 production build served by `vite preview` |
| Date | 2026-11-09 |

Every number is a single run. Repeat runs on this machine varied by roughly
±10%. For example, native 10M analysis measured 45.2 s in one run and 41.2 s
in the next.

## Results: native release (`scale_benchmarks.rs`)

| Stage | 1M | 5M | 10M |
|---|---:|---:|---:|
| Load (`Session::load_litematic`) | 56 ms | 262 ms | 591 ms |
| Mesh extraction (`get_region_mesh`) | 140 ms | 1,027 ms | 2,161 ms |
| Mesh JSON serialization (as the C-ABI does) | 15 ms | 100 ms | 195 ms |
| Mesh JSON size | 6.0 MiB | 33.3 MiB | 68.8 MiB |
| `inspect_document` | 7 ms | 36 ms | 75 ms |
| `validate_document` | 12 ms | 64 ms | 122 ms |
| `analyze` (whole document) | **3,151 ms** | **20,870 ms** | **41,244 ms** |
| …of which surface + feature pass | 2,694 ms | 17,764 ms | 37,031 ms |
| Bounded replace, preview + commit (one octant) | 11 ms (2,153 changed) | 53 ms (11,244) | 109 ms (22,520) |
| Undo / redo | 0.1 / 0.1 ms | 1.0 / 1.4 ms | 1.8 / 2.0 ms |
| Re-export `.litematic` | 122 ms | 681 ms | 1,417 ms |
| Peak RSS during load | 37 MiB | 323 MiB | 400 MiB |
| Peak RSS during mesh + JSON | 87 MiB | 465 MiB | 679 MiB |
| Peak RSS during analyze | 635 MiB | **3,298 MiB** | **6,595 MiB** |

## Results: WASM in Chromium, engine mode

All of this runs on the main thread with no worker. Times are from the start
of each step.

| Stage | 1M | 5M | 10M |
|---|---:|---:|---:|
| Load to summary (`loadLitematic`) | 114 ms | 462 ms | 873 ms |
| `getRegionMesh` total | 249 ms | 1,193 ms | 2,277 ms |
| …wasm (extract + JSON serialize) | 199 ms | 1,013 ms | 1,948 ms |
| …UTF-8 decode | 6 ms | 22 ms | 47 ms |
| …`JSON.parse` | 45 ms | 157 ms | 282 ms |
| `structuredClone(mesh)` (worker hand-off cost) | 61 ms | 243 ms | 502 ms |
| `renderRegion` build + submit (JS) | 211 ms | 685 ms | 1,310 ms |
| First frame finished on GPU (SwiftShader) | 9.9 s | 47.2 s | 94.9 s |
| Steady-state frame (SwiftShader) | 9.6 s | 52.5 s | 103.9 s |
| wasm linear memory after load / after mesh | 35 / 85 MiB | 134 / 333 MiB | 253 / 651 MiB |
| Main JS heap after load → mesh → render | 4 → 19 → 87 MiB | 6 → 81 → 344 MiB | 9 → 161 → 782 MiB |
| Peak RSS, renderer / GPU process | 309 / 172 MiB | 854 / 302 MiB | 1,652 / 470 MiB |

## Results: WASM in Chromium, real app (worker + React + Three.js)

`t` is measured from the file input's `change` event. Round trips are timed
from `postMessage` on the main thread until the main thread handles the reply.

| Stage | 1M | 5M | 10M |
|---|---:|---:|---:|
| `LOAD_LITEMATIC` round trip | 106 ms | 469 ms | 985 ms |
| `GET_REGION_MESH` round trip | 251 ms | 1,355 ms | 3,034 ms |
| First instanced draw call issued | t+0.55 s | t+2.7 s | t+5.5 s |
| First frame finished on GPU (SwiftShader) | t+9.9 s | t+48.9 s | t+95.7 s |
| Longest main-thread long task | 9.3 s | 46.2 s | 90.2 s |
| `ANALYZE_DOCUMENT` round trip | 3.9 s | 20.4 s | 49.5 s |
| App idle (last load-time call returned) | t+13.8 s | t+69.5 s | t+145.5 s |
| Peak RSS, renderer process (main + worker + wasm) | 620 MiB | 2,249 MiB | 4,335 MiB |

The `INSPECT_DOCUMENT` round trip looked like 9-90 s. The worker's actual
work there takes milliseconds (see the native table). The reply simply waits
while the main thread is blocked by the first frame.

## Results: binary face mesh (`region_faces`)

*Historical: this measured the first, unpacked encoding (16 bytes per face).
The shipped stream packs one `u32` per face, as the last bullet below
proposed: `[chunk_count, per chunk: cx, cy, cz, n, then n packed u32]`, with
bits 0–11 the in-chunk x/y/z, 12–14 the direction and 15–31 the palette
index.*

These numbers were measured on the same fixtures with a snapshot of `main`'s
in-progress sources, taken 2026-11-09 before that API was committed. Native
numbers come from the ported `scale_benchmarks.rs`. The WASM numbers come from
the engine page, which calls `schematic_region_faces` whenever the build
exports it. Every other native stage in that run was within ±10% of the
tables above.

| | 1M | 5M | 10M |
|---|---:|---:|---:|
| Faces emitted | 1,967,511 | 9,802,477 | 19,594,522 |
| Faces per occupied block (6 = no culling) | 3.57 | 3.56 | 3.56 |
| Chunks | 343 | 1,352 | 2,560 |
| Stream size | **30.0 MiB** | **149.6 MiB** | **299.0 MiB** |
| …compared with the JSON mesh | 5.0x | 4.5x | 4.3x |
| Native `region_faces` | 192 ms | 980 ms | 2,117 ms |
| Native `region_view` | <0.1 ms | <0.1 ms | 0.1 ms |
| Native peak RSS during `region_faces` | 134 MiB | 706 MiB | 1,005 MiB |
| WASM `schematic_region_faces` | 221 ms | 1,260 ms | 2,648 ms |
| Copy out of wasm memory (`ArrayBuffer.slice`) | 17 ms | 89 ms | 191 ms |
| wasm linear memory afterwards (was 86 / 339 / 662 MiB after the JSON mesh) | 159 MiB | 742 MiB | 1,442 MiB |

What this shows:

- Face culling removes about 41% of faces on this fixture. Instead of 6
  faces per block there are 3.56, so 10M drops from about 66M triangles to
  about 39M. Builds with solid interiors would cull far more. The random 55%
  fill, plus non-full blocks such as glass, stairs, slabs and lanterns, is a
  pessimistic case.
- No JSON encode or decode is needed. Wasm-side cost is similar to
  `get_region_mesh` plus JSON serialization, and copying out of wasm memory
  is cheap (17-191 ms).
- **The stream is 4-5x larger than the JSON mesh** (16 bytes per face,
  3.56 faces per block, so about 57 bytes per block versus about 13 bytes of
  JSON). At 10M it is 299 MiB per full rebuild, and the wasm heap grows to
  1.4 GiB (wasm memory never shrinks). The fields fit in far fewer bits:
  chunk-local `x`, `y` and `z` need 4 bits each, `dir` 3 bits and the palette
  index ≤16 bits. That is one `u32` per face, a 4x reduction. A per-block
  6-bit visible-face mask would shrink it further. Streaming the result per
  chunk, rather than as one region-sized buffer, would also cap the peak.

## Findings and bottlenecks

1. **Per-block cube instancing does not scale.** `VoxelRenderer.renderRegion`
   creates one `InstancedMesh` per palette entry and one 12-triangle cube per
   occupied block. Interior faces are not culled. At 10M it submits 5.5M
   instances in 34 draw calls. On SwiftShader that costs 10-100 s per frame,
   and the main thread is blocked for the whole frame (the longest long task
   roughly equals the frame time). Hardware GPUs will be much faster but were
   not measured here. Even so, 66M triangles per frame for a 10M structure
   is far more than an interactive viewport should need. The chunked,
   face-culled `region_faces` path (see the section above) removes about 41%
   of faces even on this pessimistic fixture. Uploading only dirty chunks
   would also stop full re-uploads after each edit. Its encoding still needs
   to shrink.
2. **Load-time analysis is the slowest CPU stage.** `Session::analyze` runs
   on every load and after every region switch. Most of its time goes to
   `SurfaceAnalyzer`, which collects every occupied block into a
   `BTreeMap<Position, …>`. Material counting also builds a `String` key per
   block. It needs 3 / 21 / 41 s and up to 6.6 GB natively. In the browser
   it blocks the worker for 4-50 s, so edits issued in that window queue
   behind it. Candidates: make it lazy (only when the Analysis tab is open),
   count materials by palette index, and replace the per-block maps with
   dense bitsets per chunk.
3. **The JSON mesh transport is heavy.** The flat `[x, y, z, palette]`
   array is serialized to 6.0 / 33.3 / 68.8 MiB of JSON inside wasm. JS then
   decodes it, `JSON.parse`s it into a JS number array, structured-clones it
   from worker to main thread (61-502 ms), and regroups it into one
   `{x, y, z}` object per block before building the instance matrices. Main
   JS heap after the first render reaches 87 / 344 / 782 MiB. A binary
   (`Int32Array` / `Uint8Array`) buffer that is transferred rather than
   cloned would remove the encode, decode and clone costs, and most of the
   heap growth.
4. **`get_region_mesh` itself is slow at scale.** It took 140 ms, 1.0 s and
   2.2 s natively. It inserts every occupied position into a
   `HashSet<BlockPosition>`, only to find preview-added blocks, and that
   insertion dominates its cost.
5. **Memory is close to browser limits at 10M.** The app's renderer process
   peaked at 4.3 GB. That total combines the worker's wasm linear memory
   (wasm32 is capped at 4 GiB), the main JS heap and the Three.js buffers. It
   completed here, on a 15 GiB machine, but leaves little headroom on typical
   laptops.
6. **Parsing, editing and export are healthy.** WASM load is about 1.5-2x
   native. Mesh extraction in wasm is roughly on par with native (0.8-1.3x).
   Analysis takes 1.0-1.25x the native time. Bounded edits and undo/redo stay
   cheap, because they only touch the chunks in the patch set. A 10M
   re-export takes about 1.4 s natively.

## Caveats

- Rendering numbers come from SwiftShader, a CPU rasterizer. They show the
  geometry load and the main-thread blocking behaviour, not what a user with
  a discrete or integrated GPU will see.
- Each number is a single run on a shared 4-vCPU container; expect about
  ±10% noise.
- The fixture's random 55% occupancy is a stress case for face culling and
  compression. Real builds usually have larger solid interiors, which
  compress and cull better.
